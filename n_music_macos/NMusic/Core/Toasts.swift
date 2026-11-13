import Foundation
import Observation

/// The in-window toasts at the bottom-right of the page area (spec §7.4): the queued toast
/// ("X plays next" with Open queue) and the red one for a playlist the core turned down (also
/// used for other errors the core reports). The shell draws them (`ToastOverlay`).
@MainActor
@Observable
final class Toasts {
    static let shared = Toasts()

    struct Queued: Equatable {
        /// Tells a re-pop from the last one, so the content cross-fades.
        var serial: Int
        var text: String
    }

    struct Failure: Equatable {
        var serial: Int
        /// `playlist_not_saved` for a rejected playlist.
        var title: String
        /// The core's message, untranslated.
        var message: String
    }

    private(set) var queued: Queued?
    private(set) var failure: Failure?

    @ObservationIgnored private var serial = 0
    @ObservationIgnored private var queuedTimer: Task<Void, Never>?
    @ObservationIgnored private var failureTimer: Task<Void, Never>?

    private init() {}

    /// Lifetime of the queued toast: 4 s, paused while hovered and 2.5 s after the pointer
    /// leaves.
    func showQueued(_ text: String) {
        serial += 1
        queued = Queued(serial: serial, text: text)
        scheduleQueued(after: 4)
    }

    func dismissQueued() {
        queuedTimer?.cancel()
        queued = nil
    }

    /// The pointer is over the queued toast (true) or left it (false).
    func hoverQueued(_ inside: Bool) {
        guard queued != nil else { return }
        if inside {
            queuedTimer?.cancel()
        } else {
            scheduleQueued(after: 2.5)
        }
    }

    /// The red toast: `title` over the core's `message`, for 10 s.
    func showFailure(title: String, message: String) {
        serial += 1
        failure = Failure(serial: serial, title: title, message: message)
        failureTimer?.cancel()
        failureTimer = Task { [weak self] in
            try? await Task.sleep(for: .seconds(10))
            guard !Task.isCancelled else { return }
            self?.failure = nil
        }
    }

    /// A playlist change the core turned down.
    func showRejected(_ message: String) {
        showFailure(title: tr("playlist_not_saved"), message: message)
    }

    func dismissFailure() {
        failureTimer?.cancel()
        failure = nil
    }

    private func scheduleQueued(after seconds: Double) {
        queuedTimer?.cancel()
        queuedTimer = Task { [weak self] in
            try? await Task.sleep(for: .seconds(seconds))
            guard !Task.isCancelled else { return }
            self?.queued = nil
        }
    }
}
