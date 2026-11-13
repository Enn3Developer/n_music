import Foundation

/// Runs work from the main run loop instead of from a Swift concurrency job.
///
/// Anything that spins a nested event loop (`NSMenu.popUp`, `NSOpenPanel.runModal`, an
/// `NSAlert`'s `runModal`) must not start inside a main-actor job (a `Task`, an awaited
/// continuation, a `DispatchQueue.main` block): the run loop does not service the main queue
/// from a loop nested inside a main-queue block, so every other main-actor job (the core's
/// events, Now Playing's remote commands, other tasks) would wait until the menu or panel
/// closes. A block performed by the run loop itself is not such a job.
enum MainRunLoop {
    /// Runs `work` on the main thread at the run loop's next pass in `modes`: by default the
    /// default mode, so after any menu tracking or modal session that is running now ends;
    /// `.common` runs it during those too.
    nonisolated static func perform(in modes: [RunLoop.Mode] = [.default], _ work: @escaping @MainActor () -> Void) {
        let job = RunLoopJob(work)
        RunLoop.main.perform(inModes: modes) {
            job.run()
        }
        // A block performed by the run loop doesn't wake it by itself.
        CFRunLoopWakeUp(CFRunLoopGetMain())
    }
}

/// Work carried to the main run loop: created anywhere, run only there.
private final class RunLoopJob: @unchecked Sendable {
    private let work: @MainActor () -> Void

    init(_ work: @escaping @MainActor () -> Void) {
        self.work = work
    }

    nonisolated func run() {
        // The main run loop calls it on the main thread.
        MainActor.assumeIsolated { work() }
    }
}
