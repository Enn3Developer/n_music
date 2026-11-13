#if DEBUG
import AppKit

/// Drives the Settings page of a Debug build from outside (as `DebugHooks`): post a
/// distributed notification named `com.enn3developer.n-music.debug.settings` whose object is
/// the command. Installed once Settings has shown (reach it with the Tracks hook's
/// `@<pid> go settings:playback`).
///
/// Prefix a command with `@<pid> ` to speak to one app of several. Commands:
/// `go <page id>`, `back`, `forward` (this app's history only, unlike the foundation's
/// broadcast), `scroll <points>` (scrolls the settings by that much, down when positive), `top`,
/// `updates idle|checking|current|available|failed|check` (shows that state, or checks),
/// `theme system|light|dark`,
/// `state <dir>` (writes `settings-state.txt`: the page, the section in view, the settings).
@MainActor
enum SettingsDebug {
    private static var installed = false
    /// The section the page shows as in view, reported by the page.
    static var visible = ""

    static func install() {
        guard !installed else { return }
        installed = true
        listen()
    }

    nonisolated private static func listen() {
        DistributedNotificationCenter.default().addObserver(
            forName: .init("com.enn3developer.n-music.debug.settings"), object: nil, queue: .main
        ) { notification in
            let line = notification.object as? String ?? ""
            Task { @MainActor in SettingsDebug.run(line) }
        }
    }

    private static func run(_ line: String) {
        var line = line
        if line.hasPrefix("@") {
            let target = line.dropFirst().prefix { $0 != " " }
            guard Int32(target) == ProcessInfo.processInfo.processIdentifier else { return }
            line = String(line.dropFirst(target.count + 2))
        }
        NSLog("N Music settings debug: %@", line)
        let verb = line.split(separator: " ", maxSplits: 1).first.map(String.init) ?? ""
        let argument = line.count > verb.count ? String(line.dropFirst(verb.count + 1)) : ""
        switch verb {
        case "scroll":
            guard let amount = Double(argument), let scroll = settingsScrollView() else { return }
            let clip = scroll.contentView
            let limit = max(0, (scroll.documentView?.frame.height ?? 0) - clip.bounds.height)
            let y = min(max(0, clip.bounds.origin.y + amount), limit)
            clip.scroll(to: NSPoint(x: clip.bounds.origin.x, y: y))
            scroll.reflectScrolledClipView(clip)
        case "top":
            guard let scroll = settingsScrollView() else { return }
            scroll.contentView.scroll(to: .zero)
            scroll.reflectScrolledClipView(scroll.contentView)
        case "go": Navigation.shared.go(argument)
        case "back": Navigation.shared.back()
        case "forward": Navigation.shared.forward()
        case "updates":
            Updates.shared.debugShow(argument)
        case "theme":
            if let theme = UISettings.ThemeChoice(rawValue: argument.capitalized) { UISettings.shared.theme = theme }
        case "state":
            let navigation = Navigation.shared
            let settings = UISettings.shared
            let store = Store.shared
            let lines = [
                "page=\(navigation.page) section=\(navigation.section) subpage=\(navigation.subpages["settings"] ?? "-")",
                "back=\(navigation.canGoBack) forward=\(navigation.canGoForward) visible=\(visible) offset=\(settingsScrollView().map { "\($0.contentView.bounds.origin.y)/\(($0.documentView?.frame.height ?? 0) - $0.contentView.bounds.height)" } ?? "-")",
                "theme=\(settings.theme) accent=\(settings.accent) compact=\(settings.compactRows) reduce=\(settings.reduceMotion)",
                "hidden=\(settings.hiddenColumns) locale=\(settings.locale ?? "system") window=\(settings.saveWindowSize) mini=\(settings.miniOnTop) check=\(settings.checkUpdates)",
                "options=\(store.options) device=\(store.outputDevice?.name ?? "system") devices=\(store.outputDevices.map(\.name))",
                "cache=\(String(describing: store.streamCache)) telegram=\(String(describing: store.telegram))",
                "updates=\(Updates.shared.status) version=\(Updates.shared.version)",
            ]
            let url = URL(fileURLWithPath: argument.isEmpty ? NSTemporaryDirectory() : argument)
                .appendingPathComponent("settings-state.txt")
            try? lines.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8)
        default:
            NSLog("N Music settings debug: unknown command %@", line)
        }
    }

    /// The settings' scroll view: right of the sidebar, the one with the tallest document.
    private static func settingsScrollView() -> NSScrollView? {
        guard let content = Windows.shared.main?.contentView else { return nil }
        var found: [NSScrollView] = []
        func walk(_ view: NSView) {
            if let scroll = view as? NSScrollView, !scroll.isHiddenOrHasHiddenAncestor, scroll.visibleRect.width > 0,
               scroll.convert(scroll.bounds, to: nil).minX > 260 {
                found.append(scroll)
            }
            view.subviews.forEach(walk)
        }
        walk(content)
        return found.max { ($0.documentView?.frame.height ?? 0) < ($1.documentView?.frame.height ?? 0) }
    }
}

extension Updates {
    /// Shows a state without asking GitHub (`check` asks).
    func debugShow(_ name: String) {
        switch name {
        case "idle": debugStatus = .idle
        case "checking": debugStatus = .checking
        case "current": debugStatus = .current(checked: Date())
        case "available":
            debugStatus = .available(version: "2.0.0-beta.1", page: URL(string: "https://github.com/Enn3Developer/n_music/releases")!)
        case "failed": debugStatus = .failed
        default: check()
        }
    }
}
#endif
