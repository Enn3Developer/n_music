#if DEBUG
import AppKit
import Observation

/// DEBUG-only commands for the grids, collection and playlist pages, to reach their states
/// without clicking (see `App/DebugHooks.swift` for the shell's). Installed when one of these
/// pages first shows. Post a distributed notification named
/// `com.enn3developer.n-music.debug.groups.<pid>` (this process only: several Debug builds may
/// run at once, and the shell's hooks reach them all) whose object is the command:
///
/// - `go <page id>`: navigate; `size <w> <h>`: the main window's content size; `snapshot <dir> <name>`: the visible windows as PNGs; `state <dir>`: the page and the playlists to `groups-state.txt`.
/// - `search <text>` / `sort <value>`: the grid shown (sort values: artist, name, newest,
///   oldest, mostTracks); `track-sort <sort>`: the collection or playlist page shown.
/// - `rename`, `delete`, `remove`: open that dialog on the page shown; `close` closes it.
/// - `playlist <name>`: a regular playlist of the library's first 3 tracks; `smart <name>
///   <genre>`: a smart playlist of a genre; `empty <name>`: an empty one; `rules`: open the
///   rules drawer of the playlist shown.
@MainActor
@Observable
final class GroupsDebug {
    static let shared = GroupsDebug()

    /// The last command for a page, and a serial telling repeats apart.
    private(set) var command: (verb: String, argument: String, serial: Int)?
    @ObservationIgnored private var installed = false

    private init() {}

    func install() {
        guard !installed else { return }
        installed = true
        Self.listen()
    }

    nonisolated private static func listen() {
        let name = "com.enn3developer.n-music.debug.groups.\(ProcessInfo.processInfo.processIdentifier)"
        DistributedNotificationCenter.default().addObserver(
            forName: .init(name), object: nil, queue: .main
        ) { notification in
            let line = notification.object as? String ?? ""
            Task { @MainActor in GroupsDebug.shared.run(line) }
        }
    }

    private func run(_ line: String) {
        NSLog("N Music groups debug: %@", line)
        let verb = line.split(separator: " ", maxSplits: 1).first.map(String.init) ?? ""
        let argument = line.split(separator: " ", maxSplits: 1).dropFirst().first.map(String.init) ?? ""
        let store = Store.shared
        switch verb {
        case "go":
            Navigation.shared.go(argument)
        case "size":
            let numbers = argument.split(separator: " ").compactMap { Double($0) }
            if numbers.count == 2 { Windows.shared.main?.setContentSize(NSSize(width: numbers[0], height: numbers[1])) }
        case "state":
            let navigation = Navigation.shared
            let lines = [
                "page=\(navigation.page)",
                "playlists=\(store.playlists.map { "\($0.id):\($0.name):\($0.smart)" })",
                "drawer=\(String(describing: navigation.filterDrawer))",
                "width=\(navigation.windowWidth)",
            ]
            let url = URL(fileURLWithPath: argument).appendingPathComponent("groups-state.txt")
            try? lines.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
        case "snapshot":
            // Each visible window (sheets too) as `<name>-<index>.png`.
            let parts = argument.split(separator: " ").map(String.init)
            guard parts.count == 2 else { return }
            for (index, window) in NSApp.windows.enumerated() where window.isVisible {
                guard let view = window.contentView?.superview ?? window.contentView,
                      let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds)
                else { continue }
                view.cacheDisplay(in: view.bounds, to: bitmap)
                let url = URL(fileURLWithPath: parts[0]).appendingPathComponent("\(parts[1])-\(index).png")
                try? bitmap.representation(using: .png, properties: [:])?.write(to: url)
            }
        case "playlist":
            Task {
                let tracks = await store.tracks(.library, offset: 0, limit: 3).map(\.locator)
                store.createPlaylist(name: argument, tracks: tracks)
            }
        case "empty":
            store.createPlaylist(name: argument)
        case "smart":
            let parts = argument.split(separator: " ", maxSplits: 1).map(String.init)
            guard parts.count == 2 else { return }
            store.createPlaylist(name: parts[0], rule: .all([.genre(parts[1])]))
        case "rules":
            if let id = PageID.playlistID(Navigation.shared.argument) {
                Navigation.shared.openFilterDrawer(.playlistRules(id))
            }
        default:
            command = (verb, argument, (command?.serial ?? 0) + 1)
        }
    }
}
#endif
