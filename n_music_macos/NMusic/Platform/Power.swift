import AppKit

/// Keeps the app's power use down, and playback safe from it.
///
/// - Tells the core whether any window of the app is on screen (`AppVisibilityChanged`):
///   hidden, it reports positions only as often as it saves the session instead of four times a
///   second, and the clocks (`PlayClock`, which read `Store.windowVisible`) stop redrawing.
/// - While playing, holds a ProcessInfo activity that keeps App Nap from throttling the threads
///   that decode ahead of the device; paused, it lets the app nap. Idle system sleep is left to
///   Core Audio, which keeps the Mac awake while it plays.
@MainActor
final class Power {
    private let store: Store
    private var activity: NSObjectProtocol?
    private var visible: Bool?
    private var pending = false

    init(store: Store) {
        self.store = store
        Self.watchWindows()
        store.observers.append { [weak self] event in
            if case .playbackChanged(let playing) = event {
                self?.updateActivity(playing: playing)
            }
        }
        updateVisibility()
    }

    /// The notifications come on the main queue; the closures are made here, outside the main
    /// actor, and hop to it.
    nonisolated private static func watchWindows() {
        let center = NotificationCenter.default
        for name in [
            NSApplication.didChangeOcclusionStateNotification,
            NSApplication.didHideNotification,
            NSApplication.didUnhideNotification,
            NSWindow.didChangeOcclusionStateNotification,
            NSWindow.willCloseNotification,
            NSWindow.didMiniaturizeNotification,
            NSWindow.didDeminiaturizeNotification,
            NSWindow.didBecomeKeyNotification,
        ] {
            center.addObserver(forName: name, object: nil, queue: .main) { _ in
                Task { @MainActor in Power.shared?.updateVisibility() }
            }
        }
    }

    /// The one the app delegate made.
    static weak var shared: Power?

    func updateVisibility() {
        // A window closing still counts as on screen as the notification comes: look a moment
        // later, once for a burst of notifications.
        guard !pending else { return }
        pending = true
        Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(50))
            pending = false
            let visible = !NSApp.isHidden && NSApp.windows.contains { window in
                window.isVisible && window.occlusionState.contains(.visible) && !window.isMiniaturized
                    && window.contentViewController != nil
            }
            guard visible != self.visible else { return }
            self.visible = visible
            store.setAppVisible(visible)
        }
    }

    private func updateActivity(playing: Bool) {
        if playing, activity == nil {
            activity = ProcessInfo.processInfo.beginActivity(
                options: [.userInitiatedAllowingIdleSystemSleep],
                reason: "Playing music"
            )
        } else if !playing, let activity {
            ProcessInfo.processInfo.endActivity(activity)
            self.activity = nil
        }
    }
}
