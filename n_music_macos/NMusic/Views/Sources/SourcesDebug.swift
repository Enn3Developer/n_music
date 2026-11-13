#if DEBUG
import AppKit
import Observation

/// Debug-only commands for the Sources and import pages, so their dialogs and menus can be
/// checked without clicking: post a distributed notification named
/// `com.enn3developer.n-music.debug.sources` whose object is the command.
///
/// Sources page: `web`, `telegram`, `folder`, `kinds`, `search <text>`, `sort name|tracks`,
/// `rename <i>`, `remove <i>`, `actions`, `reload` (all metadata), `sort-menu`, `card-menu <i>` (i: the card's place).
/// Folder panel: `folder-show <path>`, `folder-pick <path>` (shows it and presses Add).
/// Web dialog: `web-address <text>`, `web-name <text>`, `web-confirm`.
/// Import page: `import-web`, `import-telegram`, `import-folder`, `import-stage <location>`, `import-unstage`
/// (the first row), `import-build`.
/// Both: `close` (the menu, panel or sheet shown), `go <page>`, `size <w> <h>`.
/// Prefix `@<pid> ` to reach one process only.
@MainActor
@Observable
final class SourcesDebug {
    static let shared = SourcesDebug()

    struct Command {
        let name: String
        let argument: String
    }

    private(set) var request = 0
    @ObservationIgnored private(set) var command: Command?
    @ObservationIgnored private var installed = false

    static func install() {
        guard !shared.installed else { return }
        shared.installed = true
        listen()
    }

    nonisolated private static func listen() {
        DistributedNotificationCenter.default().addObserver(
            forName: .init("com.enn3developer.n-music.debug.sources"), object: nil, queue: .main
        ) { @Sendable notification in
            let line = notification.object as? String ?? ""
            Task { @MainActor in shared.run(line) }
        }
    }

    private func run(_ line: String) {
        // `@<pid> command`: several Debug builds may run at once; only that process answers.
        var line = line
        if line.hasPrefix("@") {
            let target = line.dropFirst().prefix { $0 != " " }
            guard Int32(target) == ProcessInfo.processInfo.processIdentifier else { return }
            line = String(line.dropFirst(target.count + 2))
        }
        NSLog("N Music sources debug: %@", line)
        let name = String(line.prefix { $0 != " " })
        let argument = line.count > name.count ? String(line.dropFirst(name.count + 1)) : ""
        switch name {
        case "go":
            Navigation.shared.go(argument)
            return
        case "size":
            let numbers = argument.split(separator: " ").compactMap { Double($0) }
            if numbers.count == 2 { Windows.shared.main?.setContentSize(NSSize(width: numbers[0], height: numbers[1])) }
            return
        default: break
        }
        if name == "kinds-render" {
            SourceMenus.renderKindRows(to: argument)
            return
        }
        if name == "folder-show", let panel = FolderPanel.shown, panel.isVisible {
            // Shows `argument` in the folder panel, without choosing it.
            panel.directoryURL = URL(fileURLWithPath: argument, isDirectory: true)
            return
        }
        if name == "folder-pick", let panel = FolderPanel.shown, panel.isVisible {
            // Shows `argument` in the folder panel and presses its button (no folder selected:
            // the one shown is chosen, and validated).
            panel.directoryURL = URL(fileURLWithPath: argument, isDirectory: true)
            Task { @MainActor in
                try? await Task.sleep(for: .milliseconds(600))
                // From the run loop, not from this task (`MainRunLoop`).
                MainRunLoop.perform { panel.ok(nil) }
            }
            return
        }
        if name == "close" {
            SourceMenus.shown?.cancelTracking()
            if let panel = FolderPanel.shown, panel.isVisible { panel.cancel(nil) }
            // The pages put their sheets away too.
        }
        command = Command(name: name, argument: argument)
        request += 1
    }
}
#endif
