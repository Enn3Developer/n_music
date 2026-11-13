import SwiftUI

/// The native menu bar (brief §3), additive to the Qt UX:
/// File › New Playlist ⌘N, Add Source…; Controls › Play/Pause (Space in the window), Next ⌘→,
/// Previous ⌘←, Volume Up ⌘↑, Volume Down ⌘↓, Shuffle, Repeat; View › Show Queue ⌘U, Mini Player
/// ⌥⌘M, Filter ⌥⌘F (and Edit › Find ⌘F: the Tracks search); Go › Back ⌘[, Forward ⌘], the pages
/// ⌘1…⌘6; the app menu's Settings… ⌘, opens the Settings page.
///
/// Menu names the Qt app never had use `tr(_:fallback:)`: English until a translation adds them.
struct AppCommands: Commands {
    private let store = Store.shared
    private let navigation = Navigation.shared
    @Environment(\.openWindow) private var openWindow

    var body: some Commands {
        // Handed to `Windows`, which opens the main window when a launch didn't.
        let _ = Windows.shared.opener = openWindow
        // View › Show/Hide Sidebar (⌃⌘S), the native sidebar's own command.
        SidebarCommands()

        CommandGroup(replacing: .appSettings) {
            Button(tr("settings") + "…") { navigation.go(PageID.settings) }
                .keyboardShortcut(",", modifiers: .command)
                .disabled(store.firstRun)
        }

        CommandGroup(replacing: .newItem) {
            Button(tr("new_playlist_ellipsis")) { show { navigation.requestNewPlaylist() } }
                .keyboardShortcut("n", modifiers: .command)
                .disabled(store.firstRun)
            Button(tr("add_source") + "…") { show { navigation.requestAddSource() } }
                .disabled(store.firstRun)
        }

        CommandGroup(after: .textEditing) {
            Button(tr("menu_find", fallback: "Find")) { show { navigation.requestFocusSearch() } }
                .keyboardShortcut("f", modifiers: .command)
                .disabled(store.firstRun)
        }

        CommandMenu(tr("menu_controls", fallback: "Controls")) {
            // Space plays and pauses in the window (AppDelegate), where text fields don't want it.
            Button(tr(store.playing ? "pause" : "play")) { store.togglePause() }
            Divider()
            Button(tr("next")) { store.next() }
                .keyboardShortcut(.rightArrow, modifiers: .command)
            Button(tr("previous")) { store.previous() }
                .keyboardShortcut(.leftArrow, modifiers: .command)
            Divider()
            Button(tr("volume_up", fallback: "Volume Up")) { store.setVolume(store.volume + 0.1) }
                .keyboardShortcut(.upArrow, modifiers: .command)
            Button(tr("volume_down", fallback: "Volume Down")) { store.setVolume(store.volume - 0.1) }
                .keyboardShortcut(.downArrow, modifiers: .command)
            Divider()
            Toggle(tr("shuffle"), isOn: Binding(get: { store.shuffle }, set: { store.setShuffle($0) }))
            Picker(tr("menu_repeat", fallback: "Repeat"), selection: Binding(
                get: { store.loopStatus }, set: { store.setLoopStatus($0) }
            )) {
                Text(tr("end_stop")).tag(LoopStatus.off)
                Text(tr("end_repeat_all")).tag(LoopStatus.playlist)
                Text(tr("end_repeat_one")).tag(LoopStatus.file)
            }
        }

        CommandGroup(after: .sidebar) {
            Button(tr("menu_show_queue", fallback: "Show Queue")) { show { navigation.toggleQueue() } }
                .keyboardShortcut("u", modifiers: .command)
                .disabled(store.firstRun)
            Button(tr("mini_player")) { Windows.shared.showMiniPlayer() }
                .keyboardShortcut("m", modifiers: [.command, .option])
                .disabled(store.firstRun)
            Button(tr("filter")) { show { navigation.openFilterDrawer(.tracks) } }
                .keyboardShortcut("f", modifiers: [.command, .option])
                .disabled(store.firstRun)
            Divider()
        }

        CommandMenu(tr("menu_go", fallback: "Go")) {
            Button(tr("go_back")) { navigation.back() }
                .keyboardShortcut("[", modifiers: .command)
                .disabled(store.firstRun || !navigation.canGoBack)
            Button(tr("go_forward")) { navigation.forward() }
                .keyboardShortcut("]", modifiers: .command)
                .disabled(store.firstRun || !navigation.canGoForward)
            Divider()
            Group {
                page("tracks", "1")
                page("albums", "2")
                page("artists", "3")
                page("genres", "4")
                page("sources", "5")
                page("queue", "6")
                page("settings", nil)
            }
            .disabled(store.firstRun)
        }
    }

    private func page(_ name: String, _ key: Character?) -> some View {
        let button = Button(tr(name)) { show { navigation.go(name) } }
        return Group {
            if let key {
                button.keyboardShortcut(KeyEquivalent(key), modifiers: .command)
            } else {
                button
            }
        }
    }

    /// Runs `action` in the main window, bringing it back from the mini player first.
    private func show(_ action: () -> Void) {
        if Windows.shared.miniShown {
            Windows.shared.expand()
        }
        action()
    }
}
