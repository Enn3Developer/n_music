import AppKit
import SwiftUI

/// The app's two windows: the SwiftUI main window (reported by `WindowAccessor`) and the mini
/// player, an AppKit window (spec §4.4, brief §14).
///
/// - SwiftUI opens the main window at launch only once the app is activated: a launch that
///   doesn't activate it (`open -g`, a login item) would show nothing, so `openMain()` opens it
///   then, through the `openWindow` action the menu bar's commands hand over (`opener`).
@MainActor
final class Windows: NSObject {
    static let shared = Windows()

    private(set) weak var main: NSWindow?
    /// SwiftUI's `openWindow`, from the commands (the one place that has it at launch).
    var opener: OpenWindowAction?
    private var mini: MiniPlayerWindow?
    private var resizeSave: Task<Void, Never>?

    /// The mini player shows (and the main window is hidden for it).
    var miniShown: Bool { mini?.isVisible == true }

    /// Opens the main window at launch when SwiftUI didn't (the app not being activated),
    /// without activating the app.
    func openMain() {
        Task {
            // The commands run (handing `opener` over) and an activated launch opens the
            // window within the first run loop turns.
            for _ in 0..<50 {
                if main != nil { return }
                if let opener {
                    try? await Task.sleep(for: .milliseconds(100))
                    if main == nil { opener(id: "main") }
                    return
                }
                try? await Task.sleep(for: .milliseconds(20))
            }
        }
    }

    func attachMain(_ window: NSWindow) {
        guard main !== window else { return }
        main = window
        window.isRestorable = false
        window.setFrameAutosaveName("")
        window.contentMinSize = NSSize(width: 760, height: 480)
        let settings = UISettings.shared
        var size = NSSize(width: 1280, height: 800)
        if settings.saveWindowSize {
            size = NSSize(width: settings.windowSize.width, height: settings.windowSize.height)
        }
        if let screen = (window.screen ?? NSScreen.main)?.visibleFrame {
            size.width = min(max(size.width, 760), screen.width)
            size.height = min(max(size.height, 480), screen.height)
        }
        window.setContentSize(size)
        window.center()
        if settings.saveWindowSize && settings.windowSize.maximized && !window.isZoomed {
            window.zoom(nil)
        }
        Self.watch(window)
    }

    /// Size changes are saved 500 ms after they settle, and as the window closes; only the
    /// size it has when not zoomed, with whether it is.
    nonisolated private static func watch(_ window: NSWindow) {
        let center = NotificationCenter.default
        for name in [NSWindow.didResizeNotification, NSWindow.willCloseNotification] {
            center.addObserver(forName: name, object: window, queue: .main) { notification in
                let closing = notification.name == NSWindow.willCloseNotification
                Task { @MainActor in Windows.shared.sizeChanged(closing: closing) }
            }
        }
    }

    private func sizeChanged(closing: Bool) {
        guard UISettings.shared.saveWindowSize, let main else { return }
        resizeSave?.cancel()
        let save = { [weak self] in
            guard let main = self?.main else { return }
            let zoomed = main.isZoomed
            var saved = UISettings.shared.windowSize
            if !zoomed {
                let size = main.contentRect(forFrameRect: main.frame).size
                saved.width = size.width
                saved.height = size.height
            }
            saved.maximized = zoomed
            if saved != UISettings.shared.windowSize {
                UISettings.shared.windowSize = saved
            }
        }
        if closing {
            save()
        } else {
            _ = main
            resizeSave = Task {
                try? await Task.sleep(for: .milliseconds(500))
                guard !Task.isCancelled else { return }
                save()
            }
        }
    }

    /// Shows the main window; `activate`: and the app (Debug hooks pass false, never taking
    /// the focus from whoever is typing).
    func showMain(activate: Bool = true) {
        guard let main else { return }
        if main.isMiniaturized { main.deminiaturize(nil) }
        if activate {
            main.makeKeyAndOrderFront(nil)
            NSApp.activate()
        } else {
            main.orderFrontRegardless()
        }
    }

    /// Hides the main window and shows the mini player.
    func showMiniPlayer(activate: Bool = true) {
        let window = mini ?? MiniPlayerWindow()
        mini = window
        window.applyLevel()
        main?.orderOut(nil)
        if activate {
            window.makeKeyAndOrderFront(nil)
            NSApp.activate()
        } else {
            window.orderFrontRegardless()
        }
    }

    /// Back to the full window: hides the mini player and shows the main window.
    func expand(activate: Bool = true) {
        mini?.orderOut(nil)
        showMain(activate: activate)
    }
}

/// The mini player: 420 × 128 by default, 360 to 640 wide, 128 high; floating while "Keep the
/// mini player on top" is on; closing it means "expand". Titled "<title> – N Music".
final class MiniPlayerWindow: NSWindow, NSWindowDelegate {
    init() {
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 420, height: 128),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        isReleasedWhenClosed = false
        identifier = NSUserInterfaceItemIdentifier("mini")
        contentMinSize = NSSize(width: 360, height: 128)
        contentMaxSize = NSSize(width: 640, height: 128)
        contentViewController = NSHostingController(rootView: MiniPlayerView())
        setContentSize(NSSize(width: 420, height: 128))
        center()
        delegate = self
        collectionBehavior.insert(.fullScreenNone)
        updateTitle()
        observe()
    }

    func applyLevel() {
        level = UISettings.shared.miniOnTop ? .floating : .normal
    }

    private func updateTitle() {
        if let track = Store.shared.current?.track {
            title = "\(track.title) – N Music"
        } else {
            title = "N Music"
        }
    }

    /// Follows the current track (title) and the on-top setting (level).
    private func observe() {
        withObservationTracking {
            _ = Store.shared.current?.track.title
            _ = UISettings.shared.miniOnTop
        } onChange: { [weak self] in
            Task { @MainActor in
                self?.updateTitle()
                self?.applyLevel()
                self?.observe()
            }
        }
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        Windows.shared.expand()
        return false
    }
}

/// Reports the NSWindow a SwiftUI view is in.
struct WindowAccessor: NSViewRepresentable {
    let found: @MainActor (NSWindow) -> Void

    func makeNSView(context: Context) -> NSView {
        let view = ReportingView()
        view.found = found
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {}

    final class ReportingView: NSView {
        var found: (@MainActor (NSWindow) -> Void)?

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            if let window { found?(window) }
        }
    }
}
