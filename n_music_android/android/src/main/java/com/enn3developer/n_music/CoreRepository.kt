package com.enn3developer.n_music

import android.content.Context
import android.os.SystemClock
import com.enn3developer.n_music.core.AlbumRow
import com.enn3developer.n_music.core.ArtistRow
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Core
import com.enn3developer.n_music.core.CoreEvent
import com.enn3developer.n_music.core.Facets
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.GenreRow
import com.enn3developer.n_music.core.GroupSort
import com.enn3developer.n_music.core.ItemId
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.OutputDevice
import com.enn3developer.n_music.core.PlaybackOptions
import com.enn3developer.n_music.core.PlaylistId
import com.enn3developer.n_music.core.PlaylistRow
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.core.QueueRow
import com.enn3developer.n_music.core.ReplayGainMode
import com.enn3developer.n_music.core.Seek
import com.enn3developer.n_music.core.SourceRow
import com.enn3developer.n_music.core.Summary
import com.enn3developer.n_music.core.TrackDetails
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
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
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
 * A scan in progress over [libraries]: the tracks it listed and how many of those it read.
 */
data class ScanState(val libraries: List<Locator>, val found: Long, val read: Long) {
    val progress: Float
        get() = if (found <= 0) 0f else (read.toFloat() / found).coerceIn(0f, 1f)
}

/** The copies of streamed tracks: on or off, how much they may take and take now, in bytes. */
data class StreamCache(val enabled: Boolean, val limit: Long, val used: Long)

/**
 * A scan as reported: the tracks it listed and how many of those it still had to read, as of the
 * report, when scans had read [base] tracks since launch; [read] is that count now.
 */
private data class Scan(
    val libraries: List<Locator>,
    val found: ULong,
    val pending: ULong,
    val base: ULong,
    val read: ULong = base,
) {
    fun state(): ScanState {
        val since = if (read > base) read - base else 0uL
        val done = (found - minOf(pending, found) + since).coerceAtMost(found)
        return ScanState(libraries, found.toLong(), done.toLong())
    }
}

/**
 * The core for the whole process. One loop reads [Core.nextEvent] into flows, a flow for each
 * piece of state so a screen only recomposes for what it reads, and [send] carries commands.
 * Library queries run on the caller's thread: call them off the main thread.
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

    private val _volume = MutableStateFlow(1.0)
    val volume: StateFlow<Double> = _volume.asStateFlow()

    /**
     * Goes up whenever the library's tracks, their names, plays or the playlists change: the
     * screens query again. A scan reports every track it reads; those count once a second.
     */
    private val _version = MutableStateFlow(0L)
    val version: StateFlow<Long> = _version.asStateFlow()

    /** The library folders and playlists; `null` until the core reported them. */
    private val _roots = MutableStateFlow<List<Locator>?>(null)
    val roots: StateFlow<List<Locator>?> = _roots.asStateFlow()

    private val scan = MutableStateFlow<Scan?>(null)
    private val _scanState = MutableStateFlow<ScanState?>(null)

    /** The running scan; `null` while none runs. */
    val scanState: StateFlow<ScanState?> = _scanState.asStateFlow()

    /** The scan of the library just set up in Welcome: none, waiting for it, or running. */
    private enum class Build { NONE, WAITING, RUNNING }

    private val build = MutableStateFlow(Build.NONE)
    private val _building = MutableStateFlow(false)

    /** The library is being built for the first time: the library shows the scan. */
    val building: StateFlow<Boolean> = _building.asStateFlow()

    private val _streamCache = MutableStateFlow<StreamCache?>(null)
    val streamCache: StateFlow<StreamCache?> = _streamCache.asStateFlow()

    private val _outputDevices = MutableStateFlow<List<OutputDevice>>(emptyList())
    val outputDevices: StateFlow<List<OutputDevice>> = _outputDevices.asStateFlow()

    private val _playlists = MutableStateFlow<List<PlaylistRow>>(emptyList())

    /** Every playlist, by name. */
    val playlists: StateFlow<List<PlaylistRow>> = _playlists.asStateFlow()

    private val _sources = MutableStateFlow<List<SourceRow>>(emptyList())

    /** The library folders and web playlists with what they hold, in the order they were added. */
    val sources: StateFlow<List<SourceRow>> = _sources.asStateFlow()

    private val _library = MutableStateFlow(Summary(0u, 0.0))

    /** How many tracks the whole library has and how long they play. */
    val library: StateFlow<Summary> = _library.asStateFlow()

    private val _ready = MutableStateFlow(false)

    /** The library's lists and summary were read once since the core started. */
    val ready: StateFlow<Boolean> = _ready.asStateFlow()

    private val _facets = MutableStateFlow(Facets(emptyList(), emptyList(), null, null))

    /** The genres, formats and years the filters offer. */
    val facets: StateFlow<Facets> = _facets.asStateFlow()

    private val _options = MutableStateFlow(PlaybackOptions(ReplayGainMode.OFF, false, 0.0))

    /** ReplayGain, resuming and crossfade. */
    val options: StateFlow<PlaybackOptions> = _options.asStateFlow()

    private val _rejections = MutableSharedFlow<String>(extraBufferCapacity = 4)

    /** Why the core turned down a playlist change. */
    val rejections: SharedFlow<String> = _rejections.asSharedFlow()

    private val libraryChanged = Channel<Unit>(Channel.CONFLATED)
    private val seekRequests = AtomicLong()

    /** Where the core keeps its settings, logs and database. */
    lateinit var dataDir: File
        private set

    /** The whole library, in the order the scan lists it. */
    val everything: Query by lazy { libraryQuery() }

    /** Starts the core once per process; [NativeLibrary.init] must have run. */
    fun start(context: Context) {
        if (::core.isInitialized) return
        // Where the Slint app kept its data, so settings, the library and the session carry over.
        dataDir = File(context.getExternalFilesDir(null) ?: context.filesDir, "config")
        core = Core.start(dataDir.path, context.cacheDir.path)
        _options.value = core.playbackOptions()
        scope.launch { readEvents() }
        scope.launch { countChanges() }
        scope.launch {
            combine(_version, _roots) { _, roots -> roots }.collectLatest { roots ->
                withContext(Dispatchers.IO) { refresh(roots) }
            }
        }
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

    /** Sets the library up for the first time: the library shows its scan until it ends. */
    fun build(roots: List<Locator>) {
        build.value = Build.WAITING
        _building.value = true
        _roots.value = roots
        send(Command.SetLibraryRoots(roots))
    }

    /**
     * Changes the library's folders and playlists with [change], then runs [after]: new ones are
     * read, and the tracks of those left out leave. The roots change here at once, so another
     * change right after builds on this one; before the core told them, it waits for them.
     */
    fun editRoots(change: (List<Locator>) -> List<Locator>, after: () -> Unit = {}) {
        fun apply(roots: List<Locator>) {
            val changed = change(roots)
            _roots.value = changed
            send(Command.SetLibraryRoots(changed))
            after()
        }
        val known = _roots.value
        if (known != null) {
            apply(known)
        } else {
            scope.launch(Dispatchers.Main) {
                _roots.filterNotNull().first()
                // Read again: an edit that waited too may have gone first.
                apply(_roots.value.orEmpty())
            }
        }
    }

    fun setReplayGain(mode: ReplayGainMode) {
        _options.value = _options.value.copy(replayGain = mode)
        send(Command.SetReplayGain(mode))
    }

    fun setResume(enabled: Boolean) {
        _options.value = _options.value.copy(resume = enabled)
        send(Command.SetResume(enabled))
    }

    fun setCrossfade(seconds: Double) {
        _options.value = _options.value.copy(crossfade = seconds)
        send(Command.SetCrossfade(seconds))
    }

    /** The settings section [key] as JSON, from the core's settings file. */
    fun setting(key: String): String? = core.setting(key)

    fun setSetting(key: String, json: String) = core.setSetting(key, json)

    // The library's queries. Each reads the library as it is now; call them off the main thread.

    fun tracks(query: Query): List<TrackRow> = core.tracks(query, 0u, UInt.MAX_VALUE)

    fun track(locator: Locator): TrackRow? = core.track(locator)

    fun details(locator: Locator): TrackDetails? = core.trackDetails(locator)

    /** The picture in the file of [locator] at its own size, still encoded. It reads the file. */
    fun coverArt(locator: Locator): ByteArray? = core.coverArt(locator)

    fun summary(filter: Filter): Summary = core.summary(filter)

    fun albums(filter: Filter, search: String, sort: GroupSort): List<AlbumRow> =
        core.albums(filter, search, sort)

    fun artists(filter: Filter, search: String, sort: GroupSort): List<ArtistRow> =
        core.artists(filter, search, sort)

    fun genres(filter: Filter, search: String, sort: GroupSort): List<GenreRow> =
        core.genres(filter, search, sort)

    fun playlist(id: PlaylistId): PlaylistRow? = core.playlist(id)

    fun playlistHolds(id: PlaylistId, tracks: List<Locator>): Int =
        core.playlistHolds(id, tracks).toInt()

    fun missingTracks(root: Locator): List<TrackRow> = core.missingTracks(root)

    /** Adds to playlist [id] those of [tracks] it lacks, off the main thread; returns them. */
    suspend fun addToPlaylist(id: PlaylistId, tracks: List<Locator>): List<Locator> = withContext(Dispatchers.IO) {
        val held = core.tracks(Query(Filter.Playlist(id), emptyList()), 0u, UInt.MAX_VALUE).mapTo(HashSet()) { it.locator }
        val missing = tracks.filter { it !in held }
        if (missing.isNotEmpty()) core.send(Command.AddToPlaylist(id, missing))
        missing
    }

    private suspend fun readEvents() {
        while (true) {
            // The core never stops on its own, and it ends the process when its bus fails.
            // Going on without it would leave every screen and the notification stuck: end the
            // process here too.
            when (val event = core.nextEvent() ?: error("The core stopped")) {
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
                is CoreEvent.VolumeChanged -> _volume.value = event.volume
                is CoreEvent.LibraryChanged -> {
                    scan.value = scan.value?.copy(read = event.read)
                    _scanState.value = scan.value?.state()
                    libraryChanged.trySend(Unit)
                }
                is CoreEvent.ScanProgress -> {
                    scan.value = if (event.libraries.isEmpty()) {
                        null
                    } else {
                        Scan(event.libraries, event.found, event.pending, event.read)
                    }
                    _scanState.value = scan.value?.state()
                    build.value = when {
                        build.value == Build.NONE -> Build.NONE
                        event.libraries.isNotEmpty() -> Build.RUNNING
                        build.value == Build.RUNNING -> Build.NONE
                        else -> build.value
                    }
                    _building.value = build.value != Build.NONE
                }
                is CoreEvent.TrackPlayed -> libraryChanged.trySend(Unit)
                is CoreEvent.PlaylistsChanged -> _version.value++
                is CoreEvent.PlaylistRejected -> _rejections.tryEmit(event.reason)
                is CoreEvent.LibraryRootsChanged -> _roots.value = event.roots
                is CoreEvent.StreamCacheChanged -> _streamCache.value = StreamCache(
                    event.enabled,
                    event.limit.toLong(),
                    event.used.toLong(),
                )
                is CoreEvent.OutputDevices -> _outputDevices.value = event.devices
                is CoreEvent.ScanFinished -> _version.value++
            }
        }
    }

    /** Counts library changes at most once a second: a scan reports every track it reads. */
    private suspend fun countChanges() {
        for (change in libraryChanged) {
            _version.value++
            delay(1000)
        }
    }

    private fun refresh(roots: List<Locator>?) {
        _playlists.value = core.playlists()
        _sources.value = core.sources(roots.orEmpty())
        _library.value = core.summary(Filter.All(emptyList()))
        _facets.value = core.facets()
        _ready.value = true
    }
}

/** Tells tracks apart in lists: what a locator points at. */
val Locator.key: String
    get() = when (this) {
        is Locator.Local -> v1
        is Locator.DocumentTree -> v1
        is Locator.Document -> uri
        is Locator.Web -> v1
        is Locator.TelegramChat -> v1
        is Locator.TelegramAudio -> uri
    }

/** [Locator] as one line of text, which [decodeLocator] reads back: for saved state. */
fun Locator.encode(): String = when (this) {
    is Locator.Local -> "local\n$v1"
    is Locator.DocumentTree -> "tree\n$v1"
    is Locator.Document -> "document\n$uri\n$name"
    is Locator.Web -> "web\n$v1"
    is Locator.TelegramChat -> "telegram\n$v1"
    is Locator.TelegramAudio -> "telegramAudio\n$uri\n$name"
}

fun decodeLocator(text: String): Locator? {
    val parts = text.split('\n')
    return when (parts.firstOrNull()) {
        "local" -> parts.getOrNull(1)?.let(Locator::Local)
        "tree" -> parts.getOrNull(1)?.let(Locator::DocumentTree)
        "document" -> if (parts.size >= 3) Locator.Document(parts[1], parts[2]) else null
        "web" -> parts.getOrNull(1)?.let(Locator::Web)
        "telegram" -> parts.getOrNull(1)?.let(Locator::TelegramChat)
        "telegramAudio" -> if (parts.size >= 3) Locator.TelegramAudio(parts[1], parts[2]) else null
        else -> null
    }
}
