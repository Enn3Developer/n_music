import Foundation

// Typed wrappers of the FFI's desktop exports (n_music_ffi/src/desktop.rs, rules.rs, queue.rs,
// telegram.rs, ui.rs): what the Qt bridge computed for its pages, ported to Rust so this app
// gets the same answers. Queries that read the library run off the main thread; the pure
// functions are cheap and run where they are called.
//
// This file follows the FFI as it grows: if an export changes, fix it here only.

extension Store {
    // MARK: Track lists

    /// Every column of `limit` tracks of `query` from `offset`; `playlist` fills "Added".
    func trackEntries(_ query: Query, playlist: PlaylistId? = nil, offset: UInt32 = 0, limit: UInt32 = .max) async -> [TrackEntry] {
        await self.query { $0.trackEntries(query: query, playlist: playlist, offset: offset, limit: limit) }
    }

    func trackEntry(_ locator: Locator, playlist: PlaylistId? = nil) async -> TrackEntry? {
        await query { $0.trackEntry(locator: locator, playlist: playlist) }
    }

    /// A list header's count and length.
    func listSummary(_ query: Query) async -> ListSummary {
        await self.query { $0.listSummary(query: query) }
    }

    /// Where `locator` is in `query`'s order (to scroll to the playing row).
    func index(of locator: Locator, in query: Query) async -> UInt32? {
        await self.query { $0.indexOf(query: query, locator: locator) }
    }

    /// The playlists holding `locator` (the track menu's checks).
    func playlists(with locator: Locator) async -> [PlaylistId] {
        await query { $0.playlistsWith(locator: locator) }
    }

    /// The library's values of `field`, for the rule editor's suggestions.
    func fieldValues(_ field: ValueField) async -> [String] {
        await query { $0.fieldValues(field: field) }
    }

    // MARK: Sources

    /// The roots as the settings keep them; `nil` before any were chosen.
    func libraryRoots() async -> [Locator]? {
        await query { $0.libraryRoots() }
    }

    /// The sources as the Sources page shows them.
    func sourceList() async -> [SourceInfo] {
        await query { $0.sourceList() }
    }

    func sourceName(_ root: Locator) async -> String {
        await query { $0.sourceName(root: root) }
    }

    /// Adds a source named `name` ("" for its default name); false when it is there already.
    @discardableResult
    func addSource(_ root: Locator, name: String) async -> Bool {
        await query { $0.addSource(root: root, name: name) }
    }

    @discardableResult
    func removeSource(_ root: Locator) async -> Bool {
        await query { $0.removeSource(root: root) }
    }

    /// Sets every source and its name at once, as "Build my library" does.
    func setSources(_ roots: [Locator], names: [String]) {
        core.setSources(roots: roots, names: names)
    }

    /// The core's own count of a running scan (the store tracks the same from events: `scan`).
    func scanStatus() async -> ScanStatus {
        await query { $0.scanStatus() }
    }

    // MARK: Output, Telegram, cache

    /// The devices to choose from, a chosen one that is not connected included.
    func outputChoices() async -> [OutputChoice] {
        await query { $0.outputChoices() }
    }

    func chosenOutputDevice() async -> OutputDevice? {
        await query { $0.outputDevice() }
    }

    func chooseOutputDevice(id: String?) {
        core.chooseOutputDevice(id: id)
    }

    func telegramSignInState() async -> TelegramSignInState? {
        await query { $0.telegramState() }
    }

    func streamCacheState() async -> StreamCacheState? {
        await query { $0.streamCache() }
    }

    // MARK: Settings

    /// `desktop.ui` as the FFI reads it (the app's live copy is `UISettings.shared`).
    func desktopUi() -> DesktopUi { core.desktopUi() }
    func setDesktopUi(_ preferences: DesktopUi) { core.setDesktopUi(preferences: preferences) }

    func settingField(_ key: String, _ field: String) -> String? {
        core.settingField(key: key, field: field)
    }

    func setSettingField(_ key: String, _ field: String, json: String) {
        core.setSettingField(key: key, field: field, json: json)
    }

    /// Where the logs are (Settings › About › Open logs folder).
    var logsDirectory: URL { URL(fileURLWithPath: core.logsDir(), isDirectory: true) }
}

// The pure functions (tracksQuery, parseSort, sortString, parseFilter, filterSpec,
// collectionFilter, queueLayout, queueMove, sourceFromLocation, sourceLocation,
// normalizeWebAddress, telegramErrorText, telegramChatKindKey, coreVersion) are called directly.
