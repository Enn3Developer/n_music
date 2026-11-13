import AppKit
import QuartzCore
import SwiftUI

/// The queue page's list and the Now Playing panel's Up next list: an NSTableView (virtualized,
/// reused rows) whose rows are SwiftUI (`QueueRowContent`).
///
/// - Double-click plays the row's item from its start, even when paused.
/// - On the page, the current and upcoming rows drag to another place with the mouse: rows
///   make room as it passes (the table's gap feedback), the list scrolls near its edges, Escape
///   puts it back; the drop goes through `queueMove` (`QueueModel.move`) and the row lands in
///   its place from where it was let go.
/// - Changes animate as Qt's RowTransitions (`apply`); the page's playing ground glides between
///   rows (`PlayingHighlight`).
/// - No selection, no context menu (as Qt).
struct QueueTable: NSViewRepresentable {
    let mode: QueueTableController.Mode
    let rows: [QueueLayoutRow]
    var compact = false

    func makeCoordinator() -> QueueTableController {
        QueueTableController(mode: mode)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let controller = context.coordinator
        switch mode {
        case .page: QueueModel.shared.pageTable = controller
        case .panel: QueueModel.shared.panelTable = controller
        }
        return controller.scrollView
    }

    func updateNSView(_ nsView: NSScrollView, context: Context) {
        context.coordinator.update(rows: rows, compact: compact)
    }
}

@MainActor
final class QueueTableController: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    enum Mode { case page, panel }

    let mode: Mode
    let scrollView = NSScrollView()
    let table = QueueNSTableView()

    private(set) var rows: [QueueLayoutRow] = []
    private var compact = UISettings.shared.compactRows
    /// Items queued a moment ago, tinted for a moment as they come in.
    private var tinted: [ItemId: ContinuousClock.Instant] = [:]
    /// The row index being dragged (of `rows`).
    private var dragged: Int?
    /// The item being dragged and whether it was the current one, as it was picked up: the
    /// drop finds it again in the session as it is then.
    private var picked: (item: ItemId, current: Bool)?
    /// The drag image's height, for the landing.
    private var dragHeight: CGFloat = 0
    /// Whether the rows came at least once (the page scrolls to the current one then).
    private(set) var placed = false
    /// Rows waiting for the list to fade out (a change too big to show row by row).
    private var pending: [QueueLayoutRow]?
    /// The landed row looks picked up until half way to its place.
    private var landing: ItemId?

    /// The page's playing ground (Qt's PlayingHighlight): two layers, the front one on the
    /// playing row, the other fading out where it was.
    private var front: HighlightView?
    private var back: HighlightView?
    private var highlightRow: Int?
    /// Track changes: when the last two came, and whether one came since the ground last moved.
    private var lastTrackChange = Store.shared.trackChange
    private var changedAt: ContinuousClock.Instant?
    private var changedBefore: ContinuousClock.Instant?
    private var armed = false

    private static let dragType = NSPasteboard.PasteboardType("com.enn3developer.n-music.queue-row")

    init(mode: Mode) {
        self.mode = mode
        super.init()
        let column = NSTableColumn(identifier: .init("row"))
        column.resizingMask = .autoresizingMask
        table.addTableColumn(column)
        table.headerView = nil
        table.style = .plain
        table.backgroundColor = .clear
        table.selectionHighlightStyle = .none
        table.intercellSpacing = .zero
        table.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
        table.gridStyleMask = []
        table.focusRingType = .none
        table.allowsEmptySelection = true
        table.allowsMultipleSelection = false
        table.usesAutomaticRowHeights = false
        table.rowHeight = rowHeight
        table.dataSource = self
        table.delegate = self
        table.target = self
        table.doubleAction = #selector(doubleClicked)
        table.wantsLayer = true
        table.setAccessibilityLabel(tr(mode == .page ? "queue" : "up_next"))
        if mode == .page {
            table.registerForDraggedTypes([Self.dragType])
            table.draggingDestinationFeedbackStyle = .gap
            table.setDraggingSourceOperationMask(.move, forLocal: true)
            table.setDraggingSourceOperationMask([], forLocal: false)
            let first = HighlightView(front: true)
            let second = HighlightView(front: false)
            for view in [second, first] {
                view.alphaValue = 0
                table.addSubview(view, positioned: .below, relativeTo: nil)
            }
            front = first
            back = second
        }
        scrollView.documentView = table
        scrollView.drawsBackground = false
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.automaticallyAdjustsContentInsets = false
        scrollView.contentInsets = NSEdgeInsets(top: 0, left: 0, bottom: mode == .page ? 24 : 16, right: 0)
        scrollView.scrollerInsets = NSEdgeInsets(top: 0, left: 0, bottom: mode == .page ? -24 : -16, right: 0)
    }

    /// The row's height with the room between rows (6 on the page, 2 in the panel).
    private var rowHeight: CGFloat {
        switch mode {
        case .page: (compact ? 42 : 54) + 6
        case .panel: (compact ? 42 : 52) + 2
        }
    }

    /// Whether the list is on screen: its page shown, its window visible. Hidden, it changes
    /// without animating.
    private var shown: Bool {
        guard let window = table.window, window.isVisible, !window.isMiniaturized else { return false }
        switch mode {
        case .page: return Navigation.shared.section == PageID.queue
        case .panel: return Navigation.shared.nowPlayingPanelShown
        }
    }

    // MARK: Updates

    func update(rows newRows: [QueueLayoutRow], compact newCompact: Bool) {
        noteTrackChange()
        if newCompact != compact {
            compact = newCompact
            table.rowHeight = rowHeight
            if pending == nil {
                rows = newRows
                table.reloadData()
                placeHighlight()
            }
            return
        }
        // Mid-drag the rows stay as they are; the drag's end lays them out again.
        guard dragged == nil else { return }
        apply(newRows)
        if !placed, !rows.isEmpty {
            placed = true
            if mode == .page { scrollToCurrent() }
        }
    }

    private func noteTrackChange() {
        let change = Store.shared.trackChange
        guard change != lastTrackChange else { return }
        lastTrackChange = change
        changedBefore = changedAt
        changedAt = .now
        armed = true
    }

    /// A track change came within the last `window`.
    private func trackChanged(within window: Duration) -> Bool {
        changedAt.map { ContinuousClock.now - $0 < window } ?? false
    }

    /// Goes to `newRows` as Qt's queue list does: a new list, a reshuffle or rows sliding far
    /// fade the list out over 80 ms, then the rows come in a row after another (15 ms apart);
    /// a change of a few rows comes as removals (fading out in 80 ms; in Up next the first row
    /// slides up as it leaves on a skip), moves (rows out of order by one place slide over
    /// 200 ms) and insertions (new rows fade in over 150 ms after 80 ms; rows coming back from
    /// farther fade in over 200 ms from 4 points up, 180 ms in). Off screen, or under reduced
    /// motion, they change at once.
    private func apply(_ newRows: [QueueLayoutRow]) {
        if pending != nil {
            pending = newRows
            return
        }
        let old = rows
        if old.count == newRows.count, zip(old, newRows).allSatisfy({ $0.entry.item == $1.entry.item }) {
            rows = newRows
            refreshVisible()
            placeHighlight()
            return
        }
        guard shown, !old.isEmpty, !Motion.shared.reduced else {
            let populates = old.isEmpty && shown && !Motion.shared.reduced
            rows = newRows
            table.reloadData()
            placeHighlight()
            if populates { populate() }
            return
        }
        let plan = QueuePlan(old: old, new: newRows)
        if plan.overhauls {
            fadeThrough(newRows)
        } else {
            step(to: newRows, plan: plan)
        }
    }

    /// Removals, moves and insertions, as Qt's `step_to`.
    private func step(to newRows: [QueueLayoutRow], plan: QueuePlan) {
        let old = rows.map(\.entry.item)
        let new = newRows.map(\.entry.item)
        let had = Set(old)
        let wanted = Set(new)
        let now = ContinuousClock.now
        for row in newRows where row.entry.queued && !had.contains(row.entry.item) {
            tinted[row.entry.item] = now
        }
        let skipping = mode == .panel && trackChanged(within: .milliseconds(600))
        rows = newRows
        var work = old
        var inserted: [Int] = []
        NSAnimationContext.runAnimationGroup { context in
            context.duration = Motion.move
            context.timingFunction = CAMediaTimingFunction(controlPoints: 0.2, 0, 0, 1)
            context.allowsImplicitAnimation = true
            table.beginUpdates()
            // Out, from the end a run at a time: rows gone, and those coming back elsewhere.
            let leaves = { (item: ItemId) in !wanted.contains(item) || plan.far.contains(item) }
            var index = work.count
            while index > 0 {
                index -= 1
                guard leaves(work[index]) else { continue }
                let last = index
                while index > 0, leaves(work[index - 1]) { index -= 1 }
                let effect: NSTableView.AnimationOptions = skipping && index == 0 ? [.effectFade, .slideUp] : .effectFade
                table.removeRows(at: IndexSet(index...last), withAnimation: effect)
                work.removeSubrange(index...last)
            }
            // Into the new order, a row at a time.
            let left = Set(work)
            let order = new.filter { left.contains($0) }
            for (to, item) in order.enumerated() where work[to] != item {
                if let from = (to + 1..<work.count).first(where: { work[$0] == item }) {
                    table.moveRow(at: from, to: to)
                    work.remove(at: from)
                    work.insert(item, at: to)
                }
            }
            // In, a run at a time: new rows, and those coming back.
            var to = 0
            while to < new.count {
                if to < work.count, work[to] == new[to] {
                    to += 1
                    continue
                }
                let next = to < work.count ? work[to] : nil
                var last = to
                while last + 1 < new.count, new[last + 1] != next { last += 1 }
                table.insertRows(at: IndexSet(to...last), withAnimation: [])
                work.insert(contentsOf: new[to...last], at: to)
                inserted.append(contentsOf: to...last)
                to = last + 1
            }
            table.endUpdates()
            placeHighlight(animated: true)
        }
        for index in inserted {
            guard let cell = table.rowView(atRow: index, makeIfNecessary: false) else { continue }
            if plan.far.contains(new[index]) {
                arrive(cell, delay: 0.18, duration: Motion.move, rise: 4)
            } else {
                arrive(cell, delay: Motion.exit, duration: Motion.fade, rise: 0)
            }
        }
        refreshVisible()
    }

    /// The list fades out in 80 ms and comes back with its new rows, a row after another.
    private func fadeThrough(_ newRows: [QueueLayoutRow]) {
        pending = newRows
        NSAnimationContext.runAnimationGroup { context in
            context.duration = Motion.exit
            table.animator().alphaValue = 0
        } completionHandler: { [weak self] in
            MainActor.assumeIsolated {
                guard let self, let newRows = self.pending else { return }
                self.pending = nil
                self.rows = newRows
                self.table.reloadData()
                self.table.alphaValue = 1
                self.swapped()
                self.placeHighlight()
                self.populate()
            }
        }
    }

    /// A new list shows from the current track on the page, from its start in the panel.
    private func swapped() {
        switch mode {
        case .page: scroll(toTop: Int(QueueModel.shared.page?.history ?? 0))
        case .panel: scroll(toTop: 0)
        }
    }

    /// The rows on screen come in a row after another, 15 ms apart from the first one the list
    /// shows (the current track on the page), fading in over 150 ms from 4 points up.
    private func populate() {
        table.layoutSubtreeIfNeeded()
        let visible = table.rows(in: table.visibleRect)
        guard visible.length > 0 else { return }
        let first = mode == .page ? Int(QueueModel.shared.page?.history ?? 0) : 0
        for index in visible.location..<(visible.location + visible.length) {
            guard let cell = table.rowView(atRow: index, makeIfNecessary: false) else { continue }
            let delay = Double(min(max(0, index - first) * 15, 75)) / 1000
            arrive(cell, delay: delay, duration: Motion.fade, rise: 4)
        }
        if let front, front.alphaValue > 0 {
            arrive(front, delay: 0, duration: Motion.fade, rise: 0)
        }
    }

    /// Fades `view` in over `duration` after `delay`, from `rise` points above its place.
    private func arrive(_ view: NSView, delay: Double, duration: Double, rise: CGFloat) {
        view.wantsLayer = true
        guard let layer = view.layer else { return }
        let start = CACurrentMediaTime() + delay * Self.slow
        let duration = duration * Self.slow
        let curve = CAMediaTimingFunction(controlPoints: 0.2, 0, 0, 1)
        let fade = CABasicAnimation(keyPath: "opacity")
        fade.fromValue = 0
        fade.toValue = 1
        fade.beginTime = start
        fade.duration = duration
        fade.fillMode = .backwards
        fade.timingFunction = curve
        layer.add(fade, forKey: "arrive")
        if rise != 0 {
            let move = CABasicAnimation(keyPath: "transform.translation.y")
            move.fromValue = -rise * Self.down(view)
            move.toValue = 0
            move.beginTime = start
            move.duration = duration
            move.fillMode = .backwards
            move.timingFunction = curve
            layer.add(move, forKey: "arrive.rise")
        }
    }

    /// The sign of a downward translation of `view`'s layer in its superlayer: its superview's
    /// layer is flipped when the superview is.
    private static func down(_ view: NSView) -> CGFloat {
        (view.superview?.isFlipped ?? true) ? 1 : -1
    }

    /// Shows the rows' new contents in the visible cells, without rebuilding them.
    private func refreshVisible() {
        let visible = table.rows(in: table.visibleRect)
        guard visible.length > 0 else { return }
        for index in visible.location..<(visible.location + visible.length) {
            guard let cell = table.view(atColumn: 0, row: index, makeIfNecessary: false) as? RowCell,
                  let content = content(at: index)
            else { continue }
            cell.host.rootView = content
            cell.draggable = draggable(index)
        }
    }

    private func content(at index: Int) -> QueueRowContent? {
        guard rows.indices.contains(index) else { return nil }
        let row = rows[index]
        var tint = false
        if let at = tinted[row.entry.item] {
            if ContinuousClock.now - at < .milliseconds(600) {
                tint = true
            } else {
                tinted[row.entry.item] = nil
            }
        }
        return QueueRowContent(
            mode: mode, row: row, compact: compact, tint: tint, lifted: landing == row.entry.item,
            ownGround: dragged != nil, onRemove: { Store.shared.removeQueued(row.entry.item) }
        )
    }

    private func draggable(_ index: Int) -> Bool {
        mode == .page && rows.indices.contains(index) && !rows[index].history
    }

    // MARK: The playing ground

    /// The ground's place on row `index`: the row's ground, inside the room between rows.
    private func highlightFrame(_ index: Int) -> NSRect {
        table.rect(ofRow: index).insetBy(dx: 0, dy: 3)
    }

    /// Puts the playing ground on the current row. After a track change it glides there over
    /// 200 ms when six rows away or fewer, else (or under reduced motion) it fades out at the
    /// old row and in at the new one over 150 ms; a change within 250 ms of the one before, or
    /// rows changing, take it there at once (`animated`: along with the rows' own animation).
    private func placeHighlight(animated: Bool = false) {
        guard mode == .page, let front, let back else { return }
        let to = rows.firstIndex { $0.current }
        let from = highlightRow
        front.playing = true
        back.playing = false
        guard let to else {
            highlightRow = nil
            front.alphaValue = 0
            back.alphaValue = 0
            return
        }
        let frame = highlightFrame(to)
        let followsTrack = armed && trackChanged(within: .seconds(1))
        if to != from { armed = false }
        let quick = followsTrack && {
            guard let changedAt, let changedBefore else { return false }
            return changedAt - changedBefore < .milliseconds(250)
        }()
        highlightRow = to
        if from == nil || front.alphaValue == 0 {
            front.frame = frame
            if quick || !shown {
                front.alphaValue = 1
            } else {
                fade(front, to: 1)
            }
            return
        }
        guard to != from, followsTrack, !quick, shown else {
            if animated, shown {
                front.animator().frame = frame
            } else {
                front.frame = frame
            }
            front.alphaValue = 1
            back.alphaValue = 0
            return
        }
        if Motion.shared.reduced || abs(to - (from ?? to)) > 6 {
            // Fades out where it was, in at the new row.
            let old = front
            self.front = back
            self.back = old
            back.frame = frame
            back.playing = true
            old.playing = false
            fade(old, to: 0)
            fade(back, to: 1)
        } else {
            NSAnimationContext.runAnimationGroup { context in
                context.duration = Motion.move
                context.timingFunction = CAMediaTimingFunction(controlPoints: 0.2, 0, 0, 1)
                front.animator().frame = frame
            }
        }
    }

    private func fade(_ view: NSView, to alpha: CGFloat) {
        NSAnimationContext.runAnimationGroup { context in
            context.duration = Motion.fade
            view.animator().alphaValue = alpha
        }
    }

    /// Hides the ground while rows are dragged (the playing row draws its own, so it rides
    /// along as rows make way), and shows it back.
    private func setHighlightHidden(_ hidden: Bool) {
        guard let front else { return }
        if hidden {
            front.alphaValue = 0
            back?.alphaValue = 0
        } else {
            highlightRow = nil
            placeHighlight()
        }
    }

    // MARK: Scrolling

    /// The current track at the top, what played above it out of view (once laid out).
    func scrollToCurrent() {
        Task { @MainActor in
            guard let page = QueueModel.shared.page else { return }
            self.scroll(toTop: Int(page.history))
        }
    }

    /// Scrolls so `index` (of `rows`) is at the top, as far as the list goes.
    func scroll(toTop index: Int) {
        table.layoutSubtreeIfNeeded()
        guard index < table.numberOfRows else { return }
        var y = index <= 0 ? 0 : table.rect(ofRow: index).minY
        let clip = scrollView.contentView
        let maxY = max(0, table.frame.height + scrollView.contentInsets.bottom - clip.bounds.height)
        y = min(y, maxY)
        clip.scroll(to: NSPoint(x: 0, y: y))
        scrollView.reflectScrolledClipView(clip)
    }

    /// Whether row `index` of `rows` shows whole (an empty list counts as showing).
    func rowShows(_ index: Int) -> Bool {
        guard !rows.isEmpty else { return true }
        let rect = table.rect(ofRow: min(index, rows.count - 1))
        let visible = scrollView.contentView.documentVisibleRect
        return rect.minY >= visible.minY && rect.maxY <= visible.maxY
    }

    // MARK: Data source and delegate

    func numberOfRows(in tableView: NSTableView) -> Int {
        rows.count
    }

    func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat {
        rowHeight
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard let content = content(at: row) else { return nil }
        let cell = tableView.makeView(withIdentifier: RowCell.identifier, owner: nil) as? RowCell
            ?? RowCell(content, actionWidth: mode == .page ? 44 : 0)
        cell.host.rootView = content
        cell.draggable = draggable(row)
        return cell
    }

    func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? {
        PlainRowView()
    }

    func tableView(_ tableView: NSTableView, shouldSelectRow row: Int) -> Bool { false }

    func tableView(_ tableView: NSTableView, isGroupRow row: Int) -> Bool { false }

    @objc private func doubleClicked() {
        let index = table.clickedRow
        guard rows.indices.contains(index) else { return }
        QueueModel.shared.play(rows[index].entry.item)
    }

    // MARK: Dragging (page only)

    func tableView(_ tableView: NSTableView, pasteboardWriterForRow row: Int) -> (any NSPasteboardWriting)? {
        guard draggable(row) else { return nil }
        let item = NSPasteboardItem()
        item.setString(String(row), forType: Self.dragType)
        return item
    }

    func tableView(
        _ tableView: NSTableView, draggingSession session: NSDraggingSession,
        willBeginAt screenPoint: NSPoint, forRowIndexes rowIndexes: IndexSet
    ) {
        guard let row = rowIndexes.first, rows.indices.contains(row) else { return }
        dragged = row
        picked = (rows[row].entry.item, rows[row].current)
        landing = nil
        session.animatesToStartingPositionsOnCancelOrFail = true
        // The row lifts: `raised`, a border, 102 %.
        let rect = tableView.rect(ofRow: row)
        let scale = tableView.window?.backingScaleFactor ?? 2
        let pixels = Int(((compact ? 32 : 40) * scale).rounded(.up))
        let entry = rows[row]
        let cover = entry.entry.track.cover.flatMap { Artwork.shared.cached(path: $0, pixels: pixels) }
        let lifted = QueueRowContent(
            mode: mode, row: entry, compact: compact, tint: false, lifted: true, liftedCover: cover,
            onRemove: {}
        )
        .frame(width: rect.width, height: rect.height)
        let renderer = ImageRenderer(content: lifted)
        renderer.scale = scale
        if let image = renderer.nsImage {
            let grow = Motion.shared.reduced ? 0 : 0.02
            let frame = rect.insetBy(dx: -rect.width * grow / 2, dy: -rect.height * grow / 2)
            dragHeight = frame.height
            session.enumerateDraggingItems(
                options: [], for: tableView, classes: [NSPasteboardItem.self], searchOptions: [:]
            ) { item, _, _ in
                item.setDraggingFrame(frame, contents: image)
            }
        }
        // The rows making way carry the playing ground themselves.
        setHighlightHidden(true)
        refreshVisible()
        tableView.hideRows(at: IndexSet(integer: row), withAnimation: [])
    }

    func tableView(
        _ tableView: NSTableView, draggingSession session: NSDraggingSession,
        endedAt screenPoint: NSPoint, operation: NSDragOperation
    ) {
        guard dragged != nil else { return }
        // Canceled, or dropped elsewhere: back as it was, and what came in during the drag.
        dragged = nil
        picked = nil
        let hidden = tableView.hiddenRowIndexes
        if !hidden.isEmpty {
            tableView.unhideRows(at: hidden, withAnimation: [])
        }
        catchUp()
    }

    /// After a drag that moved nothing: the rows as the session stands now.
    private func catchUp() {
        if let page = QueueModel.shared.page, page.rows != rows {
            rows = page.rows
            table.reloadData()
        }
        refreshVisible()
        setHighlightHidden(false)
    }

    func tableView(
        _ tableView: NSTableView, validateDrop info: any NSDraggingInfo,
        proposedRow row: Int, proposedDropOperation dropOperation: NSTableView.DropOperation
    ) -> NSDragOperation {
        guard info.draggingSource as? NSTableView === tableView, dragged != nil else { return [] }
        if dropOperation == .on {
            tableView.setDropRow(row, dropOperation: .above)
        }
        return .move
    }

    func tableView(
        _ tableView: NSTableView, acceptDrop info: any NSDraggingInfo,
        row: Int, dropOperation: NSTableView.DropOperation
    ) -> Bool {
        guard dragged != nil, let picked else { return false }
        // Where the row was let go: the drag image's top, in the table.
        let imageTop = tableView.convert(info.draggedImageLocation, from: nil).y - dragHeight
        dragged = nil
        self.picked = nil
        tableView.unhideRows(at: tableView.hiddenRowIndexes, withAnimation: [])
        // The place among the rows shown (which stayed as they were during the drag), by the
        // rows around it: the session may have changed since the drag began.
        guard let from = rows.firstIndex(where: { $0.entry.item == picked.item }) else {
            catchUp()
            return false
        }
        let above = max(0, row)
        let place = QueueModel.drop(rows, from: from, to: above > from ? above - 1 : above)
        let drop = QueueModel.Drop(item: picked.item, current: picked.current, after: place.after, before: place.before)
        guard let item = QueueModel.shared.move(drop), let page = QueueModel.shared.page else {
            catchUp()
            return false
        }
        rows = page.rows
        tableView.reloadData()
        setHighlightHidden(false)
        if let index = rows.firstIndex(where: { $0.entry.item == item }) {
            // The image stood 1 % of its height above the row's top (it was 102 %).
            land(index, from: imageTop + dragHeight * 0.01 - tableView.rect(ofRow: index).minY)
        }
        return true
    }

    /// The dropped row eases into its place from where it was let go over 200 ms (Qt's
    /// `landing` curve), looking picked up until half way; under reduced motion it snaps there.
    private func land(_ index: Int, from lift: CGFloat) {
        guard !Motion.shared.reduced, abs(lift) > 0.5 else { return }
        table.layoutSubtreeIfNeeded()
        guard let cell = table.view(atColumn: 0, row: index, makeIfNecessary: false) as? RowCell,
              let rowView = table.rowView(atRow: index, makeIfNecessary: false)
        else { return }
        let item = rows[index].entry.item
        landing = item
        if let content = content(at: index) { cell.host.rootView = content }
        // The playing ground stays off the row until it has landed.
        let current = rows[index].current
        if current { front?.alphaValue = 0 }
        // The row view (rows clip their cells), over the rows it passes.
        rowView.wantsLayer = true
        rowView.layer?.zPosition = 10
        let move = CABasicAnimation(keyPath: "transform.translation.y")
        move.fromValue = lift * Self.down(rowView)
        move.toValue = 0
        move.duration = Motion.move * Self.slow
        move.timingFunction = CAMediaTimingFunction(controlPoints: 0.3, 0.7, 0.4, 1)
        rowView.layer?.add(move, forKey: "land")
        Task { @MainActor [weak self] in
            try? await Task.sleep(for: .seconds(Motion.move / 2 * Self.slow))
            guard let self, self.landing == item else { return }
            self.landing = nil
            self.refreshVisible()
            try? await Task.sleep(for: .seconds(Motion.move / 2 * Self.slow))
            rowView.layer?.zPosition = 0
            if current, self.dragged == nil {
                self.highlightRow = nil
                self.placeHighlight()
            }
        }
    }

    /// Stretches the landing and the rows' arrivals (`QueueDebug`'s `slow`), to look at them.
    #if DEBUG
    static var slow = 1.0
    #else
    static let slow = 1.0
    #endif

    #if DEBUG
    /// Plays the landing of row `index` from `lift` points away (`QueueDebug`'s `land`).
    func debugLand(_ index: Int, lift: CGFloat) {
        guard rows.indices.contains(index) else { return }
        land(index, from: lift)
    }
    #endif
}

/// The page's playing ground with the bars where the row's action goes; it lets the mouse
/// through to the rows above it.
private final class HighlightView: NSHostingView<PlayingGround> {
    var playing: Bool {
        get { rootView.front }
        set { if rootView.front != newValue { rootView = PlayingGround(front: newValue) } }
    }

    init(front: Bool) {
        super.init(rootView: PlayingGround(front: front))
        sizingOptions = []
        translatesAutoresizingMaskIntoConstraints = true
        autoresizingMask = [.width]
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError() }

    required init(rootView: PlayingGround) {
        super.init(rootView: rootView)
    }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }
}

/// Qt's PlayingHighlight layer: `selected`, radius 8, the bars 8 + 18 points from its end
/// (playing only on the front layer, and only while the queue page shows).
struct PlayingGround: View {
    let front: Bool
    private let store = Store.shared
    private let navigation = Navigation.shared

    var body: some View {
        ZStack(alignment: .trailing) {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(Theme.selected)
            EqualizerBars(
                playing: front && store.playing, size: 14, color: Theme.accentText,
                visible: navigation.section == PageID.queue
            )
            .frame(width: 36)
            .padding(.trailing, 8)
        }
        .accessibilityHidden(true)
    }
}

/// No selection or group look: the SwiftUI row draws its own ground.
private final class PlainRowView: NSTableRowView {
    override func drawSelection(in dirtyRect: NSRect) {}
    override func drawBackground(in dirtyRect: NSRect) {}
    override var isEmphasized: Bool {
        get { false }
        set {}
    }
}

/// The table: Escape cancels a drag (AppKit does), it never takes the first responder (the
/// page's Space plays and pauses), and the pointer is a closed hand while it carries a row.
final class QueueNSTableView: NSTableView {
    override var acceptsFirstResponder: Bool { false }

    override func draggingSession(_ session: NSDraggingSession, willBeginAt screenPoint: NSPoint) {
        super.draggingSession(session, willBeginAt: screenPoint)
        NSCursor.closedHand.set()
    }

    override func draggingSession(_ session: NSDraggingSession, movedTo screenPoint: NSPoint) {
        super.draggingSession(session, movedTo: screenPoint)
        NSCursor.closedHand.set()
    }

    override func draggingSession(_ session: NSDraggingSession, endedAt screenPoint: NSPoint, operation: NSDragOperation) {
        super.draggingSession(session, endedAt: screenPoint, operation: operation)
        NSCursor.arrow.set()
        window?.invalidateCursorRects(for: self)
    }
}

/// A row: its SwiftUI contents, which take the mouse only in the action slot (the "×"), so a
/// press anywhere else reaches the table (double-click, drag). Over the grip of a row that
/// drags, the pointer is an open hand.
private final class RowCell: NSView {
    static let identifier = NSUserInterfaceItemIdentifier("queue-row")
    let host: NSHostingView<QueueRowContent>
    private let actionWidth: CGFloat
    var draggable = false {
        didSet { if draggable != oldValue { window?.invalidateCursorRects(for: self) } }
    }

    init(_ content: QueueRowContent, actionWidth: CGFloat) {
        host = NSHostingView(rootView: content)
        self.actionWidth = actionWidth
        super.init(frame: .zero)
        identifier = Self.identifier
        wantsLayer = true
        host.sizingOptions = []
        host.translatesAutoresizingMaskIntoConstraints = true
        host.autoresizingMask = [.width, .height]
        addSubview(host)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError() }

    override func layout() {
        super.layout()
        host.frame = bounds
    }

    override func resetCursorRects() {
        super.resetCursorRects()
        // The grip: 4 points in, 16 wide.
        if draggable { addCursorRect(NSRect(x: 0, y: 0, width: 24, height: bounds.height), cursor: .openHand) }
    }

    override func hitTest(_ point: NSPoint) -> NSView? {
        let local = convert(point, from: superview)
        guard bounds.contains(local), local.x >= bounds.maxX - actionWidth else { return nil }
        return super.hitTest(point)
    }
}

/// How the rows shown go to new ones (Qt's `Plan`, `queue.rs`): the items both lists hold,
/// those out of order by more than one place (they leave and come back), and the most places
/// any other kept row slides. A new list, more than three rows going far or rows sliding more
/// than three places is too much to show row by row. O(n log n): a longest increasing run.
struct QueuePlan {
    let kept: Int
    let far: Set<ItemId>
    let slide: Int

    var overhauls: Bool { kept == 0 || far.count > 3 || slide > 3 }

    init(old: [QueueLayoutRow], new: [QueueLayoutRow]) {
        var place: [ItemId: Int] = [:]
        place.reserveCapacity(new.count)
        for (index, row) in new.enumerated() { place[row.entry.item] = index }
        // Where each kept item stood and stands.
        var kept: [(from: Int, to: Int)] = []
        kept.reserveCapacity(min(old.count, new.count))
        for (index, row) in old.enumerated() {
            if let to = place[row.entry.item] { kept.append((index, to)) }
        }
        let staying = Set(Self.longestIncreasing(kept.map(\.to)))
        // Each kept item's rank among the kept ones in the new order.
        let byNew = kept.indices.sorted { kept[$0].to < kept[$1].to }
        var rank = [Int](repeating: 0, count: kept.count)
        for (newRank, index) in byNew.enumerated() { rank[index] = newRank }
        var far = Set<ItemId>()
        var slide = 0
        for (index, pair) in kept.enumerated() {
            if !staying.contains(index), abs(index - rank[index]) > 1 {
                far.insert(old[pair.from].entry.item)
            } else {
                slide = max(slide, abs(pair.from - pair.to))
            }
        }
        self.kept = kept.count
        self.far = far
        self.slide = slide
    }

    /// The places in `values` of a longest increasing run of them, in order.
    static func longestIncreasing(_ values: [Int]) -> [Int] {
        var tails: [Int] = []
        var before = [Int](repeating: -1, count: values.count)
        for (index, value) in values.enumerated() {
            var low = 0
            var high = tails.count
            while low < high {
                let middle = (low + high) / 2
                if values[tails[middle]] < value { low = middle + 1 } else { high = middle }
            }
            if low > 0 { before[index] = tails[low - 1] }
            if low == tails.count { tails.append(index) } else { tails[low] = index }
        }
        var run: [Int] = []
        var at = tails.last ?? -1
        while at >= 0 {
            run.append(at)
            at = before[at]
        }
        return run.reversed()
    }
}
