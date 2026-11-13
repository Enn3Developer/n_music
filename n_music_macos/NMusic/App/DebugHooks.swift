#if DEBUG
import AppKit

/// Drives a Debug build from outside, for checking it without clicking through it (or moving
/// the user's pointer). Post a distributed notification named
/// `com.enn3developer.n-music.debug` whose object is the command:
///
///     swift -e 'import Foundation; DistributedNotificationCenter.default()
///         .postNotificationName(.init("com.enn3developer.n-music.debug"),
///                               object: "@4242 go albums", deliverImmediately: true)'
///
/// `@<pid> ` first speaks to that process only; without it every running Debug build obeys.
/// Nothing here activates the app (the person at the Mac may be typing elsewhere).
///
/// Commands: `go <page id>`, `back`, `forward`, `size <width> <height>` (the main window's
/// content), `window` (orders the main window in front, without activating), `mini`,
/// `expand`, `play-all`, `play`, `pause`, `next`, `previous`, `volume <0…1>`, `toast`,
/// `reject`, `new-playlist`, `close-dialog` (the New playlist one), `drawer` (the Tracks filter drawer), `close-drawer`,
/// `snapshot <dir>` (each visible window as a PNG, drawn by the app: no screen recording, works
/// while covered), `dump <dir>` (the accessibility trees as text), `state <dir>` (the store and
/// navigation as text), `menus <dir>` (the menu bar as text), `quit`.
///
/// The areas' own hooks (`TracksDebug`, `GroupsDebug`, `QueueDebug`, `SourcesDebug`,
/// `SettingsDebug`) listen on their own names from launch on: see each file.
@MainActor
enum DebugHooks {
    static func install() {
        DistributedNotificationCenter.default().addObserver(
            forName: .init("com.enn3developer.n-music.debug"), object: nil, queue: .main
        ) { notification in
            let command = notification.object as? String ?? ""
            Task { @MainActor in run(command) }
        }
        // The areas' hooks, from launch on (not only once their page has shown).
        TracksDebug.install()
        GroupsDebug.shared.install()
        QueueDebug.install()
        SourcesDebug.install()
        SettingsDebug.install()
    }

    /// The command of `line` for this process: `nil` when it names another one (`@<pid> `).
    static func addressed(_ line: String) -> String? {
        guard line.hasPrefix("@") else { return line }
        let target = line.dropFirst().prefix { $0 != " " }
        guard Int32(target) == ProcessInfo.processInfo.processIdentifier else { return nil }
        return String(line.dropFirst(target.count + 1).drop { $0 == " " })
    }

    private static func run(_ line: String) {
        guard let line = addressed(line) else { return }
        let parts = line.split(separator: " ").map(String.init)
        let argument = parts.count > 1 ? parts[1] : ""
        let store = Store.shared
        let navigation = Navigation.shared
        NSLog("N Music debug: %@", line)
        switch parts.first {
        case "go": navigation.go(argument)
        case "back": navigation.back()
        case "forward": navigation.forward()
        case "size":
            if parts.count > 2, let width = Double(parts[1]), let height = Double(parts[2]) {
                Windows.shared.main?.setContentSize(NSSize(width: width, height: height))
            }
        case "window": Windows.shared.showMain(activate: false)
        case "mini": Windows.shared.showMiniPlayer(activate: false)
        case "expand": Windows.shared.expand(activate: false)
        case "play-all": store.play(.library)
        case "play": store.play()
        case "pause": store.pause()
        case "next": store.next()
        case "previous": store.previous()
        case "volume": if let volume = Double(argument) { store.setVolume(volume) }
        case "toast": Toasts.shared.showQueued(tr("plays_next", "Debug track"))
        case "reject": Toasts.shared.showRejected("A debug rejection.")
        case "new-playlist": navigation.requestNewPlaylist()
        case "close-dialog": navigation.newPlaylistDialog = nil
        case "drawer": navigation.openFilterDrawer(.tracks)
        case "close-drawer": navigation.filterDrawer = nil
        case "snapshot": snapshot(to: argument.isEmpty ? NSTemporaryDirectory() : argument)
        case "dump": dump(to: argument.isEmpty ? NSTemporaryDirectory() : argument)
        case "state": state(to: argument.isEmpty ? NSTemporaryDirectory() : argument)
        case "menus": menus(to: argument.isEmpty ? NSTemporaryDirectory() : argument)
        case "quit": NSApp.terminate(nil)
        default: NSLog("N Music debug: unknown command %@", line)
        }
    }

    private static func menus(to directory: String) {
        var lines: [String] = []
        func walk(_ menu: NSMenu, depth: Int) {
            for item in menu.items {
                let key = item.keyEquivalent.isEmpty ? "" : " [\(item.keyEquivalentModifierMask.rawValue):\(item.keyEquivalent)]"
                lines.append(String(repeating: "  ", count: depth) + (item.isSeparatorItem ? "---" : item.title + key + (item.isEnabled ? "" : " (disabled)")))
                if let submenu = item.submenu, depth < 3 { walk(submenu, depth: depth + 1) }
            }
        }
        if let main = NSApp.mainMenu { walk(main, depth: 0) }
        let url = URL(fileURLWithPath: directory).appendingPathComponent("menus.txt")
        try? lines.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
    }

    private static func state(to directory: String) {
        let store = Store.shared
        let navigation = Navigation.shared
        let lines = [
            "page=\(navigation.page) back=\(navigation.canGoBack) forward=\(navigation.canGoForward)",
            "visited=\(navigation.visited)",
            "width=\(navigation.windowWidth) wide=\(navigation.isWide)",
            "firstRun=\(store.firstRun) ready=\(store.ready) roots=\(store.roots?.map(\.key) ?? [])",
            "summary=\(store.summary.tracks) tracks, \(store.summary.length) s",
            "scan=\(store.scan.map { "\($0.read)/\($0.found)" } ?? "none") libraryVersion=\(store.libraryVersion)",
            "playlists=\(store.playlists.map(\.name))",
            "sources=\(store.sources.map(\.displayName))",
            "playing=\(store.playing) current=\(store.current?.track.title ?? "-") volume=\(store.volume)",
            "position=\(store.position()) length=\(store.length) queue=\(store.queue.count)",
            "loop=\(store.loopStatus) shuffle=\(store.shuffle) devices=\(store.outputDevices.map(\.name))",
            "windows=\(NSApp.windows.map { "\(type(of: $0)) '\($0.title)' visible=\($0.isVisible) \(Int($0.frame.width))x\(Int($0.frame.height)) occlusion=\($0.occlusionState.contains(.visible))" })",
            "toasts=\(String(describing: Toasts.shared.queued)) \(String(describing: Toasts.shared.failure))",
            "windowVisible=\(store.windowVisible) language=\(Tr.shared.language ?? "system") strings=\(Tr.shared.strings.count)",
        ]
        let url = URL(fileURLWithPath: directory).appendingPathComponent("state.txt")
        try? lines.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
    }

    /// Writes the windows' accessibility trees as text: what is on screen, without a screenshot.
    private static func dump(to directory: String) {
        var lines: [String] = []
        func walk(_ element: Any, depth: Int) {
            guard depth < 40, let element = element as? NSAccessibilityProtocol else { return }
            let role = element.accessibilityRole()?.rawValue ?? "?"
            let label = element.accessibilityLabel() ?? ""
            let value = (element.accessibilityValue() as? CustomStringConvertible)?.description ?? ""
            let title = element.accessibilityTitle() ?? ""
            let text = [title, label, value].filter { !$0.isEmpty }.joined(separator: " | ")
            lines.append(String(repeating: "  ", count: depth) + role + (text.isEmpty ? "" : ": " + text))
            var children = element.accessibilityChildren() ?? []
            if children.isEmpty, let view = element as? NSView {
                children = view.subviews
            }
            for child in children {
                walk(child, depth: depth + 1)
            }
        }
        for window in NSApp.windows where window.isVisible {
            lines.append("WINDOW \(window.title) \(Int(window.frame.width))x\(Int(window.frame.height))")
            if let content = window.contentView { walk(content, depth: 1) }
        }
        let url = URL(fileURLWithPath: directory).appendingPathComponent("accessibility.txt")
        try? lines.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
    }

    private static func snapshot(to directory: String) {
        for (index, window) in NSApp.windows.enumerated() where window.isVisible {
            guard let view = window.contentView?.superview ?? window.contentView,
                  let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds)
            else { continue }
            view.cacheDisplay(in: view.bounds, to: bitmap)
            let url = URL(fileURLWithPath: directory).appendingPathComponent("window-\(index).png")
            try? bitmap.representation(using: .png, properties: [:])?.write(to: url)
        }
    }
}
#endif
