import Foundation
import Observation

/// A scan in progress over `libraries`: the tracks it listed and how many of those it read.
struct ScanState: Equatable, Sendable {
    var libraries: [Locator]
    /// 0 until the scan listed its sources.
    var found: UInt64
    var read: UInt64

    var progress: Double { found == 0 ? 0 : min(1, Double(read) / Double(found)) }
}

/// Where signing in to Telegram is. `ended` counts the steps that ended since launch, so a
/// screen tells the answer to its own request from an older one.
struct TelegramState: Equatable, Sendable {
    var status: TelegramStatus
    var busy: Bool
    var error: TelegramError?
    var ended: Int
}

/// What a search for Telegram chats found: the `serial`th answer.
struct TelegramChats: Equatable, Sendable {
    var query: String
    var chats: [TelegramChatInfo]
    var error: TelegramError?
    var serial: Int
}

/// The current item of the play session and its track.
struct CurrentTrack: Equatable, Sendable {
    var item: ItemId
    var track: TrackRow
}

/// The core and what it last announced, for the whole app: `Store.shared`.
///
/// - Views hold it as `private let store = Store.shared` (never through the environment:
///   SwiftUI doesn't hand the environment to separately hosted views like table cells).
/// - One task reads `Core.nextEvent` off the main thread and applies each event here, on the
///   main actor. `observers` hear every event after it applied.
/// - Commands go out through the typed methods below; `send` never blocks.
/// - Library queries run off the main thread: `await store.tracks(query)`, or `query { core in … }`
///   for anything else. Re-run them when `libraryVersion` (tracks, names, plays) or
///   `playlistsVersion` changes.
/// - The playback position is interpolated: read `position()` (or use `PlayClock`), not a
///   stored value; reports re-sync it only on a jump or a drift over 0.35 s.
@MainActor
@Observable
final class Store {
    static let shared = Store()

    @ObservationIgnored let core: Core
    /// `~/Library/Application Support/n_music`: settings, the library database, the logs.
    @ObservationIgnored let dataDirectory: URL
    /// `~/Library/Caches/n_music`: covers, copies of streamed tracks.
    @ObservationIgnored let cacheDirectory: URL

    // MARK: Playback

    private(set) var playing = false
    private(set) var current: CurrentTrack?
    /// The current track's format, genres and plays; `nil` until read.
    private(set) var details: TrackDetails?
    /// Goes up at every change of the current item; `trackChangeBack` tells whether the new
    /// one plays before the old in the session (the title then comes down from above).
    private(set) var trackChange = 0
    private(set) var trackChangeBack = false
    /// The last reported (or seeked-to) position, at `positionAt`.
    private(set) var basePosition: Double = 0
    private(set) var positionAt = ContinuousClock.now
    /// The decoder's length; 0 while it doesn't know yet (`length` then uses the tags').
    private(set) var reportedLength: Double = 0
    /// From 0 to 1.
    private(set) var volume: Double = 1
    private(set) var loopStatus: LoopStatus = .off
    private(set) var shuffle = false
    /// The play session, in play order.
    private(set) var queue: [QueueRow] = []
    private(set) var options: PlaybackOptions
    private(set) var outputDevices: [OutputDevice] = []
    /// The device chosen to play on; `nil` follows the system default.
    private(set) var outputDevice: OutputDevice?
    /// Some window of the app is on screen (see `Power`): clocks animate only then.
    var windowVisible = true

    // MARK: The library

    /// Goes up when the library's tracks, names or plays change: track and group lists query
    /// again. During a scan it goes up at most once a second.
    private(set) var libraryVersion = 0
    /// Goes up when the playlists change (added, renamed, deleted, their tracks or rules).
    private(set) var playlistsVersion = 0
    /// The library's roots; `nil` until the core announced them.
    private(set) var roots: [Locator]?
    private(set) var sources: [SourceRow] = []
    /// Every playlist, in the core's order (the sidebar's).
    private(set) var playlists: [PlaylistSummary] = []
    /// The whole library: how many tracks and how long they play.
    private(set) var summary = Summary(tracks: 0, length: 0)
    private(set) var facets = Facets(genres: [], codecs: [], firstYear: nil, lastYear: nil)
    /// A scan runs; the engine starts with one.
    private(set) var scan: ScanState? = ScanState(libraries: [], found: 0, read: 0)
    /// When the last complete scan of every source finished in this launch.
    private(set) var lastCompleteScan: Date?
    /// The copies of streamed tracks: on or off, how much they may take and take now, in bytes
    /// (`StreamCacheState` is the core's record).
    private(set) var streamCache: StreamCacheState?
    /// The library was read once since launch: lists show nothing before (spec §9.2).
    private(set) var ready = false
    /// No sources were ever chosen: the import page fills the window until they are.
    private(set) var firstRun: Bool

    // MARK: Telegram: `nil` while there is none, as in a build without N Music's credentials.

    private(set) var telegram: TelegramState?
    private(set) var telegramChats: TelegramChats?

    /// Called on the main actor for every event, after the store applied it.
    @ObservationIgnored var observers: [(CoreEvent) -> Void] = []

    @ObservationIgnored private var scanReport: (libraries: [Locator], found: UInt64, pending: UInt64, base: UInt64)?
    @ObservationIgnored private var seekRequest: UInt64 = 0
    @ObservationIgnored private var unmutedVolume: Double = 0
    @ObservationIgnored private var changePending = false
    @ObservationIgnored private var changeCounting = false
    @ObservationIgnored private var refreshing: Task<Void, Never>?
    @ObservationIgnored private var detailsTask: Task<Void, Never>?
    @ObservationIgnored private var knownPlaylists: Set<PlaylistId>?
    @ObservationIgnored private var creatingPlaylists = 0

    private init() {
        let home = URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
        // The Qt desktop app's folders, so its library and settings carry over.
        dataDirectory = home.appendingPathComponent("Library/Application Support/n_music", isDirectory: true)
        cacheDirectory = home.appendingPathComponent("Library/Caches/n_music", isDirectory: true)
        let core = Core.start(dataDir: dataDirectory.path, cacheDir: cacheDirectory.path, privateDir: dataDirectory.path)
        self.core = core
        options = core.playbackOptions()
        firstRun = core.firstRun()
        outputDevice = Self.savedOutputDevice(core)
        // The player applies its saved settings and reports volume, repeat and shuffle; these
        // are the values until the reports come.
        if let playback = Self.section(core, "core.playback") {
            volume = (playback["volume"] as? NSNumber)?.doubleValue ?? 1
            shuffle = playback["shuffle"] as? Bool ?? false
            switch playback["loop_status"] as? String {
            case "Playlist": loopStatus = .playlist
            case "File": loopStatus = .file
            default: loopStatus = .off
            }
        }
        Self.listen(core)
        send(.listOutputDevices)
    }

    /// Reads the core's events off the main thread for as long as the app runs.
    nonisolated private static func listen(_ core: Core) {
        Task.detached(priority: .userInitiated) {
            while let event = await core.nextEvent() {
                await MainActor.run { Store.shared.apply(event) }
            }
            // The core never stops by itself, and ends the process when its bus fails.
            NSLog("N Music: the core stopped")
        }
    }

    /// Sends `command` as is. Never blocks: safe from the main thread. Prefer the typed methods.
    func send(_ command: Command) {
        core.send(command: command)
    }

    /// Stops the core cleanly, off the main thread, then calls `done` on the main thread, at
    /// most once: the app quits then. A core that takes longer than `shutdownLimit` is left
    /// behind (the app quits anyway). Calls while it stops only wait for the same end.
    ///
    /// The end comes back through the run loop in its common modes, not through the main
    /// queue: AppKit waits for the reply in a modal run loop, and when the quit began inside a
    /// main-actor job (a task calling `NSApp.terminate`) that loop never serves the main queue.
    func shutdown(_ done: @escaping @MainActor () -> Void) {
        shutdownDone.append(done)
        guard !shuttingDown else { return }
        shuttingDown = true
        let core = self.core
        let finished: @Sendable () -> Void = {
            MainRunLoop.perform(in: [.common]) { Store.shared.shutdownFinished() }
        }
        Task.detached(priority: .userInitiated) {
            // Blocks until the bus stopped (the session saved by its shutdown handlers) and the
            // settings were flushed to disk. Idempotent; the event stream ends afterwards.
            core.shutdown()
            finished()
        }
        DispatchQueue.global(qos: .userInitiated).asyncAfter(deadline: .now() + Self.shutdownLimit, execute: finished)
    }

    nonisolated private static let shutdownLimit: TimeInterval = 5
    @ObservationIgnored private var shuttingDown = false
    @ObservationIgnored private var shutdownDone: [@MainActor () -> Void] = []

    private func shutdownFinished() {
        let done = shutdownDone
        shutdownDone = []
        guard !done.isEmpty else { return }
        #if DEBUG
        NSLog("N Music: the core shut down")
        #endif
        for call in done {
            call()
        }
    }

    /// Runs `read` with the core off the main thread; for queries without a typed method.
    nonisolated func query<T: Sendable>(_ read: @escaping @Sendable (Core) -> T) async -> T {
        let core = self.core
        return await Task.detached(priority: .userInitiated) { read(core) }.value
    }

    // MARK: - Events

    private func apply(_ event: CoreEvent) {
        switch event {
        case .playbackChanged(let playing):
            settlePosition()
            self.playing = playing
        case .trackChanged(let item, let track):
            let previous = current
            current = CurrentTrack(item: item, track: track)
            if previous?.item != item {
                let place = { (item: ItemId?) in item.flatMap { item in self.queue.firstIndex { $0.item == item } } }
                if let was = place(previous?.item), let now = place(item) {
                    trackChangeBack = now < was
                } else {
                    trackChangeBack = false
                }
                trackChange += 1
                reportedLength = 0
            }
            if previous?.track.locator != track.locator || details == nil {
                // The previous track's facts never show for this one while its own are read.
                if previous?.track.locator != track.locator { details = nil }
                loadDetails(track.locator)
            }
        case .positionChanged(let position, let length, let seek, let discontinuity):
            // Positions from before the last seek would make the slider jump back.
            guard seek >= seekRequest else { break }
            if reportedLength != length { reportedLength = length }
            let predicted = self.position()
            if discontinuity || abs(predicted - position) > 0.35 || (!playing && basePosition != position) {
                basePosition = position
                positionAt = .now
            }
        case .volumeChanged(let volume):
            self.volume = volume
        case .loopStatusChanged(let status):
            loopStatus = status
        case .shuffleChanged(let enabled):
            shuffle = enabled
        case .queueChanged(let entries):
            queue = entries
        case .trackPlayed(let locator):
            if current?.track.locator == locator { loadDetails(locator) }
            libraryChanged()
        case .playlistsChanged(let playlists):
            playlistsChanged(playlists)
        case .playlistRejected(let reason):
            Toasts.shared.showRejected(reason)
        case .libraryChanged(let read):
            if let report = scanReport {
                scan = Self.scanState(report, read: read)
            }
            libraryChanged()
        case .scanProgress(let libraries, let found, let pending, let read):
            if libraries.isEmpty {
                scanReport = nil
                scan = nil
            } else {
                scanReport = (libraries, found, pending, read)
                scan = Self.scanState(scanReport!, read: read)
            }
        case .scanFinished(let libraries, let complete):
            if complete, let roots, roots.allSatisfy(libraries.contains) {
                lastCompleteScan = Date()
            }
            bumpLibrary()
        case .libraryRootsChanged(let roots):
            self.roots = roots
            Navigation.shared.prune(roots: roots)
            // `setSources` ("Build my library") ends the first run in the core.
            if firstRun, !core.firstRun() { firstRun = false }
            refresh()
        case .streamCacheChanged(let enabled, let limit, let used):
            streamCache = StreamCacheState(enabled: enabled, limit: limit, used: used)
        case .outputDevices(let devices):
            outputDevices = devices
        case .telegramStatusChanged(let status, let busy, let error):
            let before = telegram
            let ended = (before?.ended ?? 0) + (before?.busy == true && !busy ? 1 : 0)
            telegram = TelegramState(status: status, busy: busy, error: error, ended: ended)
        case .telegramChatsFound(let query, let chats, let error):
            telegramChats = TelegramChats(
                query: query, chats: chats, error: error, serial: (telegramChats?.serial ?? 0) + 1
            )
        }
        for observer in observers {
            observer(event)
        }
    }

    private static func scanState(
        _ report: (libraries: [Locator], found: UInt64, pending: UInt64, base: UInt64),
        read: UInt64
    ) -> ScanState {
        let since = read > report.base ? read - report.base : 0
        let unread = report.pending > since ? report.pending - since : 0
        return ScanState(libraries: report.libraries, found: report.found, read: report.found - min(unread, report.found))
    }

    private func playlistsChanged(_ playlists: [PlaylistSummary]) {
        self.playlists = playlists
        let ids = Set(playlists.map(\.id))
        Navigation.shared.prune(playlists: ids)
        // A playlist this app asked for opens as it arrives.
        if let known = knownPlaylists {
            for playlist in playlists where !known.contains(playlist.id) {
                guard creatingPlaylists > 0 else { break }
                creatingPlaylists -= 1
                Navigation.shared.go(PageID.playlist(playlist.id))
            }
        }
        knownPlaylists = ids
        playlistsVersion += 1
    }

    /// Counts library changes at most once a second: a scan reports every track it reads.
    private func libraryChanged() {
        changePending = true
        guard !changeCounting else { return }
        changeCounting = true
        Task {
            while changePending {
                changePending = false
                bumpLibrary()
                try? await Task.sleep(for: .seconds(1))
            }
            changeCounting = false
        }
    }

    private func bumpLibrary() {
        libraryVersion += 1
        refresh()
    }

    /// Reads the sources, the library's summary and the facets again.
    private func refresh() {
        refreshing?.cancel()
        let roots = roots ?? []
        refreshing = Task {
            let read = await query { core in
                (core.sources(roots: roots), core.summary(filter: .everything), core.facets())
            }
            guard !Task.isCancelled else { return }
            if sources != read.0 { sources = read.0 }
            if summary != read.1 { summary = read.1 }
            if facets != read.2 { facets = read.2 }
            ready = true
        }
    }

    private func loadDetails(_ locator: Locator) {
        detailsTask?.cancel()
        detailsTask = Task {
            let details = await query { $0.trackDetails(locator: locator) }
            guard !Task.isCancelled else { return }
            self.details = details
        }
    }

    // MARK: - Position

    /// The current track's length: the decoder's, else its tags'; 0 while unknown.
    var length: Double {
        reportedLength > 0 ? reportedLength : (current?.track.length ?? 0)
    }

    /// Where playback is at `now`, advancing linearly from the last report while playing.
    func position(at now: ContinuousClock.Instant = .now) -> Double {
        guard playing else { return basePosition }
        let elapsed = now - positionAt
        let seconds = Double(elapsed.components.seconds) + Double(elapsed.components.attoseconds) / 1e18
        let advanced = basePosition + max(0, seconds)
        let length = self.length
        return length > 0 ? min(advanced, length) : advanced
    }

    private func settlePosition() {
        basePosition = position()
        positionAt = .now
    }

    /// Seeks the current track to `seconds`: the position shows at once, and reports from
    /// before the seek are ignored.
    func seek(to seconds: Double) {
        seekRequest += 1
        basePosition = max(0, seconds)
        positionAt = .now
        send(.seek(seek: .tracked(position: basePosition, request: seekRequest)))
    }

    func seek(by seconds: Double) {
        let length = self.length
        seek(to: min(max(0, position() + seconds), length > 0 ? length : .infinity))
    }

    /// Plays session item `item` from `position` seconds (a queue row's double-click).
    func play(item: ItemId, at position: Double = 0) {
        send(.seek(seek: .toItem(item: item, position: position)))
    }

    // MARK: - Playback commands

    /// Plays what `query` selects, from `start` (else its first track).
    func play(_ query: Query, from start: Locator? = nil) {
        send(.playFrom(query: query, start: start))
    }

    /// Plays what `query` selects in shuffled order.
    func shufflePlay(_ query: Query) {
        if !shuffle { setShuffle(true) }
        play(query)
    }

    /// Pauses; or plays the current track; or, with none, the library (the core decides).
    func togglePause() { send(.togglePause) }
    func play() { send(.play) }
    func pause() { send(.pause) }
    func next() { send(.playNext) }
    /// The previous track, or the start of this one when past its first seconds.
    func previous() { send(.playPrevious) }
    /// The start of this track, whatever the position.
    func skipBack() { send(.skipBack) }

    /// Queues `tracks` to play next (`next`) or after the queued ones. `title` names them in
    /// the toast shown when their place in Up next is out of sight; `nil` shows none.
    func enqueue(_ tracks: [Locator], next: Bool, title: String? = nil) {
        guard !tracks.isEmpty else { return }
        send(.enqueue(tracks: tracks, next: next))
        if let title { Navigation.shared.queued(title, next: next, count: tracks.count) }
    }

    func removeQueued(_ item: ItemId) { send(.removeQueued(item: item)) }
    func clearQueued() { send(.clearQueued) }
    /// Moves an upcoming item before `before` (`nil`: to the end).
    func moveUpcoming(_ item: ItemId, before: ItemId?) { send(.moveUpcoming(item: item, before: before)) }
    /// Moves the current item before `before`, keeping it current.
    func moveCurrent(_ item: ItemId, before: ItemId?) { send(.moveCurrent(item: item, before: before)) }

    func setVolume(_ volume: Double) {
        let volume = min(max(volume, 0), 1)
        self.volume = volume
        send(.setVolume(volume: volume))
    }

    var muted: Bool { volume <= 0 }

    /// Silences playback, saving the volume; or brings it back (0.5 when none was saved).
    func toggleMute() {
        if volume > 0 {
            unmutedVolume = volume
            setVolume(0)
        } else {
            setVolume(unmutedVolume > 0 ? unmutedVolume : 0.5)
        }
    }

    func setShuffle(_ enabled: Bool) {
        shuffle = enabled
        send(.setShuffle(enabled: enabled))
    }

    func toggleShuffle() { setShuffle(!shuffle) }

    func setLoopStatus(_ status: LoopStatus) {
        loopStatus = status
        send(.setLoopStatus(status: status))
    }

    /// Off → repeat all → repeat one → off, as the repeat button does.
    func cycleRepeat() { setLoopStatus(loopStatus.cycled) }
    /// The core's own repeat toggle.
    func toggleRepeat() { send(.toggleRepeat) }

    func setOutputDevice(_ device: OutputDevice?) {
        outputDevice = device
        send(.setOutputDevice(device: device))
    }

    /// Asks the core for the outputs again; `outputDevices` changes when it answers.
    func listOutputDevices() { send(.listOutputDevices) }
    /// The system's devices changed: the core follows the default again if it must.
    func outputDeviceChanged() { send(.outputDeviceChanged) }

    func setReplayGain(_ mode: ReplayGainMode) {
        options.replayGain = mode
        send(.setReplayGain(mode: mode))
    }

    func setResume(_ enabled: Bool) {
        options.resume = enabled
        send(.setResume(enabled: enabled))
    }

    func setCrossfade(_ seconds: Double) {
        options.crossfade = seconds
        send(.setCrossfade(seconds: seconds))
    }

    /// Plays from a deeper buffer where the platform can (on the Mac: the power-saving I/O
    /// buffer, when the core supports it).
    func setOffload(_ enabled: Bool) {
        options.offload = enabled
        send(.setOffload(enabled: enabled))
    }

    /// Switches the output device to each track's sample rate (Settings › Playback). Turning it
    /// off leaves the device at the rate it has; either way the output opens again at once.
    func setMatchRate(_ enabled: Bool) {
        options.matchRate = enabled
        send(.setMatchRate(enabled: enabled))
    }

    /// The fade of the sleep timer: it ends `endsIn` from now after `fade` of fading; `nil`
    /// plays at full volume again.
    func setSleepFade(endsIn: Duration?, fade: Duration) {
        let milliseconds = { (duration: Duration) in
            duration.components.seconds * 1000 + duration.components.attoseconds / 1_000_000_000_000_000
        }
        send(.setSleepFade(endsInMs: endsIn.map(milliseconds), fadeMs: UInt64(max(0, milliseconds(fade)))))
    }

    /// Tells the core whether a window of the app is on screen (see `Power`).
    func setAppVisible(_ visible: Bool) {
        windowVisible = visible
        send(.appVisibilityChanged(visible: visible))
    }

    // MARK: - Playlists

    /// Creates a playlist; it opens as it arrives. `rule` makes it smart.
    func createPlaylist(name: String, rule: Filter? = nil, sort: [SortKey] = [], tracks: [Locator] = []) {
        creatingPlaylists += 1
        send(.createPlaylist(name: name, rule: rule, sort: sort, tracks: tracks))
    }

    func renamePlaylist(_ id: PlaylistId, to name: String) { send(.renamePlaylist(id: id, name: name)) }
    func deletePlaylist(_ id: PlaylistId) { send(.deletePlaylist(id: id)) }
    func setPlaylistRule(_ id: PlaylistId, rule: Filter?) { send(.setPlaylistRule(id: id, rule: rule)) }
    func setPlaylistSort(_ id: PlaylistId, sort: [SortKey]) { send(.setPlaylistSort(id: id, sort: sort)) }
    func addToPlaylist(_ id: PlaylistId, tracks: [Locator]) { send(.addToPlaylist(id: id, tracks: tracks)) }
    func removeFromPlaylist(_ id: PlaylistId, tracks: [Locator]) { send(.removeFromPlaylist(id: id, tracks: tracks)) }

    // MARK: - Sources and scans

    /// Scans `library` (`nil`: every source). `checkCache` false reads every tag and cover again
    /// (Reload metadata); true picks up what changed (Update).
    func scan(_ library: Locator? = nil, checkCache: Bool = true) {
        send(.scanRequested(library: library, checkCache: checkCache))
    }

    /// Sets the library's roots: new ones are read, the tracks of those left out leave. Ends the
    /// first run.
    func setRoots(_ roots: [Locator]) {
        self.roots = roots
        Navigation.shared.prune(roots: roots)
        firstRun = false
        send(.setLibraryRoots(roots: roots))
        refresh()
    }

    func addRoots(_ added: [Locator]) {
        let known = roots ?? []
        let new = added.filter { !known.contains($0) }
        guard !new.isEmpty else { return }
        setRoots(known + new)
    }

    func removeRoot(_ root: Locator) {
        setRoots((roots ?? []).filter { $0 != root })
    }

    /// Names a source; `nil` (or empty) goes back to its default name.
    func renameSource(_ root: Locator, to name: String?) {
        let name = name?.trimmingCharacters(in: .whitespacesAndNewlines)
        send(.renameLibrary(library: root, name: name?.isEmpty == false ? name : nil))
    }

    func setStreamCache(enabled: Bool, limit: UInt64) {
        streamCache?.enabled = enabled
        streamCache?.limit = limit
        send(.setStreamCache(enabled: enabled, limit: limit))
    }

    // MARK: - Telegram

    func telegramSignIn(phone: String) { send(.telegramSignIn(phone: phone)) }
    func telegramCode(_ code: String) { send(.telegramCode(code: code)) }
    func telegramPassword(_ password: String) { send(.telegramPassword(password: password)) }
    func telegramSignOut() { send(.telegramSignOut) }
    func findTelegramChats(_ query: String) { send(.findTelegramChats(query: query)) }

    // MARK: - Queries (off the main thread)

    /// The tracks `query` selects, in its order: `limit` of them from `offset`.
    func tracks(_ query: Query, offset: UInt32 = 0, limit: UInt32 = .max) async -> [TrackRow] {
        await self.query { $0.tracks(query: query, offset: offset, limit: limit) }
    }

    func count(_ query: Query) async -> UInt32 {
        await self.query { $0.count(query: query) }
    }

    func summary(_ filter: Filter) async -> Summary {
        await query { $0.summary(filter: filter) }
    }

    func track(_ locator: Locator) async -> TrackRow? {
        await query { $0.track(locator: locator) }
    }

    func trackDetails(_ locator: Locator) async -> TrackDetails? {
        await query { $0.trackDetails(locator: locator) }
    }

    /// The picture embedded in the file, still encoded; reads the file.
    func coverArt(_ locator: Locator) async -> Data? {
        await query { $0.coverArt(locator: locator) }
    }

    func albums(_ filter: Filter = .everything, search: String = "", sort: GroupSort) async -> [AlbumRow] {
        await query { $0.albums(filter: filter, search: search, sort: sort) }
    }

    func artists(_ filter: Filter = .everything, search: String = "", sort: GroupSort) async -> [ArtistRow] {
        await query { $0.artists(filter: filter, search: search, sort: sort) }
    }

    func genres(_ filter: Filter = .everything, search: String = "", sort: GroupSort) async -> [GenreRow] {
        await query { $0.genres(filter: filter, search: search, sort: sort) }
    }

    func loadFacets() async -> Facets {
        await query { $0.facets() }
    }

    /// Every playlist with what it holds, by name.
    func playlistRows() async -> [PlaylistRow] {
        await query { $0.playlists() }
    }

    func playlist(_ id: PlaylistId) async -> PlaylistRow? {
        await query { $0.playlist(id: id) }
    }

    /// How many of `tracks` playlist `id` has.
    func playlistHolds(_ id: PlaylistId, tracks: [Locator]) async -> UInt32 {
        await query { $0.playlistHolds(id: id, tracks: tracks) }
    }

    func loadSources(_ roots: [Locator]) async -> [SourceRow] {
        await query { $0.sources(roots: roots) }
    }

    func missingTracks(_ root: Locator) async -> [TrackRow] {
        await query { $0.missingTracks(root: root) }
    }

    func playbackOptions() async -> PlaybackOptions {
        await query { $0.playbackOptions() }
    }

    // MARK: - Settings sections

    /// The settings section `key` as a JSON object; `nil` when there is none.
    func settingsSection(_ key: String) -> [String: Any]? { Self.section(core, key) }

    /// Stores `object` as the settings section `key`.
    func setSettingsSection(_ key: String, _ object: [String: Any]) {
        guard let data = try? JSONSerialization.data(withJSONObject: object, options: [.sortedKeys]) else { return }
        core.setSetting(key: key, json: String(decoding: data, as: UTF8.self))
    }

    nonisolated private static func section(_ core: Core, _ key: String) -> [String: Any]? {
        guard let json = core.setting(key: key) else { return nil }
        return try? JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any]
    }

    nonisolated private static func savedOutputDevice(_ core: Core) -> OutputDevice? {
        guard let device = section(core, "core.playback")?["output_device"] as? [String: Any],
              let id = device["id"] as? String
        else { return nil }
        return OutputDevice(id: id, name: device["name"] as? String ?? id)
    }
}
