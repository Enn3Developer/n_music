import SwiftUI

@main
struct NMusicApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate

    var body: some Scene {
        // One main window, as the Qt app; the mini player is its own AppKit window
        // (`MiniPlayerWindow`), and Settings is a page in this one (brief §4). SwiftUI opens
        // it at launch only when the app is activated: `Windows.openMain()` opens it otherwise.
        Window("N Music", id: "main") {
            MainWindow()
        }
        .defaultSize(width: 1280, height: 800)
        .windowToolbarStyle(.unified(showsTitle: true))
        .commands {
            AppCommands()
        }
    }
}
