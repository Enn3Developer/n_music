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
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
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

/** A scan in progress: the tracks it listed and how many of those it still reads. */
data class Scan(val found: ULong, val pending: ULong)

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

    /** The whole library, in the order the scan lists it; `null` until first read. */
    private val _tracks = MutableStateFlow<List<TrackRow>?>(null)
    val tracks: StateFlow<List<TrackRow>?> = _tracks.asStateFlow()

    private val _scan = MutableStateFlow<Scan?>(null)
    val scan: StateFlow<Scan?> = _scan.asStateFlow()

    /** The library folders; `null` until the core reports them. */
    private val _roots = MutableStateFlow<List<Locator>?>(null)
    val roots: StateFlow<List<Locator>?> = _roots.asStateFlow()

    /** One-off messages for a snackbar. */
    private val _notices = MutableSharedFlow<String>(extraBufferCapacity = 8)
    val notices: SharedFlow<String> = _notices.asSharedFlow()

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
                is CoreEvent.LibraryChanged -> libraryChanged.trySend(Unit)
                is CoreEvent.ScanProgress -> _scan.value =
                    if (event.libraries.isEmpty()) null else Scan(event.found, event.pending)
                is CoreEvent.LibraryRootsChanged -> _roots.value = event.roots
                is CoreEvent.PlaylistRejected -> _notices.emit(event.reason)
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
}

/** Tells tracks apart in lists: what a locator points at. */
val Locator.key: String
    get() = when (this) {
        is Locator.Local -> v1
        is Locator.DocumentTree -> v1
        is Locator.Document -> uri
        is Locator.Web -> v1
    }
