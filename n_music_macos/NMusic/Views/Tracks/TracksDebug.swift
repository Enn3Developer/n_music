#if DEBUG
import AppKit
import SwiftUI

/// Drives the Tracks page of a Debug build from outside (as `DebugHooks`): post a distributed
/// notification named `com.enn3developer.n-music.debug.tracks` whose object is the command.
///
/// Prefix a command with `@<pid> ` to speak to one app of several. Commands: `window` (orders
/// the main window in front without activating the app), `search <text>`, `sort <sort>`, `filter <editor
/// JSON>`, `drawer`, `close`, `select <row,row…>`, `menu <row> [seconds]` (the right-click menu,
/// closed after the seconds, 3 by default), `keymenu [_ seconds]` (⇧F10's), `more <row>
/// [seconds]` (the row's "⋯" menu), `submenu <row>` (logs the menu's entries), `item <row>
/// <index[.index]>` (runs a menu entry), `playlist <name>` (of the selected rows), `play <row>`, `scroll <row>`, `dense on|off`, `hide <columns>`
/// (comma-separated; empty shows all), `smart` (the New smart playlist dialog).
@MainActor
enum TracksDebug {
    private static var installed = false

    /// Called at launch by `DebugHooks.install()` (and again, harmlessly, as the page shows).
    static func install() {
        guard !installed else { return }
        installed = true
        listen()
    }

    nonisolated private static func listen() {
        DistributedNotificationCenter.default().addObserver(
            forName: .init("com.enn3developer.n-music.debug.tracks"), object: nil, queue: .main
        ) { notification in
            let line = notification.object as? String ?? ""
            Task { @MainActor in TracksDebug.run(line) }
        }
    }

    private static func run(_ line: String) {
        // Several Debug builds may run: `@<pid> command` speaks to one.
        var line = line
        if line.hasPrefix("@") {
            let target = line.dropFirst().prefix { $0 != " " }
            guard Int32(target) == ProcessInfo.processInfo.processIdentifier else { return }
            line = String(line.dropFirst(target.count + 2))
        }
        NSLog("N Music tracks debug: %@", line)
        let verb = line.split(separator: " ", maxSplits: 1).first.map(String.init) ?? ""
        let argument = line.count > verb.count ? String(line.dropFirst(verb.count + 1)) : ""
        let words = argument.split(separator: " ").map(String.init)
        let list = TracksModel.shared.list
        let seconds = words.count > 1 ? Double(words[1]) ?? 3 : 3
        switch verb {
        case "go": Navigation.shared.go(argument)
        case "size":
            if words.count > 1, let width = Double(words[0]), let height = Double(words[1]) {
                Windows.shared.main?.setContentSize(NSSize(width: width, height: height))
            }
        case "search": TracksDebugState.shared.search = argument
        case "sort": list.sort = argument
        case "filter": list.filter = argument
        case "drawer": Navigation.shared.openFilterDrawer(.tracks)
        case "close": Navigation.shared.filterDrawer = nil
        case "select":
            let rows = IndexSet(argument.split(separator: ",").compactMap { Int($0) })
            table?.selectRowIndexes(rows, byExtendingSelection: false)
            table?.window?.makeFirstResponder(table)
        case "menu":
            // The right-click path: the menu at the row, closed after the seconds.
            guard let table, let row = words.first.flatMap(Int.init) else { return }
            table.scrollRowToVisible(row)
            let rect = table.rect(ofRow: row)
            closeLastMenu(table, after: seconds)
            table.controller?.openContextMenu(row: row, at: NSPoint(x: rect.minX + 120, y: rect.midY))
        case "keymenu":
            // ⇧F10's path: the selection's menu.
            guard let table else { return }
            closeLastMenu(table, after: seconds)
            table.controller?.openMenuForSelection()
        case "submenu":
            // Logs the entries once the playlists holding the tracks are read.
            guard let table, let row = words.first.flatMap(Int.init) else { return }
            Task {
                guard let menu = await table.controller?.debugMenu(row: row) else {
                    NSLog("N Music tracks debug menu: none")
                    return
                }
                try? await Task.sleep(for: .seconds(1))
                let lines = menu.items.map { item in
                    var line = "\(item.isSeparatorItem ? "—" : item.title) enabled=\(item.isEnabled)"
                    if let submenu = item.submenu {
                        line += " [" + submenu.items.map { sub in
                            "\(sub.isSeparatorItem ? "—" : sub.title)\(sub.state == .on ? " ✓" : "")\(sub.isEnabled ? "" : " (disabled)")"
                        }.joined(separator: "; ") + "]"
                    }
                    return line
                }
                NSLog("N Music tracks debug menu: %@", lines.joined(separator: " | "))
            }
        case "more":
            guard let table, let row = words.first.flatMap(Int.init) else { return }
            table.scrollRowToVisible(row)
            let column = table.column(withIdentifier: TrackColumn.more.identifier)
            guard column >= 0, let cell = table.view(atColumn: column, row: row, makeIfNecessary: true) else { return }
            closeLastMenu(table, after: seconds)
            table.controller?.openMenu(row: row, from: cell.subviews.first ?? cell)
        case "item":
            // item <row> <path>: runs the track menu's entry at `path` (like 2.3: the 4th entry
            // of the 3rd entry's submenu), as a click would.
            guard let table, words.count > 1, let row = Int(words[0]) else { return }
            let path = words[1].split(separator: ".").compactMap { Int($0) }
            Task {
                guard let menu = await table.controller?.debugMenu(row: row) else { return }
                var target = menu
                for index in path.dropLast() {
                    guard let submenu = target.item(at: index)?.submenu else { return }
                    target = submenu
                }
                if let last = path.last { target.performActionForItem(at: last) }
            }
        case "responder":
            let responder = Windows.shared.main?.firstResponder
            var chain: [String] = []
            var next: NSResponder? = responder
            while let item = next, chain.count < 6 {
                chain.append(String(describing: type(of: item)))
                next = item.nextResponder
            }
            NSLog("N Music tracks responder: %@ drawer=%@", chain.joined(separator: " > "),
                  String(describing: Navigation.shared.filterDrawer))
        case "key":
            // The app's key window, without activating the app (for keys posted to its pid).
            Windows.shared.main?.makeKey()
            NSLog("N Music tracks key: %@", String(describing: NSApp.keyWindow))
        case "window":
            // Orders the main window in front without activating the app.
            Windows.shared.showMain(activate: false)
        case "playlist":
            // playlist <name>: a regular playlist holding the selected rows.
            let rows = table?.selectedRowIndexes ?? []
            Task {
                let entries = await list.entries(at: rows)
                Store.shared.createPlaylist(name: argument, tracks: entries.map(\.track.locator))
            }
        case "geom":
            guard let table else { return }
            let columns = table.tableColumns.filter { !$0.isHidden }.map { "\($0.identifier.rawValue.dropFirst(6))=\(Int($0.width))" }
            NSLog("N Music tracks geom: table=%@ clip=%@ row0=%@ columns=%@",
                  NSStringFromRect(table.frame), NSStringFromRect(table.enclosingScrollView?.contentView.bounds ?? .zero),
                  NSStringFromRect(table.rowView(atRow: 0, makeIfNecessary: false)?.frame ?? .zero), columns.joined(separator: " "))
        case "smartmake":
            Store.shared.createPlaylist(name: argument, rule: parseFilter(json: list.filter) ?? .everything,
                                        sort: parseSort(sort: list.sort, playlist: nil))
        case "rules":
            if let id = Store.shared.playlists.first(where: { $0.smart })?.id {
                Navigation.shared.openFilterDrawer(.playlistRules(id))
            }
        case "play":
            if let row = Int(argument) { list.play(row: row) }
        case "scroll":
            if let row = Int(argument) { table?.scrollRowToVisible(row) }
        case "dense": UISettings.shared.compactRows = argument == "on"
        case "hide": UISettings.shared.hiddenColumns = argument.split(separator: ",").map(String.init)
        case "smart":
            TracksModel.shared.smartPlaylist = SmartPlaylistRequest(filter: list.filter, sort: list.sort)
        default: NSLog("N Music tracks debug: unknown command %@", line)
        }
    }

    private static var table: TrackNSTable? {
        func find(_ view: NSView) -> TrackNSTable? {
            if let table = view as? TrackNSTable, table.window != nil, !table.isHiddenOrHasHiddenAncestor,
               table.controller?.list === TracksModel.shared.list {
                return table
            }
            for child in view.subviews {
                if let found = find(child) { return found }
            }
            return nil
        }
        return Windows.shared.main?.contentView.flatMap(find)
    }

    /// Closes the menu the table opens next, after the seconds.
    private static func closeLastMenu(_ table: TrackNSTable, after seconds: Double) {
        let before = table.controller?.lastMenu
        Task {
            for _ in 0..<40 {
                try? await Task.sleep(for: .milliseconds(50))
                if let menu = table.controller?.lastMenu, menu !== before {
                    try? await Task.sleep(for: .seconds(seconds))
                    menu.cancelTrackingWithoutAnimation()
                    return
                }
            }
        }
    }
}

/// The search text the debug hooks type (the page's field is its own state).
@MainActor
@Observable
final class TracksDebugState {
    static let shared = TracksDebugState()
    var search: String?
    private init() {}
}

#endif
