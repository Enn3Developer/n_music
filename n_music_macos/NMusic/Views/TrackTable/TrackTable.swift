import AppKit
import SwiftUI

/// Which value columns a track table shows (spec §3.2, `qml/TrackColumns.qml`): `library`
/// year and plays (and last played, genre and format when roomy), `playlist` when tracks were
/// added, `smart` plays and when they were last played.
enum TrackTableLayout: Sendable, Equatable {
    case library, playlist, smart
}

/// The track table of Tracks, collection and playlist pages (spec §3.2): an `NSTableView` over a
/// `TrackList`, its rows read a page at a time as they show.
///
/// - Columns per §3.2 by the table's width and the window's size class (compact: no header, no
///   number or value columns), visibility from Settings › Track list columns; not resizable or
///   movable. Clicking a header sorts by it, again to reverse (writes `list.sort`).
/// - Single and multiple selection (click, ⇧/⌘-click, ⌘A); double-click or Return plays from
///   the row with the list's whole query; right-click, the row's "⋯" or ⇧F10 opens the track
///   menu (`TrackMenu`), for the selection when the row is in it.
/// - The playing row: `selected` ground, title in the accent, bars in the number column.
///
///     TrackTable(list: list, layout: .playlist, playlist: (id, name))
struct TrackTable: View {
    let list: TrackList
    var layout: TrackTableLayout = .library
    /// The playlist the list shows, offering "Remove from “%1”" in the track menu.
    var playlist: (id: PlaylistId, name: String)? = nil

    private let store = Store.shared
    private let settings = UISettings.shared
    private let navigation = Navigation.shared

    var body: some View {
        // The size class is the window's, as Qt's (the filter drawer covers the page).
        let width = navigation.windowWidth
        TrackTableRepresentable(config: TrackTableConfig(
            list: ObjectIdentifier(list),
            layout: layout,
            playlistID: playlist?.id,
            playlistName: playlist?.name,
            sort: list.sort,
            playing: store.current?.track.locator,
            isPlaying: store.playing,
            compact: width < 1100,
            dense: settings.compactRows,
            hidden: Set(settings.hiddenColumns),
            accent: settings.accent,
            language: Tr.shared.language ?? "",
            reduced: Motion.shared.reduced,
            visible: store.windowVisible
        ), list: list)
        .onAppear { list.activate() }
        .accessibilityLabel(tr("tracks"))
    }
}

/// What the table shows, compared to know what to update.
struct TrackTableConfig: Equatable {
    var list: ObjectIdentifier
    var layout: TrackTableLayout
    var playlistID: PlaylistId?
    var playlistName: String?
    var sort: String
    var playing: Locator?
    var isPlaying: Bool
    var compact: Bool
    var dense: Bool
    var hidden: Set<String>
    var accent: String
    var language: String
    var reduced: Bool
    var visible: Bool
}

/// The table's columns. `lead` and `trail` are the row's padding plus the table's side inset.
enum TrackColumn: String, CaseIterable {
    case lead, number, title, album, genre, year, added, plays, lastPlayed, format, time, more, trail

    var identifier: NSUserInterfaceItemIdentifier { NSUserInterfaceItemIdentifier("track." + rawValue) }

    /// The translation key of its header.
    var titleKey: String? {
        switch self {
        case .title: "column_title"
        case .album: "column_album"
        case .genre: "column_genre"
        case .year: "column_year"
        case .added: "column_added"
        case .plays: "column_plays"
        case .lastPlayed: "column_last_played"
        case .format: "column_format"
        case .time: "column_time"
        default: nil
        }
    }

    var alignRight: Bool {
        switch self {
        case .number, .year, .plays, .lastPlayed, .time: true
        default: false
        }
    }

    /// The sort a header click writes: the field, then the tie-breaker (`qml/TrackHeader.qml`).
    var sort: (field: String, then: String)? {
        switch self {
        case .title: ("title", "")
        case .album: ("album", "")
        case .genre: ("genre", "album")
        case .year: ("year", "album")
        case .added: ("added", "")
        case .plays: ("plays", "title")
        case .lastPlayed: ("lastPlayed", "")
        case .format: ("format", "album")
        case .time: ("length", "")
        default: nil
        }
    }

    /// The column a sort's first field puts the arrow on.
    static func sorted(by field: String) -> TrackColumn? {
        allCases.first { $0.sort?.field == field }
    }
}

/// Column widths of a table whose column area is `width` (`qml/TrackColumns.qml`); 0 hides.
struct TrackColumnWidths {
    /// The table's side inset (16, 10 in compact windows; `qml/TrackTable.qml:21`).
    static func inset(compact: Bool) -> CGFloat { compact ? 10 : 16 }
    /// The side inset plus the row's padding (12), each side.
    static func edge(compact: Bool) -> CGFloat { inset(compact: compact) + 12 }

    var widths: [TrackColumn: CGFloat] = [:]

    init(width: CGFloat, layout: TrackTableLayout, compact: Bool, hidden: Set<String>) {
        let slim = compact
        let shows = { (name: String) in !hidden.contains(name) }
        let roomy = width >= 1180 && !slim
        let number: CGFloat = !slim && shows("number") ? 44 : 0
        let year: CGFloat = !slim && layout == .library && shows("year") ? 56 : 0
        let plays: CGFloat = slim || !shows("plays") ? 0 : layout == .library ? 56 : layout == .smart ? 64 : 0
        let lastPlayed: CGFloat = !slim && shows("lastPlayed") && (layout == .smart || (roomy && layout == .library)) ? 120 : 0
        let added: CGFloat = !slim && layout == .playlist && shows("added") ? 110 : 0
        let format: CGFloat = roomy && shows("format") ? 90 : 0
        let time: CGFloat = shows("time") ? 60 : 0
        let more: CGFloat = 40
        let flexible = max(0, width - number - year - plays - lastPlayed - added - format - time - more)
        let hasAlbum = shows("album")
        let genre: CGFloat = roomy && shows("genre") ? max(90, flexible * 0.8 / 4.7) : 0
        let title: CGFloat = !hasAlbum ? flexible - genre
            : compact ? max(200, flexible * 2 / 3.2)
            : roomy ? max(260, flexible * 2.4 / 4.7)
            : max(220, flexible * 2.2 / 3.6)
        let album: CGFloat = hasAlbum ? max(compact ? 120 : 140, flexible - title - genre) : 0
        widths = [
            .lead: Self.edge(compact: compact), .number: number, .title: max(120, title), .album: album, .genre: genre,
            .year: year, .added: added, .plays: plays, .lastPlayed: lastPlayed, .format: format,
            .time: time, .more: more, .trail: Self.edge(compact: compact),
        ]
    }
}

// MARK: - The AppKit table

struct TrackTableRepresentable: NSViewRepresentable {
    let config: TrackTableConfig
    let list: TrackList

    func makeCoordinator() -> TrackTableController { TrackTableController() }

    func makeNSView(context: Context) -> NSScrollView {
        context.coordinator.make(list: list, config: config)
    }

    func updateNSView(_ view: NSScrollView, context: Context) {
        context.coordinator.update(list: list, config: config)
    }

    static func dismantleNSView(_ view: NSScrollView, coordinator: TrackTableController) {
        coordinator.dismantle()
    }
}

/// Lays the table out and answers it: rows from the list, cells, selection, menus, sorting.
@MainActor
final class TrackTableController: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    private(set) var list: TrackList?
    private var config: TrackTableConfig?
    private var scrollView: TrackScrollView!
    private(set) var table: TrackNSTable!
    private var header: NSTableHeaderView?
    private var columns: [TrackColumn: NSTableColumn] = [:]
    private var laidOutWidth: CGFloat = -1
    private var syncingSort = false
    private var lastSortChange = 0
    private var sortWatch = false
    /// The rows whose menu is open (their ground is `selected`).
    private var menuRows = IndexSet()
    /// The playing row's ground, under the rows.
    private let highlight = PlayingHighlightView()
    /// Its fading copy while it crossfades to another row.
    private let ghost = PlayingHighlightView()
    /// The row the highlight sits on.
    private var highlightRow: Int?
    private var lastTrackChange = Date.distantPast

    // MARK: Making

    func make(list: TrackList, config: TrackTableConfig) -> NSScrollView {
        let table = TrackNSTable()
        table.controller = self
        table.style = .fullWidth
        table.headerView = TrackHeaderView(frame: NSRect(x: 0, y: 0, width: 100, height: 36))
        header = table.headerView
        table.intercellSpacing = .zero
        table.allowsMultipleSelection = true
        table.allowsEmptySelection = true
        table.allowsColumnReordering = false
        table.allowsColumnResizing = false
        table.allowsColumnSelection = false
        table.allowsTypeSelect = false
        table.columnAutoresizingStyle = .noColumnAutoresizing
        table.usesAlternatingRowBackgroundColors = false
        table.backgroundColor = .clear
        table.gridStyleMask = []
        table.selectionHighlightStyle = .regular
        table.focusRingType = .none
        table.target = self
        table.doubleAction = #selector(doubleClicked(_:))
        table.setAccessibilityLabel(tr("tracks"))
        highlight.isHidden = true
        ghost.isHidden = true
        table.addSubview(ghost, positioned: .below, relativeTo: nil)
        table.addSubview(highlight, positioned: .below, relativeTo: nil)

        for column in TrackColumn.allCases {
            let tableColumn = NSTableColumn(identifier: column.identifier)
            tableColumn.resizingMask = []
            tableColumn.minWidth = 0
            tableColumn.maxWidth = 10_000
            let header = TrackHeaderCell(textCell: "")
            header.leading = column == .format ? 14 : 0
            header.trailing = column == .number ? 14 : 0
            header.sortable = column.sort != nil
            tableColumn.headerCell = header
            if let sort = column.sort {
                tableColumn.sortDescriptorPrototype = NSSortDescriptor(key: sort.field, ascending: true)
            }
            table.addTableColumn(tableColumn)
            columns[column] = tableColumn
        }

        let scrollView = TrackScrollView()
        scrollView.documentView = table
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.automaticallyAdjustsContentInsets = false
        scrollView.contentInsets = NSEdgeInsets(top: 0, left: 0, bottom: 12, right: 0)
        scrollView.onLayout = { [weak self] in self?.layoutColumns() }
        self.scrollView = scrollView
        self.table = table
        NotificationCenter.default.addObserver(
            self, selector: #selector(scrolled(_:)), name: NSView.boundsDidChangeNotification, object: scrollView.contentView
        )
        scrollView.contentView.postsBoundsChangedNotifications = true

        table.dataSource = self
        table.delegate = self
        update(list: list, config: config)
        return scrollView
    }

    func dismantle() {
        NotificationCenter.default.removeObserver(self)
        list?.onChange = nil
    }

    // MARK: Updating

    func update(list: TrackList, config: TrackTableConfig) {
        let old = self.config
        self.config = config
        if self.list !== list {
            self.list?.onChange = nil
            self.list = list
            list.onChange = { [weak self] change in self?.changed(change) }
            lastSortChange = list.sortChange
            table.reloadData()
            reportVisible()
        }
        if old?.compact != config.compact {
            table.headerView = config.compact ? nil : header
        }
        let dense = config.dense
        let rowHeight: CGFloat = dense ? 40 : 52
        if table.rowHeight != rowHeight {
            table.rowHeight = rowHeight
            if let highlightRow, highlightRow < table.numberOfRows {
                highlight.frame = highlightFrame(highlightRow)
            }
        }
        if old?.layout != config.layout || old?.compact != config.compact || old?.hidden != config.hidden {
            laidOutWidth = -1
            layoutColumns()
        }
        if old?.language != config.language || old?.sort != config.sort || old == nil {
            labelHeaders()
        }
        if old?.sort != config.sort || old == nil {
            syncSortIndicator()
        }
        let restyle = old?.dense != config.dense || old?.accent != config.accent || old?.language != config.language
            || old?.layout != config.layout || old?.compact != config.compact
        if restyle {
            table.reloadData()
            highlight.restyle()
            ghost.restyle()
            placeHighlight(.jump)
        } else if old?.playing != config.playing || old?.isPlaying != config.isPlaying
                    || old?.reduced != config.reduced || old?.visible != config.visible {
            refreshVisibleRows()
        }
        if old != nil, old?.playing != config.playing {
            placeHighlight(.track)
        }
        if list.sortChange != lastSortChange {
            lastSortChange = list.sortChange
            sortFeedback()
        }
    }

    private func changed(_ change: TrackList.Change) {
        switch change {
        case .reset(let top):
            let selected = table.selectedRowIndexes
            table.reloadData()
            if top {
                if table.numberOfRows > 0 { table.scrollRowToVisible(0) }
                table.deselectAll(nil)
            } else if let last = selected.last, last < table.numberOfRows {
                table.selectRowIndexes(selected, byExtendingSelection: false)
            }
            reportVisible()
            placeHighlight(.jump)
            if let list, list.sortChange != lastSortChange {
                lastSortChange = list.sortChange
                sortFeedback()
            }
        case .inPlace:
            refreshVisibleRows()
            placeHighlight(.jump)
        case .rows(let range):
            if highlightRow == nil { placeHighlight(.jump) }
            let visible = table.rows(in: table.visibleRect)
            let lower = max(range.lowerBound, visible.location)
            let upper = min(range.upperBound, visible.location + visible.length)
            guard lower < upper else { return }
            table.reloadData(forRowIndexes: IndexSet(integersIn: lower..<upper),
                             columnIndexes: IndexSet(integersIn: 0..<table.numberOfColumns))
        }
    }

    /// The order changed and its rows came: they fade from 60 % and the playing row is tinted
    /// with the accent for a moment, so the eye finds it again (spec §3.2, Sort feedback).
    private func sortFeedback() {
        table.alphaValue = 0.6
        NSAnimationContext.runAnimationGroup { context in
            context.duration = Motion.fade
            context.timingFunction = CAMediaTimingFunction(controlPoints: 0.2, 0, 0, 1)
            table.animator().alphaValue = 1
        }
        if highlightRow != nil, !highlight.isHidden {
            highlight.flash()
        }
    }

    // MARK: The playing highlight

    enum HighlightMove {
        /// The list changed (sort, search, rows): it goes straight to the row.
        case jump
        /// Another track plays: it glides there when near, else crossfades.
        case track
    }

    /// Finds the playing row among the loaded rows around those on screen.
    private func findPlayingRow() -> Int? {
        guard let list, let playing = config?.playing else { return nil }
        let visible = table.rows(in: table.visibleRect)
        let lower = max(0, visible.location - 100)
        let upper = min(table.numberOfRows, visible.location + visible.length + 100)
        guard lower < upper else { return nil }
        for row in lower..<upper where list.loadedEntry(at: row)?.track.locator == playing {
            return row
        }
        return nil
    }

    private func highlightFrame(_ row: Int) -> NSRect {
        let inset = TrackColumnWidths.inset(compact: config?.compact ?? false)
        return table.rect(ofRow: row).insetBy(dx: inset, dy: 0)
    }

    /// Puts the highlight on the playing row (`qml/PlayingHighlight.qml`): it glides 200 ms to
    /// a row at most 6 away, crossfades 150 ms to one farther (or under reduced motion), jumps
    /// for a change of the list or a track change within 250 ms of the last, fades in from
    /// nothing, and hides while the playing track is not in the list.
    func placeHighlight(_ move: HighlightMove) {
        let row = findPlayingRow()
        let now = Date()
        let hurried = now.timeIntervalSince(lastTrackChange) < 0.25
        if move == .track { lastTrackChange = now }
        guard let row else {
            highlightRow = nil
            highlight.isHidden = true
            return
        }
        highlight.picked = table.selectedRowIndexes.contains(row)
        let frame = highlightFrame(row)
        let reduced = config?.reduced ?? false
        if highlightRow == nil || highlight.isHidden {
            highlight.frame = frame
            highlight.isHidden = false
            if !reduced && move == .track {
                highlight.alphaValue = 0
                NSAnimationContext.runAnimationGroup { context in
                    context.duration = Motion.fade
                    highlight.animator().alphaValue = 1
                }
            } else {
                highlight.alphaValue = 1
            }
        } else if let old = highlightRow, old != row, move == .track, !hurried {
            if abs(row - old) <= 6 && !reduced {
                NSAnimationContext.runAnimationGroup { context in
                    context.duration = Motion.move
                    context.timingFunction = CAMediaTimingFunction(controlPoints: 0.2, 0, 0, 1)
                    highlight.animator().frame = frame
                }
            } else {
                ghost.picked = highlight.picked
                ghost.frame = highlight.frame
                ghost.isHidden = false
                ghost.alphaValue = 1
                highlight.frame = frame
                highlight.alphaValue = 0
                NSAnimationContext.runAnimationGroup { context in
                    context.duration = Motion.fade
                    ghost.animator().alphaValue = 0
                    highlight.animator().alphaValue = 1
                } completionHandler: { [weak self] in
                    MainActor.assumeIsolated { self?.ghost.isHidden = true }
                }
            }
        } else {
            highlight.frame = frame
            highlight.alphaValue = 1
        }
        highlightRow = row
    }

    /// Fills the rows on screen again (new data of the same rows, the playing track).
    private func refreshVisibleRows() {
        let visible = table.rows(in: table.visibleRect)
        guard visible.length > 0 else { return }
        for row in visible.location..<(visible.location + visible.length) {
            if let rowView = table.rowView(atRow: row, makeIfNecessary: false) as? TrackRowView {
                configure(rowView, row: row)
            }
            for (index, column) in table.tableColumns.enumerated() {
                guard let cell = table.view(atColumn: index, row: row, makeIfNecessary: false) as? TrackCellView,
                      let kind = TrackColumn(rawValue: String(column.identifier.rawValue.dropFirst("track.".count)))
                else { continue }
                fill(cell, kind: kind, row: row)
            }
        }
    }

    // MARK: Columns

    private func layoutColumns() {
        guard let config else { return }
        let width = scrollView.contentView.bounds.width
        guard width > 0, abs(width - laidOutWidth) > 0.5 else { return }
        laidOutWidth = width
        let area = max(0, width - 2 * TrackColumnWidths.edge(compact: config.compact))
        let widths = TrackColumnWidths(width: area, layout: config.layout, compact: config.compact, hidden: config.hidden)
        for (kind, column) in columns {
            let value = widths.widths[kind] ?? 0
            let hidden = value <= 0
            if column.isHidden != hidden { column.isHidden = hidden }
            if !hidden, abs(column.width - value) > 0.25 { column.width = value }
        }
        // The table may add room of its own beyond the columns (spacing, its style's padding):
        // the flexible column gives it back, so nothing scrolls sideways.
        table.tile()
        let overflow = table.frame.width - width
        if overflow > 0.5, let flexible = columns[widths.widths[.album, default: 0] > 0 ? .album : .title] {
            flexible.width = max(60, flexible.width - overflow)
        }
        if let highlightRow, highlightRow < table.numberOfRows {
            highlight.frame = highlightFrame(highlightRow)
        }
    }

    private func labelHeaders() {
        guard let config else { return }
        let first = config.sort.split(separator: ",").first.map(String.init) ?? ""
        let firstField = first.hasPrefix("-") ? String(first.dropFirst()) : first
        for (kind, column) in columns {
            guard let cell = column.headerCell as? TrackHeaderCell else { continue }
            let text: String = kind == .number ? "#" : kind.titleKey.map { tr($0) } ?? ""
            cell.configure(text: text, alignRight: kind.alignRight, sorted: kind.sort?.field == firstField,
                           ascending: !first.hasPrefix("-"))
            column.headerToolTip = nil
        }
        header?.needsDisplay = true
    }

    /// Puts the native sort arrow on the column of the sort's first key, if it has one.
    private func syncSortIndicator() {
        guard let config else { return }
        let first = config.sort.split(separator: ",").first.map(String.init) ?? ""
        let descending = first.hasPrefix("-")
        let field = descending ? String(first.dropFirst()) : first
        syncingSort = true
        if let column = TrackColumn.sorted(by: field), let sort = column.sort {
            table.sortDescriptors = [NSSortDescriptor(key: sort.field, ascending: !descending)]
        } else {
            table.sortDescriptors = []
        }
        syncingSort = false
    }

    func tableView(_ tableView: NSTableView, sortDescriptorsDidChange oldDescriptors: [NSSortDescriptor]) {
        guard !syncingSort, let list, let descriptor = tableView.sortDescriptors.first, let key = descriptor.key,
              let column = TrackColumn.sorted(by: key), let sort = column.sort
        else { return }
        list.sort = (descriptor.ascending ? "" : "-") + sort.field + (sort.then.isEmpty ? "" : "," + sort.then)
    }

    // MARK: Rows

    func numberOfRows(in tableView: NSTableView) -> Int {
        list?.count ?? 0
    }

    func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? {
        let identifier = NSUserInterfaceItemIdentifier("track.row")
        let view = tableView.makeView(withIdentifier: identifier, owner: nil) as? TrackRowView ?? {
            let view = TrackRowView()
            view.identifier = identifier
            return view
        }()
        configure(view, row: row)
        return view
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard let tableColumn,
              let kind = TrackColumn(rawValue: String(tableColumn.identifier.rawValue.dropFirst("track.".count)))
        else { return nil }
        if kind == .lead || kind == .trail { return nil }
        let cell = tableView.makeView(withIdentifier: tableColumn.identifier, owner: nil) as? TrackCellView ?? {
            let cell = TrackCellView.make(kind)
            cell.identifier = tableColumn.identifier
            return cell
        }()
        fill(cell, kind: kind, row: row)
        return cell
    }

    func tableView(_ tableView: NSTableView, shouldSelectRow row: Int) -> Bool { true }

    func tableViewSelectionDidChange(_ notification: Notification) {
        // The playing row's ground is `raised` while it is selected.
        if let highlightRow {
            highlight.picked = table.selectedRowIndexes.contains(highlightRow)
        }
    }

    /// Shows which rows have their menu open.
    private func showMenuRows(_ rows: IndexSet) {
        let changed = menuRows.symmetricDifference(rows)
        menuRows = rows
        let column = table.column(withIdentifier: TrackColumn.more.identifier)
        for row in changed where row < table.numberOfRows {
            if let rowView = table.rowView(atRow: row, makeIfNecessary: false) as? TrackRowView {
                configure(rowView, row: row)
            }
            if column >= 0, let cell = table.view(atColumn: column, row: row, makeIfNecessary: false) as? TrackCellView {
                fill(cell, kind: .more, row: row)
            }
        }
    }

    private func isPlaying(_ entry: TrackEntry?) -> Bool {
        guard let entry, let playing = config?.playing else { return false }
        return entry.track.locator == playing
    }

    private func configure(_ rowView: TrackRowView, row: Int) {
        rowView.inset = TrackColumnWidths.inset(compact: config?.compact ?? false)
        rowView.playing = isPlaying(list?.entry(at: row))
        rowView.menuOpen = menuRows.contains(row)
    }

    private func fill(_ cell: TrackCellView, kind: TrackColumn, row: Int) {
        guard let config else { return }
        let entry = list?.entry(at: row)
        let style = TrackCellView.Style(
            dense: config.dense,
            playing: isPlaying(entry),
            animating: config.isPlaying && config.visible && !config.reduced,
            compact: config.compact,
            menuOpen: menuRows.contains(row)
        )
        cell.fill(entry: entry, row: row, style: style, controller: self)
    }

    // MARK: Scrolling

    @objc private func scrolled(_ notification: Notification) {
        reportVisible()
        if highlightRow == nil, config?.playing != nil { placeHighlight(.jump) }
    }

    private func reportVisible() {
        guard let list else { return }
        let range = table.rows(in: table.visibleRect)
        let rows = range.location..<(range.location + range.length)
        if list.visibleRows != rows { list.visibleRows = rows }
    }

    // MARK: Playing

    @objc private func doubleClicked(_ sender: Any?) {
        let row = table.clickedRow
        guard row >= 0 else { return }
        list?.play(row: row)
    }

    /// Return: plays from the first selected row; nothing without one.
    func playSelected() {
        guard let row = table.selectedRowIndexes.first else { return }
        list?.play(row: row)
    }

    // MARK: The track menu

    /// The rows a menu opened on `row` acts on: the selection when `row` is in it.
    private func rows(for row: Int) -> IndexSet {
        let selected = table.selectedRowIndexes
        return selected.contains(row) ? selected : IndexSet(integer: row)
    }

    /// The tracks of `rows`, read first when some are not loaded (rows scrolled away from a
    /// ⌘A); `nil` when none could be read.
    private func tracks(of rows: IndexSet) async -> [TrackRow]? {
        guard let list else { return nil }
        let known = rows.compactMap { list.loadedEntry(at: $0) }
        let entries = known.count == rows.count ? known : await list.entries(at: rows)
        return entries.isEmpty ? nil : entries.map(\.track)
    }

    /// Makes the track menu of `rows`, once their tracks are known.
    func makeMenu(rows: IndexSet) async -> NSMenu? {
        guard let list, let config, let tracks = await tracks(of: rows) else { return nil }
        let playlist = config.playlistID.flatMap { id in config.playlistName.map { (id: id, name: $0) } }
        let menu = TrackMenu.make(tracks: tracks, origin: list.origin, playlist: playlist)
        return menu.numberOfItems > 0 ? menu : nil
    }

    /// Opens the track menu of `rows` at `point` of `view`, the row it opened on (`marked`)
    /// showing it as open until it closes.
    private func popUp(rows: IndexSet, marked: Int, at point: @escaping (NSMenu) -> NSPoint, in view: NSView) {
        Task { [weak self] in
            guard let self, let menu = await self.makeMenu(rows: rows), view.window != nil else { return }
            self.lastMenu = menu
            menu.minimumWidth = 248
            // The menu's tracking loop runs from the run loop, not from this task: a nested
            // loop inside a main-actor job would hold every other main-actor job (the core's
            // events, other tasks) until the menu closes.
            MainRunLoop.perform { [weak self] in
                guard let self, view.window != nil else { return }
                self.showMenuRows(IndexSet(integer: marked))
                menu.popUp(positioning: nil, at: point(menu), in: view)
                self.showMenuRows([])
            }
        }
    }

    /// The last menu a row's "⋯" or ⇧F10 opened.
    private(set) weak var lastMenu: NSMenu?

    #if DEBUG
    func debugMenu(row: Int) async -> NSMenu? {
        await makeMenu(rows: rows(for: row))
    }
    #endif

    /// A right click (or ⌃-click): the row's menu at the pointer, opening rightward.
    func openContextMenu(row: Int, at point: NSPoint) {
        popUp(rows: rows(for: row), marked: row, at: { _ in point }, in: table)
    }

    /// The row's "⋯": the menu below it, to its left.
    func openMenu(row: Int, from button: NSView) {
        popUp(rows: rows(for: row), marked: row, at: { menu in
            let width = max(menu.size.width, 248)
            return NSPoint(x: button.bounds.maxX - width, y: button.isFlipped ? button.bounds.maxY + 4 : button.bounds.minY - 4)
        }, in: button)
    }

    /// ⇧F10: the selected row's menu, 48 points in, opening rightward.
    func openMenuForSelection() {
        guard let row = table.selectedRowIndexes.first else { return }
        table.scrollRowToVisible(row)
        let rect = table.rect(ofRow: row)
        popUp(rows: table.selectedRowIndexes, marked: row, at: { _ in NSPoint(x: rect.minX + 48, y: rect.maxY) }, in: table)
    }
}

/// The scroll view, telling its controller when its width changes.
final class TrackScrollView: NSScrollView {
    var onLayout: (() -> Void)?

    override func layout() {
        super.layout()
        onLayout?()
    }
}

/// The table: Return plays, ⇧F10 opens the menu; Space is the app's play/pause.
final class TrackNSTable: NSTableView {
    weak var controller: TrackTableController?

    override func keyDown(with event: NSEvent) {
        let modifiers = event.modifierFlags.intersection([.command, .control, .option, .shift])
        switch event.keyCode {
        case 36 where modifiers.isEmpty, 76 where modifiers.isEmpty:
            controller?.playSelected()
        case 109 where modifiers == .shift:
            controller?.openMenuForSelection()
        default:
            super.keyDown(with: event)
        }
    }

    /// Right click and ⌃-click open the row's menu (built once its tracks are read), without
    /// AppKit's outline: the row shows its menu open with the `selected` ground instead.
    override func menu(for event: NSEvent) -> NSMenu? {
        let point = convert(event.locationInWindow, from: nil)
        let row = row(at: point)
        if row >= 0 { controller?.openContextMenu(row: row, at: point) }
        return nil
    }

    override var acceptsFirstResponder: Bool { true }
}

/// The header row: 36 points on the page's ground with a 1-point line under it. A sortable
/// column's label turns `text` under the pointer.
final class TrackHeaderView: NSTableHeaderView {
    private var hovered = -1 {
        didSet { if oldValue != hovered { needsDisplay = true } }
    }
    private var tracking: NSTrackingArea?

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let tracking { removeTrackingArea(tracking) }
        let area = NSTrackingArea(rect: bounds, options: [.mouseEnteredAndExited, .mouseMoved, .activeInKeyWindow, .inVisibleRect],
                                  owner: self)
        addTrackingArea(area)
        tracking = area
    }

    override func mouseMoved(with event: NSEvent) {
        super.mouseMoved(with: event)
        hovered = column(at: convert(event.locationInWindow, from: nil))
    }

    override func mouseEntered(with event: NSEvent) {
        super.mouseEntered(with: event)
        hovered = column(at: convert(event.locationInWindow, from: nil))
    }

    override func mouseExited(with event: NSEvent) {
        super.mouseExited(with: event)
        hovered = -1
    }

    override func draw(_ dirtyRect: NSRect) {
        NSColor(Theme.bg).setFill()
        bounds.fill()
        NSColor(Theme.line).setFill()
        NSRect(x: 0, y: isFlipped ? bounds.maxY - 1 : bounds.minY, width: bounds.width, height: 1).fill()
        guard let tableView else { return }
        for (index, column) in tableView.tableColumns.enumerated() where !column.isHidden {
            let rect = headerRect(ofColumn: index)
            guard rect.insetBy(dx: -20, dy: 0).intersects(dirtyRect) else { continue }
            if let cell = column.headerCell as? TrackHeaderCell {
                cell.hovered = index == hovered
            }
            column.headerCell.draw(withFrame: rect, in: self)
        }
    }
}

/// A header label as the Qt table's (`qml/TrackHeader.qml`): 12-point semibold capitals in
/// `text3`, `text` for the column sorted by or under the pointer, with a 12-point arrow 4
/// points beside it (on the left of right-aligned ones). No separators: a quiet row of labels
/// over a 1-point line.
final class TrackHeaderCell: NSTableHeaderCell {
    private var alignRight = false
    /// Room before the label (the Format column's badge sits 14 points in).
    var leading: CGFloat = 0
    /// Room after a right-aligned label (14 for "#", as its numbers).
    var trailing: CGFloat = 0
    /// Clicking it sorts.
    var sortable = false
    var hovered = false
    private var text = ""
    /// `nil` when the column is not the sort's first key.
    private var ascending: Bool?

    func configure(text: String, alignRight: Bool, sorted: Bool, ascending: Bool) {
        self.alignRight = alignRight
        self.ascending = sorted ? ascending : nil
        self.text = text.uppercased()
        stringValue = self.text
    }

    override func draw(withFrame cellFrame: NSRect, in controlView: NSView) {
        drawInterior(withFrame: cellFrame, in: controlView)
    }

    override func drawInterior(withFrame cellFrame: NSRect, in controlView: NSView) {
        guard !text.isEmpty else { return }
        let paragraph = NSMutableParagraphStyle()
        paragraph.alignment = alignRight ? .right : .left
        paragraph.lineBreakMode = .byTruncatingTail
        let lit = ascending != nil || (hovered && sortable)
        let label = NSAttributedString(string: text, attributes: [
            .font: NSFont.systemFont(ofSize: 12, weight: .semibold),
            .foregroundColor: NSColor(lit ? Theme.text : Theme.text3),
            .kern: 0.48,
            .paragraphStyle: paragraph,
        ])
        let size = label.size()
        var frame = cellFrame
        frame.origin.y = cellFrame.midY - size.height / 2
        frame.size.height = size.height
        frame.origin.x += leading
        frame.size.width = max(0, cellFrame.width - trailing - leading)
        label.draw(with: frame, options: [.usesLineFragmentOrigin, .truncatesLastVisibleLine])
        // The arrow sits beside the label, into the next column if it must (as Qt's does).
        guard let ascending,
              let image = NSImage(systemSymbolName: ascending ? "arrow.up" : "arrow.down", accessibilityDescription: nil)?
                .withSymbolConfiguration(.init(pointSize: 12, weight: .semibold))
        else { return }
        let textWidth = min(size.width, frame.width)
        let x = alignRight ? cellFrame.maxX - trailing - textWidth - 4 - image.size.width : frame.minX + textWidth + 4
        let rect = NSRect(x: x, y: cellFrame.midY - image.size.height / 2, width: image.size.width, height: image.size.height)
        let tinted = NSImage(size: image.size, flipped: false) { bounds in
            image.draw(in: bounds)
            NSColor(Theme.text).set()
            bounds.fill(using: .sourceAtop)
            return true
        }
        tinted.draw(in: rect, from: .zero, operation: .sourceOver, fraction: 1, respectFlipped: true, hints: nil)
    }

    override func drawSortIndicator(withFrame cellFrame: NSRect, in controlView: NSView, ascending: Bool, priority: Int) {}
}
