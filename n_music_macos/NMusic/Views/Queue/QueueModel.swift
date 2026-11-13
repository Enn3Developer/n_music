import Foundation
import Observation

/// The play session laid out for the queue page (what played, the current item, what is left)
/// and for the Now Playing panel (what is left), by the core's `queueLayout`, off the main
/// thread. One for the app: `QueueModel.shared`, started by the first view that shows it.
///
/// After a drop the page shows the rows `queueMove` predicted until the session reports the
/// move, so the dropped row stays where it landed.
@MainActor
@Observable
final class QueueModel {
    static let shared = QueueModel()

    /// History, current, upcoming; `nil` until the first layout.
    private(set) var page: QueueLayout?
    /// Upcoming only.
    private(set) var panel: QueueLayout?

    /// The tables showing the layouts, for `upNextShows`.
    @ObservationIgnored weak var pageTable: QueueTableController?
    @ObservationIgnored weak var panelTable: QueueTableController?

    @ObservationIgnored private let store = Store.shared
    @ObservationIgnored private var started = false
    @ObservationIgnored private var generation = 0

    private init() {}

    func start() {
        guard !started else { return }
        started = true
        #if DEBUG
        QueueDebug.install()
        #endif
        store.observers.append { [weak self] event in
            switch event {
            case .queueChanged, .trackChanged: self?.relayout()
            default: break
            }
        }
        // The queued toast shows only when the queued track's place in Up next is out of sight.
        Navigation.shared.upNextShows = { next in
            QueueModel.shared.placeShows(next: next)
        }
        relayout()
    }

    private func relayout() {
        generation += 1
        let serial = generation
        let entries = store.queue
        let current = store.current?.item
        Task {
            let layouts = await Task.detached(priority: .userInitiated) {
                (
                    queueLayout(entries: entries, current: current, showHistory: true, showCurrent: true),
                    queueLayout(entries: entries, current: current, showHistory: false, showCurrent: false)
                )
            }.value
            guard serial == generation else { return }
            if page != layouts.0 { page = layouts.0 }
            if panel != layouts.1 { panel = layouts.1 }
        }
    }

    // MARK: Moves

    /// What a drag holds: the item picked up, its role then, and its neighbours where it was
    /// let go, all in the rows the table showed (which stay as they are during the drag while
    /// the session may change under them).
    struct Drop {
        /// The item dragged.
        let item: ItemId
        /// It was the current item when picked up (else one still to play).
        let current: Bool
        /// The row above where it was let go (the dragged one left out); `nil` at the top.
        let after: ItemId?
        /// The row below where it was let go; `nil` at the bottom.
        let before: ItemId?
    }

    /// Drops the page row at `from` at `to` (both indices of `page.rows`, `to` counted after
    /// taking the row out). Returns the item moved, `nil` when nothing moved.
    @discardableResult
    func move(from: Int, to: Int) -> ItemId? {
        guard let rows = page?.rows, rows.indices.contains(from), to >= 0 else { return nil }
        return move(Self.drop(rows, from: from, to: to))
    }

    /// The drop of the row at `from` of `rows` at `to` (counted after taking it out).
    static func drop(_ rows: [QueueLayoutRow], from: Int, to: Int) -> Drop {
        var rest = rows
        let row = rest.remove(at: from)
        let to = min(to, rest.count)
        return Drop(
            item: row.entry.item,
            current: row.current,
            after: to > 0 ? rest[to - 1].entry.item : nil,
            before: to < rest.count ? rest[to].entry.item : nil
        )
    }

    /// Carries out `drop` on the session as it is now: the item where it is now, its place
    /// found again by its neighbours. Nothing moves (`nil`) when the item left the session, or
    /// took another role during the drag (it became the current one, or played), or when
    /// neither neighbour is still there.
    @discardableResult
    func move(_ drop: Drop) -> ItemId? {
        guard let rows = page?.rows,
              let from = rows.firstIndex(where: { $0.entry.item == drop.item })
        else { return nil }
        let row = rows[from]
        guard !row.history, row.current == drop.current else { return nil }
        // The place among the rows without the dragged one.
        let index = { (item: ItemId) -> Int? in
            rows.firstIndex { $0.entry.item == item }.map { $0 > from ? $0 - 1 : $0 }
        }
        let to: Int
        if let before = drop.before, let place = index(before) {
            to = place
        } else if let after = drop.after, let place = index(after) {
            to = place + 1
        } else {
            return nil
        }
        guard let move = Self.queueMove(rows, from: from, to: to) else { return nil }
        // The session reports the move in a moment; until then, the rows as they will stand.
        generation += 1
        self.page = Self.layout(move.rows)
        if move.current {
            store.moveCurrent(move.item, before: move.before)
        } else {
            store.moveUpcoming(move.item, before: move.before)
        }
        return move.item
    }

    /// What a drop does (the core's `queue_move`, `n_music_ffi/src/queue.rs`, ported to leave
    /// the rows on this side: crossing the FFI with a whole session's rows on the main thread
    /// costs far more than the move).
    struct Move {
        /// The item moved.
        let item: ItemId
        /// What it now plays before; `nil` after everything.
        let before: ItemId?
        /// It is the current item: `MoveCurrent`, else `MoveUpcoming`.
        let current: Bool
        /// The rows once it landed, until the session reports the move.
        let rows: [QueueLayoutRow]
    }

    /// Drops the item at `from` of `rows`, as `queueLayout` listed them, at `to`. Only the
    /// current item and those still to play move; one still to play goes among those played too,
    /// above the current item, and then counts as played and is no longer queued. The current
    /// item moved down makes the context's items it passes played, while the queued ones it passes
    /// play right after it. `nil` when nothing moves.
    nonisolated static func queueMove(_ input: [QueueLayoutRow], from: Int, to requested: Int) -> Move? {
        guard input.indices.contains(from), requested >= 0, !input[from].history else { return nil }
        var rows = input
        let dragged = rows[from]
        let (item, current, queued) = (dragged.entry.item, dragged.current, dragged.entry.queued)
        // The rows played before the current item, the first ones.
        let played = rows.prefix { $0.history }.count
        // Those still to play go among those played too, above the current item.
        let first = current || rows.contains { $0.current }
            ? 0
            : rows.firstIndex { !$0.history && !$0.current } ?? from
        let to = max(min(requested, rows.count - 1), first)
        guard to != from else { return nil }
        let row = rows.remove(at: from)
        rows.insert(row, at: to)
        if current {
            for index in min(from, to)...max(from, to) {
                rows[index].history = index < to && (index < played || !rows[index].entry.queued)
            }
        } else {
            let history = rows.firstIndex { $0.current }.map { to < $0 } ?? false
            rows[to].history = history
            rows[to].entry.queued = queued && !history
        }
        let before = to + 1 < rows.count ? rows[to + 1].entry.item : nil
        if current && to > from {
            // The queued items it passed going down play right after it.
            var kept: [QueueLayoutRow] = []
            var passed: [QueueLayoutRow] = []
            kept.reserveCapacity(rows.count)
            for (index, row) in rows.enumerated() {
                if (from..<to).contains(index) && row.entry.queued {
                    passed.append(row)
                } else {
                    kept.append(row)
                }
            }
            let place = kept.firstIndex { $0.entry.item == item }.map { $0 + 1 } ?? kept.count
            kept.insert(contentsOf: passed, at: place)
            rows = kept
        }
        return Move(item: item, before: before, current: current, rows: rows)
    }

    private static func layout(_ rows: [QueueLayoutRow]) -> QueueLayout {
        let history = rows.reduce(0) { $0 + ($1.history ? 1 : 0) }
        let currentRow = rows.firstIndex { $0.current }
        let upcoming = rows.filter { !$0.history && !$0.current }
        return QueueLayout(
            rows: rows,
            history: UInt32(history),
            upNext: UInt32(upcoming.reduce(0) { $0 + ($1.entry.queued ? 1 : 0) }),
            left: UInt32(upcoming.count),
            currentRow: currentRow.map(UInt32.init)
        )
    }

    // MARK: Playing

    /// Plays `item` from its start, even when paused (a row's double-click).
    func play(_ item: ItemId) {
        store.play(item: item, at: 0)
        store.play()
    }

    // MARK: The queued toast

    /// Whether the place of a track queued now shows: right after the current one when it
    /// plays `next`, else after the queued ones; on the queue page or in the panel.
    private func placeShows(next: Bool) -> Bool {
        let navigation = Navigation.shared
        if navigation.section == PageID.queue {
            guard let page, let table = pageTable else { return true }
            let at = Int(page.history) + (page.currentRow != nil ? 1 : 0) + (next ? 0 : Int(page.upNext))
            return table.rowShows(at)
        }
        if navigation.nowPlayingPanelShown {
            guard let panel, let table = panelTable else { return true }
            return table.rowShows(next ? 0 : Int(panel.upNext))
        }
        return false
    }
}
