import AppKit

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private var nowPlaying: NowPlaying?
    private var power: Power?
    private var devices: OutputDevices?
    private var monitors: [Any] = []

    func applicationDidFinishLaunching(_ notification: Notification) {
        let store = Store.shared
        UISettings.shared.applyAppearance()
        _ = Motion.shared
        nowPlaying = NowPlaying(store: store)
        let power = Power(store: store)
        Power.shared = power
        self.power = power
        devices = OutputDevices(store: store)
        monitors = Self.installMonitors()
        #if DEBUG
        DebugHooks.install()
        #endif
        Windows.shared.openMain()
    }

    /// Closing the main window quits, as the Qt app (the mini player's close means "expand").
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    /// Quitting (⌘Q, the last window closed, the Dock) waits for the core to stop cleanly: the
    /// session's exact place and the settings not yet written (the window's size saved as it
    /// closed among them) are saved first, as the Qt app does on exit.
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        Store.shared.shutdown {
            NSApp.reply(toApplicationShouldTerminate: true)
        }
        return .terminateLater
    }

    /// A Dock click shows the main window, from the mini player too. The app owns its windows:
    /// SwiftUI's own handling would open the (empty) Settings scene.
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if Windows.shared.miniShown {
            Windows.shared.expand()
        } else if let main = Windows.shared.main, !main.isVisible || main.isMiniaturized {
            Windows.shared.showMain()
        }
        return false
    }

    func applicationDockMenu(_ sender: NSApplication) -> NSMenu? {
        let store = Store.shared
        let menu = NSMenu()
        if let track = store.current?.track {
            let title = NSMenuItem(title: track.title, action: nil, keyEquivalent: "")
            title.isEnabled = false
            menu.addItem(title)
            menu.addItem(.separator())
        }
        menu.addItem(item(tr(store.playing ? "pause" : "play"), #selector(togglePause)))
        menu.addItem(item(tr("next"), #selector(next)))
        menu.addItem(item(tr("previous"), #selector(previous)))
        return menu
    }

    private func item(_ title: String, _ action: Selector) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        return item
    }

    @objc private func togglePause() { Store.shared.togglePause() }
    @objc private func next() { Store.shared.next() }
    @objc private func previous() { Store.shared.previous() }

    /// Space plays and pauses in the main window and the mini player, unless something that
    /// takes text has the focus or a sheet is open (a menu shortcut would take it from text
    /// fields). The mouse's back and forward buttons move through the history anywhere in the
    /// main window, except during the first run.
    ///
    /// Made outside the main actor: AppKit calls monitors on the main thread today, and the
    /// handlers check that rather than trust it.
    nonisolated private static func installMonitors() -> [Any] {
        var monitors: [Any] = []
        if let keys = NSEvent.addLocalMonitorForEvents(matching: .keyDown, handler: { event in
            guard event.keyCode == 49,
                  event.modifierFlags.intersection([.command, .control, .option, .shift]).isEmpty
            else { return event }
            // AppKit hands the event over on the main thread, where it stays.
            nonisolated(unsafe) let event = event
            let handled = MainActor.assumeIsolated { () -> Bool in
                #if DEBUG
                NSLog("N Music: Space in %@", event.window?.title ?? "no window")
                #endif
                guard let window = event.window, window.attachedSheet == nil, !window.isSheet,
                      window === Windows.shared.main || window is MiniPlayerWindow,
                      !(window.firstResponder is NSText)
                else { return false }
                Store.shared.togglePause()
                return true
            }
            return handled ? nil : event
        }) {
            monitors.append(keys)
        }
        if let mouse = NSEvent.addLocalMonitorForEvents(matching: .otherMouseDown, handler: { event in
            let number = event.buttonNumber
            guard number == 3 || number == 4 else { return event }
            nonisolated(unsafe) let event = event
            let handled = MainActor.assumeIsolated { () -> Bool in
                guard let window = event.window, window === Windows.shared.main,
                      !Store.shared.firstRun
                else { return false }
                if number == 3 { Navigation.shared.back() } else { Navigation.shared.forward() }
                return true
            }
            return handled ? nil : event
        }) {
            monitors.append(mouse)
        }
        return monitors
    }
}
