import Foundation
import Observation

/// Where the play session came from, for the queue's "Playing from %1" and its "Open in
/// library" link: the label, detail and page of the track list that last started playing (Qt's
/// `TrackList.label`, `detail`, `origin`). `nil` until a list played.
@MainActor
@Observable
final class PlayingFrom {
    static let shared = PlayingFrom()

    struct Context: Equatable, Sendable {
        /// Like "Tracks", an album's or a playlist's name.
        var label: String
        /// Like “foo” · Genre is J-Pop; empty without one.
        var detail: String
        /// The page to open the list on.
        var page: String
    }

    private(set) var context: Context?

    private init() {}

    func set(_ context: Context) { self.context = context }
}

/// A track list as the Qt app's `TrackList`: the tracks of `playlist` (all without one) that
/// match `search` and `filter` (editor JSON, see `Filters`), in the order of `sort`. It reads
/// its count and length, and its rows a page at a time as the table shows them, off the main
/// thread, and reads them again when the library or the playlists change.
///
/// A page keeps one in `@State` (or a singleton, as Tracks does for the filter drawer) and
/// hands it to `TrackTable`; its header reads `ready`, `count`, `total`, `length`.
///
///     @State private var list = TrackList(sort: "album", label: name, origin: page)
///     …
///     TrackTable(list: list, layout: .library)
///         .onChange(of: argument) { list.filter = collectionFilter(…) }
@MainActor
@Observable
final class TrackList {
    // MARK: What it lists

    /// Typed text; matching re-reads 150 ms after the last keystroke.
    var search = "" {
        didSet { if oldValue != search { scheduleSearch() } }
    }
    /// Rules as the editor writes them; "" for none.
    var filter = "" {
        didSet { if oldValue != filter { reload(.query) } }
    }
    /// Like `artist,album` or `-year,album`.
    var sort: String {
        didSet { if oldValue != sort { reload(.sort) } }
    }
    var playlist: PlaylistId? {
        didSet { if oldValue != playlist { reload(.query) } }
    }
    /// While set the list reads nothing and lists nothing (a source page without its source,
    /// whose filter would otherwise match the whole library); clearing it reads again.
    var suspended = false {
        didSet {
            guard oldValue != suspended else { return }
            if suspended {
                generation += 1
                reading?.cancel()
                loading.removeAll()
            } else if watching {
                reload(.query)
            }
        }
    }

    /// What the queue says it plays from: "Tracks", an album's name…
    var label: String
    /// The page showing the list (`PageID`): "Go to album" leaves out the page shown, and the
    /// queue's "Open in library" goes back to it.
    var origin: String
    /// More on what it lists, for the queue (the search and the rules in words). `nil`: the
    /// search in quotes, then the rules' summary, as Tracks writes it.
    var detail: String?

    // MARK: What it found

    /// The first answer came: nothing shows before (spec §9.2).
    private(set) var ready = false
    /// The tracks listed.
    private(set) var count = 0
    /// The tracks of the whole library.
    private(set) var total = 0
    /// How long the listed tracks play, in seconds.
    private(set) var length: Double = 0
    /// Up to four covers of the listed tracks.
    private(set) var covers: [String] = []
    /// Goes up when the order changed and the new rows came (the table's sort feedback).
    private(set) var sortChange = 0

    /// The query the list runs.
    var query: Query {
        tracksQuery(search: searched, filter: filter, sort: sort, playlist: playlist)
    }

    /// What the table is told of changes (it registers itself).
    enum Change {
        /// Other rows, or another number of them: everything shows again. `top`: scroll to the
        /// first row (another search, filter or order).
        case reset(top: Bool)
        /// The same number of rows, read again: they change in place, the scroll stays.
        case inPlace
        /// Rows that were loading came.
        case rows(Range<Int>)
    }

    @ObservationIgnored var onChange: ((Change) -> Void)?
    /// The rows on screen, which the table reports as it scrolls: they load first.
    @ObservationIgnored var visibleRows: Range<Int> = 0..<0 {
        didSet { trim() }
    }

    // MARK: Private

    nonisolated private static let pageSize = 100
    /// Pages kept around the rows on screen; the others are read again when they show.
    private static let keptPages = 30

    /// What the search was when matching last ran.
    @ObservationIgnored private var searched = ""
    @ObservationIgnored private var pages: [Int: [TrackEntry]] = [:]
    @ObservationIgnored private var loading: Set<Int> = []
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var reading: Task<Void, Never>?
    @ObservationIgnored private var searching: Task<Void, Never>?
    @ObservationIgnored private var watching = false

    init(search: String = "", filter: String = "", sort: String = "artist,album",
         playlist: PlaylistId? = nil, label: String = "", origin: String = PageID.tracks, detail: String? = nil) {
        self.search = search
        self.searched = search
        self.filter = filter
        self.sort = sort
        self.playlist = playlist
        self.label = label
        self.origin = origin
        self.detail = detail
    }

    /// Starts reading (and following the library). The table does it as it shows; a page that
    /// shows its table only once there are rows calls it as it appears. Making a list reads
    /// nothing: SwiftUI makes and drops `@State` initial values freely.
    func activate() {
        guard !watching else { return }
        watching = true
        watch()
        reload(.query)
    }

    /// The row's track and columns; `nil` while its page loads (it then loads, and the table
    /// hears `.rows`).
    func entry(at row: Int) -> TrackEntry? {
        guard row >= 0, row < count else { return nil }
        let page = row / Self.pageSize
        if let rows = pages[page] {
            let index = row - page * Self.pageSize
            return index < rows.count ? rows[index] : nil
        }
        load(page)
        return nil
    }

    /// The row's track and columns if its page is loaded; reads nothing.
    func loadedEntry(at row: Int) -> TrackEntry? {
        guard row >= 0, row < count, let rows = pages[row / Self.pageSize] else { return nil }
        let index = row % Self.pageSize
        return index < rows.count ? rows[index] : nil
    }

    /// The tracks of `rows`, reading those not loaded (without keeping them: a ⌘A of a large
    /// library would otherwise hold all of it).
    func entries(at rows: IndexSet) async -> [TrackEntry] {
        var known = pages
        let wanted = Set(rows.map { $0 / Self.pageSize }).subtracting(known.keys)
        if !wanted.isEmpty {
            let query = self.query
            let playlist = self.playlist
            let read = await Store.shared.query { core in
                wanted.map { page in
                    (page, core.trackEntries(query: query, playlist: playlist,
                                             offset: UInt32(page * Self.pageSize), limit: UInt32(Self.pageSize)))
                }
            }
            for (page, rows) in read {
                known[page] = rows
            }
        }
        return rows.compactMap { row in
            guard let page = known[row / Self.pageSize] else { return nil }
            let index = row % Self.pageSize
            return index < page.count ? page[index] : nil
        }
    }

    // MARK: Playing

    /// The queue's context for this list.
    var context: PlayingFrom.Context {
        let detail = self.detail ?? {
            let text = searched.trimmingCharacters(in: .whitespacesAndNewlines)
            return [text.isEmpty ? "" : tr("search_detail", text), Filters.summary(filter)]
                .filter { !$0.isEmpty }.joined(separator: " · ")
        }()
        return PlayingFrom.Context(label: label, detail: detail, page: origin)
    }

    /// Plays the list from `row` (double-click, Return).
    func play(row: Int) {
        guard let entry = entry(at: row) else {
            Task {
                if let entry = await entries(at: IndexSet(integer: row)).first { play(entry.track.locator) }
            }
            return
        }
        play(entry.track.locator)
    }

    func play(_ locator: Locator) {
        PlayingFrom.shared.set(context)
        Store.shared.play(query, from: locator)
    }

    /// Play (shuffle off, from the first row) or Shuffle (shuffle on).
    func playAll(shuffle: Bool) {
        guard count > 0, !suspended else { return }
        PlayingFrom.shared.set(context)
        if shuffle {
            Store.shared.shufflePlay(query)
        } else {
            if Store.shared.shuffle { Store.shared.setShuffle(false) }
            Store.shared.play(query)
        }
    }

    // MARK: Reading

    private enum Reason { case query, sort, library }

    private func scheduleSearch() {
        searching?.cancel()
        searching = Task {
            try? await Task.sleep(for: .milliseconds(150))
            guard !Task.isCancelled else { return }
            searched = search
            reload(.query)
        }
    }

    /// Re-reads when the library's tracks or the playlists change.
    private func watch() {
        withObservationTracking {
            _ = Store.shared.libraryVersion
            _ = Store.shared.playlistsVersion
        } onChange: { [weak self] in
            Task { @MainActor in
                guard let self else { return }
                self.watch()
                self.reload(.library)
            }
        }
    }

    /// Reads the count and the rows on screen again; only the latest answer applies. The same
    /// number of rows after a library change changes them in place.
    private func reload(_ reason: Reason) {
        guard !suspended else { return }
        generation += 1
        let generation = self.generation
        let query = self.query
        let playlist = self.playlist
        let before = count
        let top = reason != .library
        // The pages to show at once: the first ones for a new order or search, else those on
        // screen.
        let shown = top ? 0..<max(1, min(visibleRows.count, Self.pageSize * 2)) : visibleRows
        let firstPage = max(0, shown.lowerBound / Self.pageSize)
        let lastPage = max(firstPage, (max(shown.upperBound, 1) - 1) / Self.pageSize)
        reading?.cancel()
        loading.removeAll()
        reading = Task {
            let read = await Store.shared.query { core -> (ListSummary, [Int: [TrackEntry]]) in
                let summary = core.listSummary(query: query)
                var pages: [Int: [TrackEntry]] = [:]
                let last = summary.tracks == 0 ? -1 : min(lastPage, (Int(summary.tracks) - 1) / Self.pageSize)
                if last >= firstPage {
                    for page in firstPage...last {
                        pages[page] = core.trackEntries(query: query, playlist: playlist,
                                                        offset: UInt32(page * Self.pageSize), limit: UInt32(Self.pageSize))
                    }
                }
                return (summary, pages)
            }
            guard !Task.isCancelled, generation == self.generation else { return }
            let (summary, fresh) = read
            pages = fresh
            loading.removeAll()
            let count = Int(summary.tracks)
            if self.count != count { self.count = count }
            if total != Int(summary.total) { total = Int(summary.total) }
            if length != summary.length { length = summary.length }
            if covers != summary.covers { covers = summary.covers }
            if !ready { ready = true }
            if reason == .library && count == before {
                onChange?(.inPlace)
            } else {
                onChange?(.reset(top: top))
            }
            if reason == .sort { sortChange += 1 }
        }
    }

    private func load(_ page: Int) {
        guard !suspended, !loading.contains(page) else { return }
        loading.insert(page)
        let generation = self.generation
        let query = self.query
        let playlist = self.playlist
        Task {
            let rows = await Store.shared.query { core in
                core.trackEntries(query: query, playlist: playlist,
                                  offset: UInt32(page * Self.pageSize), limit: UInt32(Self.pageSize))
            }
            guard generation == self.generation else { return }
            loading.remove(page)
            pages[page] = rows
            let start = page * Self.pageSize
            onChange?(.rows(start..<min(count, start + rows.count)))
        }
    }

    /// Forgets the pages far from the rows on screen.
    private func trim() {
        guard pages.count > Self.keptPages else { return }
        let middle = visibleRows.isEmpty ? 0 : (visibleRows.lowerBound + visibleRows.count / 2) / Self.pageSize
        let far = pages.keys.sorted { abs($0 - middle) > abs($1 - middle) }
        for page in far.prefix(pages.count - Self.keptPages) {
            pages.removeValue(forKey: page)
        }
    }
}
