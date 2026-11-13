#if DEBUG
import AppKit

/// Debug-only commands for the queue, the panel and the mini player, posted like
/// `DebugHooks`' but named `com.enn3developer.n-music.debug.player`; a command starting with
/// `@<pid>` runs only in that process. `go <page>`, `play-all`, `pause`, `next`, `previous`,
/// `size <w> <h>` mirror `DebugHooks` for that reason; `show` puts the main window on screen
/// behind every other one, `mini` / `mini-off` show and hide a mini player there, without
/// activating the app; `windows` logs the app's windows. Installed once the queue
/// model starts (open the queue page or a wide window first: `go queue`).
///
/// `enqueue <n> [next]` queues the library's first n tracks (after the current one with
/// `next`), `move <from> <to>` drops page row `from` at `to` as a drag would, `drop <from> <to>
/// [next]` the same with the session moving on mid-drag, `move-check [rounds]` checks the Swift
/// move against the core's, `context <label>`
/// sets the "Playing from" label, `play-row <i>` plays page row i, `top` scrolls the page to the
/// current track, `land <row> <lift>` plays a drop's landing, `slow <factor>` stretches the
/// landing and row arrivals, `rows <dir>` writes the page's rows to `<dir>/queue.txt`, `compact <0|1>`.
@MainActor
enum QueueDebug {
    private static var installed = false
    private static var debugMini: MiniPlayerWindow?

    static func install() {
        guard !installed else { return }
        installed = true
        DistributedNotificationCenter.default().addObserver(
            forName: .init("com.enn3developer.n-music.debug.player"), object: nil, queue: .main
        ) { notification in
            let command = notification.object as? String ?? ""
            Task { @MainActor in run(command) }
        }
    }

    private static func run(_ line: String) {
        var parts = line.split(separator: " ").map(String.init)
        // `@<pid> command`: only that instance runs it (several Debug apps may be running).
        if let first = parts.first, first.hasPrefix("@") {
            guard Int32(first.dropFirst()) == ProcessInfo.processInfo.processIdentifier else { return }
            parts.removeFirst()
        }
        let store = Store.shared
        let model = QueueModel.shared
        NSLog("N Music player debug: %@", line)
        switch parts.first {
        case "enqueue":
            let count = parts.count > 1 ? UInt32(parts[1]) ?? 1 : 1
            let next = parts.contains("next")
            Task {
                let tracks = await store.tracks(.library, offset: 20, limit: count)
                store.enqueue(
                    tracks.map(\.locator), next: next,
                    title: tracks.count == 1 ? tracks.first?.title : Format.count(tracks.count, "track_one", "tracks_many")
                )
            }
        case "move":
            if parts.count > 2, let from = Int(parts[1]), let to = Int(parts[2]) {
                model.move(from: from, to: to)
            }
        case "move-check":
            // `move-check [rounds]`: the Swift `queueMove` against the core's `queue_move` on
            // the page's rows with history/current/queued flags laid out at random.
            let rounds = parts.count > 1 ? Int(parts[1]) ?? 200 : 200
            let base = Array((model.page?.rows ?? []).prefix(24))
            guard !base.isEmpty else { NSLog("N Music player debug: move-check: no rows"); return }
            var generator = SystemRandomNumberGenerator()
            var checked = 0
            var differ = 0
            for _ in 0..<rounds {
                var rows = base.shuffled(using: &generator)
                let count = Int.random(in: 1...rows.count, using: &generator)
                rows = Array(rows.prefix(count))
                let current: Int? = Bool.random(using: &generator) ? Int.random(in: 0..<count, using: &generator) : nil
                let history = current ?? (Bool.random(using: &generator) ? Int.random(in: 0...count, using: &generator) : 0)
                for index in rows.indices {
                    rows[index].current = index == current
                    rows[index].history = index < history
                    rows[index].entry.queued = !rows[index].history && Bool.random(using: &generator)
                }
                for from in 0..<(count + 1) {
                    for to in 0..<(count + 2) {
                        let swift = QueueModel.queueMove(rows, from: from, to: to)
                        let core = queueMove(rows: rows, from: UInt32(from), to: UInt32(to))
                        checked += 1
                        let same = swift?.item == core?.item && swift?.before == core?.before
                            && swift?.current == core?.current && swift?.rows == core?.rows
                        if !same {
                            differ += 1
                            if differ <= 5 {
                                let flags = rows.map { ($0.history ? "H" : "-") + ($0.current ? "C" : "-") + ($0.entry.queued ? "Q" : "-") }
                                NSLog("N Music player debug: move-check differs: %@ from %d to %d", flags.joined(separator: " "), from, to)
                            }
                        }
                    }
                }
            }
            NSLog("N Music player debug: move-check: %d moves, %d differ", checked, differ)
        case "drop":
            // `drop <from> <to> [next]`: a drag of page row `from` let go at `to`, the session
            // going to the next track in between with `next` (the drop then finds the item again,
            // or cancels when it changed role).
            guard parts.count > 2, let from = Int(parts[1]), let to = Int(parts[2]),
                  let rows = model.page?.rows, rows.indices.contains(from) else { return }
            let drop = QueueModel.drop(rows, from: from, to: to)
            let skip = parts.count > 3 && parts[3] == "next"
            Task { @MainActor in
                if skip {
                    store.next()
                    try? await Task.sleep(for: .milliseconds(1200))
                }
                let moved = model.move(drop)
                NSLog("N Music player debug: drop %@ (current %d, after %@, before %@): %@",
                      "\(drop.item)", drop.current ? 1 : 0, "\(drop.after.map { "\($0)" } ?? "-")",
                      "\(drop.before.map { "\($0)" } ?? "-")", moved.map { "moved \($0)" } ?? "canceled")
            }
        case "context":
            PlayContext.shared.set(.init(label: parts.dropFirst().joined(separator: " "), detail: ["“debug”"], page: PageID.albums))
        case "play-row":
            if parts.count > 1, let index = Int(parts[1]), let rows = model.page?.rows, rows.indices.contains(index) {
                model.play(rows[index].entry.item)
            }
        case "go":
            if parts.count > 1 { Navigation.shared.go(parts[1]) }
        case "shuffle":
            store.setShuffle(parts.count > 1 && parts[1] == "1")
        case "play-all":
            store.play(.library)
        case "pause":
            store.pause()
        case "next":
            store.next()
        case "previous":
            store.previous()
        case "size":
            if parts.count > 2, let width = Double(parts[1]), let height = Double(parts[2]) {
                Windows.shared.main?.setContentSize(NSSize(width: width, height: height))
            }
        case "windows":
            for window in NSApp.windows {
                NSLog("N Music player debug: window %@ %@ visible=%d", window.title, NSStringFromRect(window.frame), window.isVisible ? 1 : 0)
            }
        case "show":
            // A launch in the background may open no window: SwiftUI's reopen handling opens
            // it. Then behind every other window: on screen for a capture, never in the way.
            if Windows.shared.main == nil {
                _ = NSApp.delegate?.applicationShouldHandleReopen?(NSApp, hasVisibleWindows: false)
            }
            Task { @MainActor in
                for _ in 0..<20 {
                    if let main = Windows.shared.main {
                        main.orderBack(nil)
                        return
                    }
                    try? await Task.sleep(for: .milliseconds(50))
                }
            }
        case "mini":
            // The mini player without activating the app (`Windows.showMiniPlayer` activates it).
            let window = debugMini ?? MiniPlayerWindow()
            debugMini = window
            Windows.shared.main?.orderOut(nil)
            window.orderBack(nil)
        case "mini-off":
            // The main window first: with no window ordered in, the app quits.
            Windows.shared.main?.orderBack(nil)
            debugMini?.orderOut(nil)
        case "land":
            // `land <row> <lift>`: the drop's landing from `lift` points away.
            if parts.count > 2, let index = Int(parts[1]), let lift = Double(parts[2]) {
                model.pageTable?.debugLand(index, lift: lift)
            }
        case "slow":
            QueueTableController.slow = parts.count > 1 ? Double(parts[1]) ?? 1 : 1
        case "top":
            model.pageTable?.scrollToCurrent()
        case "compact":
            UISettings.shared.compactRows = parts.count > 1 && parts[1] == "1"
        case "rows":
            let directory = parts.count > 1 ? parts[1] : NSTemporaryDirectory()
            let page = model.page
            var lines = [
                "history=\(page?.history ?? 0) upNext=\(page?.upNext ?? 0) left=\(page?.left ?? 0) currentRow=\(page.flatMap(\.currentRow).map(String.init) ?? "-")",
                "context=\(PlayContext.shared.label) | \(PlayContext.shared.detail) | \(PlayContext.shared.page)",
                "panel=\(model.panel?.rows.count ?? 0) facts=\(CurrentFacts.shared.audio) / \(CurrentFacts.shared.gain) / \(CurrentFacts.shared.plays)",
            ]
            for (index, row) in (page?.rows ?? []).prefix(40).enumerated() {
                let flags = (row.history ? "H" : "-") + (row.current ? "C" : "-") + (row.entry.queued ? "Q" : "-")
                lines.append("\(index) \(flags) \(row.entry.item) \(row.entry.track.title)")
            }
            let url = URL(fileURLWithPath: directory).appendingPathComponent("queue.txt")
            try? lines.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
        default:
            NSLog("N Music player debug: unknown command %@", line)
        }
    }
}
#endif
