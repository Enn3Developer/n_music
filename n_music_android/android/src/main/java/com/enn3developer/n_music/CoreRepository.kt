package com.enn3developer.n_music

import android.content.Context
import android.os.SystemClock
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Core
import com.enn3developer.n_music.core.CoreEvent
import com.enn3developer.n_music.core.ItemId
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.core.QueueRow
import com.enn3developer.n_music.core.Seek
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.core.libraryQuery
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.util.concurrent.atomic.AtomicLong

/** The current item of the play session and its track. */
data class Current(val item: ItemId, val track: TrackRow)

/** Where playback is, as the core last reported it. */
data class Position(
    val position: Double = 0.0,
    val length: Double = 0.0,
    /** The last `Seek.Tracked` request that applied, 0 before any. */
    val seek: ULong = 0u,
    /** The position jumped (seek, new track, pause) rather than advanced with playback. */
    val discontinuity: Boolean = true,
    /** When it was reported, in [SystemClock.elapsedRealtime] milliseconds. */
    val at: Long = SystemClock.elapsedRealtime(),
)

/**
 * A scan in progress: the tracks it listed, how many of those it still had to read then, and how
 * many it read since.
 */
private data class Scan(val found: ULong, val pending: ULong, val read: ULong = 0u) {
    val progress: Float
        get() = if (found == 0uL) {
            0f
        } else {
            ((found - pending + read).toFloat() / found.toFloat()).coerceIn(0f, 1f)
        }
}

/**
 * The core for the whole process. One loop reads [Core.nextEvent] into flows, a flow for each
 * piece of state so a screen only recomposes for what it reads, and [send] carries commands.
 * [NMusicApplication] starts it, so it outlives activities and the playback service alike.
 */
object CoreRepository {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private lateinit var core: Core

    private val _playing = MutableStateFlow(false)
    val playing: StateFlow<Boolean> = _playing.asStateFlow()

    private val _current = MutableStateFlow<Current?>(null)
    val current: StateFlow<Current?> = _current.asStateFlow()

    private val _position = MutableStateFlow(Position())
    val position: StateFlow<Position> = _position.asStateFlow()

    private val _queue = MutableStateFlow<List<QueueRow>>(emptyList())
    val queue: StateFlow<List<QueueRow>> = _queue.asStateFlow()

    private val _loopStatus = MutableStateFlow(LoopStatus.PLAYLIST)
    val loopStatus: StateFlow<LoopStatus> = _loopStatus.asStateFlow()

    private val _shuffle = MutableStateFlow(false)
    val shuffle: StateFlow<Boolean> = _shuffle.asStateFlow()

    /** The whole library, in the order the scan lists it. */
    private val _tracks = MutableStateFlow<List<TrackRow>>(emptyList())

    /**
     * The library in play order, as the Slint app listed it: the session's tracks first, each
     * once, then the tracks it does not have, in library order.
     */
    val rows: StateFlow<List<TrackRow>> = combine(_tracks, _queue, ::playOrder)
        .stateIn(scope, SharingStarted.Eagerly, emptyList())

    private val scan = MutableStateFlow<Scan?>(null)

    /** How far the running scan is, from 0 to 1; 0 while none runs. */
    val scanProgress: StateFlow<Float> = scan
        .map { it?.progress ?: 0f }
        .stateIn(scope, SharingStarted.Eagerly, 0f)

    private val libraryChanged = Channel<Unit>(Channel.CONFLATED)
    private val seekRequests = AtomicLong()

    /** The whole library, in the order the scan lists it. */
    val library: Query by lazy { libraryQuery() }

    /** Starts the core once per process; [NativeLibrary.init] must have run. */
    fun start(context: Context) {
        if (::core.isInitialized) return
        // Where the Slint app kept its data, so settings, the library and the session carry over.
        val dataDir = File(context.getExternalFilesDir(null) ?: context.filesDir, "config")
        core = Core.start(dataDir.path, context.cacheDir.path)
        scope.launch { readEvents() }
        scope.launch { refreshLibrary() }
    }

    /** Never blocks, so it is safe from the main thread. */
    fun send(command: Command) = core.send(command)

    /**
     * Seeks the current track to [seconds]. Positions report the returned request in
     * [Position.seek] once the seek applied, so a slider can ignore the ones sent before.
     */
    fun seek(seconds: Double): ULong {
        val request = seekRequests.incrementAndGet().toULong()
        send(Command.Seek(Seek.Tracked(seconds, request)))
        return request
    }

    /** The settings section [key] as JSON, from the core's settings file. */
    fun setting(key: String): String? = core.setting(key)

    fun setSetting(key: String, json: String) = core.setSetting(key, json)

    private suspend fun readEvents() {
        while (true) {
            when (val event = core.nextEvent() ?: break) {
                is CoreEvent.PlaybackChanged -> _playing.value = event.playing
                is CoreEvent.TrackChanged -> _current.value = Current(event.item, event.track)
                is CoreEvent.PositionChanged -> _position.value = Position(
                    position = event.position,
                    length = event.length,
                    seek = event.seek,
                    discontinuity = event.discontinuity,
                )
                is CoreEvent.QueueChanged -> _queue.value = event.entries
                is CoreEvent.LoopStatusChanged -> _loopStatus.value = event.status
                is CoreEvent.ShuffleChanged -> _shuffle.value = event.enabled
                is CoreEvent.LibraryChanged -> {
                    scan.value = scan.value?.copy(read = event.read)
                    libraryChanged.trySend(Unit)
                }
                is CoreEvent.ScanProgress -> scan.value =
                    if (event.libraries.isEmpty()) null else Scan(event.found, event.pending)
                else -> {}
            }
        }
    }

    /**
     * Reads the library again after it changed, at most once a second: a scan reports every
     * track it reads.
     */
    private suspend fun refreshLibrary() {
        for (change in libraryChanged) {
            _tracks.value = withContext(Dispatchers.IO) {
                core.tracks(library, 0u, UInt.MAX_VALUE)
            }
            delay(1000)
        }
    }

    private fun playOrder(tracks: List<TrackRow>, queue: List<QueueRow>): List<TrackRow> {
        if (queue.isEmpty()) return tracks
        val byKey = tracks.associateBy { it.locator.key }
        val placed = HashSet<String>(queue.size)
        val rows = ArrayList<TrackRow>(tracks.size)
        for (entry in queue) {
            val key = entry.track.locator.key
            val track = byKey[key] ?: continue
            if (placed.add(key)) rows += track
        }
        tracks.filterTo(rows) { it.locator.key !in placed }
        return rows
    }
}

/** Tells tracks apart in lists: what a locator points at. */
val Locator.key: String
    get() = when (this) {
        is Locator.Local -> v1
        is Locator.DocumentTree -> v1
        is Locator.Document -> uri
        is Locator.Web -> v1
    }
