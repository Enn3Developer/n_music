import SwiftUI

/// The main window's content (spec §1.3): the first-run import page alone, or the sidebar, the
/// page area (with the Now Playing panel on the right when wide), and the player bar across the
/// whole width at the bottom.
struct MainWindow: View {
    private let store = Store.shared
    private let navigation = Navigation.shared
    private let settings = UISettings.shared

    var body: some View {
        Group {
            if store.firstRun {
                ImportPage()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(Theme.bg)
            } else {
                Shell()
            }
        }
        .frame(minWidth: 760, minHeight: 480)
        .background(WindowAccessor { Windows.shared.attachMain($0) })
        .background(
            GeometryReader { geometry in
                Color.clear
                    .onAppear { navigation.windowWidth = geometry.size.width }
                    .onChange(of: geometry.size.width) { _, width in navigation.windowWidth = width }
            }
        )
        .tint(Theme.accent)
    }
}

/// Everything but the first run.
private struct Shell: View {
    private let navigation = Navigation.shared
    @State private var columns = NavigationSplitViewVisibility.all

    var body: some View {
        @Bindable var navigation = navigation
        // The player bar under the split view, not a safe-area inset of it: an inset lets the
        // detail column run on under the bar, which then hides the toasts and the pages' ends.
        VStack(spacing: 0) {
            NavigationSplitView(columnVisibility: $columns) {
                Sidebar()
                    .navigationSplitViewColumnWidth(min: 200, ideal: 240, max: 320)
            } detail: {
                HStack(spacing: 0) {
                    PageArea()
                    if navigation.nowPlayingPanelShown {
                        Rectangle().fill(Theme.line).frame(width: 1)
                        NowPlayingPanel()
                            .frame(width: 360)
                            .frame(maxHeight: .infinity)
                            .background(Theme.panel)
                            .transition(.opacity)
                    }
                }
            }
            .navigationTitle("N Music")
            PlayerBar()
        }
        .sheet(item: $navigation.newPlaylistDialog) { request in
            PromptDialog(
                title: tr("new_playlist"),
                placeholder: tr("playlist_name"),
                confirmTitle: tr("create")
            ) { name in
                Store.shared.createPlaylist(name: name, tracks: request.tracks)
                return nil
            }
        }
    }
}

/// The pages, the toasts over their bottom-right corner, and the page arrival (fade + 8-point
/// rise, none under reduced motion; brief §16).
private struct PageArea: View {
    private let navigation = Navigation.shared
    @State private var opacity = 1.0
    @State private var rise = 0.0

    var body: some View {
        ZStack {
            // Each section's page stays alive once shown, keeping its search, filter, sort and
            // scroll (spec §2.1); only the current one shows and takes input.
            ForEach(navigation.visited, id: \.self) { section in
                let current = section == navigation.section
                PageRouter(section: section, argument: navigation.subpages[section] ?? "")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .opacity(current ? opacity : 0)
                    .offset(y: current ? rise : 0)
                    .allowsHitTesting(current)
                    .disabled(!current)
                    .accessibilityHidden(!current)
                    .zIndex(current ? 1 : 0)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Theme.bg)
        // Over the page (and the scrim it lays under it), as Qt's: the page keeps its size.
        .overlay { FilterDrawerPanel() }
        .clipped()
        .overlay(alignment: .bottomTrailing) {
            ToastOverlay()
                .padding(.trailing, 24)
                .padding(.bottom, 20)
        }
        .onChange(of: navigation.arrival) {
            guard !Motion.shared.reduced else { return }
            var instant = Transaction()
            instant.disablesAnimations = true
            withTransaction(instant) {
                opacity = 0
                rise = 8
            }
            Task { @MainActor in
                withAnimation(Motion.standard(Motion.fade)) { opacity = 1 }
                withAnimation(Motion.standard(Motion.move)) { rise = 0 }
            }
        }
    }
}

/// The filter drawer (spec §3.7) while `Navigation.filterDrawer` is set: anchored to the page
/// area's right edge, min(520, page) wide and full height, over the page (whose scrim dims it
/// and closes the drawer on a click), with a 1-point `line2` left edge and a 48-point shadow
/// falling on the page. It comes in fading and sliding 24 points over 250 ms and leaves over
/// 150 ms on the leaving curve; under reduced motion it only fades.
private struct FilterDrawerPanel: View {
    private let navigation = Navigation.shared
    private let motion = Motion.shared

    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: .trailing) {
                if let context = navigation.filterDrawer {
                    HStack(spacing: 0) {
                        LinearGradient(colors: [.clear, Self.shadow], startPoint: .leading, endPoint: .trailing)
                            .frame(width: 48)
                            .allowsHitTesting(false)
                        Rectangle().fill(Theme.line2).frame(width: 1)
                        FilterDrawer(context: context)
                            .frame(width: max(0, min(520, geometry.size.width) - 1))
                            // Clicks missing the controls stay in the drawer.
                            .contentShape(Rectangle())
                    }
                    .frame(maxHeight: .infinity)
                    .transition(.asymmetric(
                        insertion: travel.animation(Motion.standard(Motion.enter)),
                        removal: travel.animation(Motion.leaving(Motion.fade))
                    ))
                }
            }
            .frame(width: geometry.size.width, height: geometry.size.height, alignment: .trailing)
        }
        .animation(Motion.standard(Motion.enter), value: navigation.filterDrawer)
    }

    private var travel: AnyTransition {
        motion.reduced ? .opacity : .opacity.combined(with: .offset(x: 24))
    }

    /// Black at 35 % in dark mode, 12 % in light (`qml/FilterDrawer.qml:146-160`).
    private static let shadow = Color(nsColor: NSColor(name: nil) { appearance in
        let dark = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
        return NSColor(white: 0, alpha: dark ? 0.35 : 0.12)
    })
}

/// The page of `section`, given what followed its `:` the last time it showed.
struct PageRouter: View {
    let section: String
    let argument: String

    var body: some View {
        switch section {
        case "tracks": TracksPage()
        case "albums": GroupsPage(kind: .album)
        case "artists": GroupsPage(kind: .artist)
        case "genres": GroupsPage(kind: .genre)
        case "album": CollectionPage(kind: .album, argument: argument)
        case "artist": CollectionPage(kind: .artist, argument: argument)
        case "genre": CollectionPage(kind: .genre, argument: argument)
        case "source": CollectionPage(kind: .source, argument: argument)
        case "sources": SourcesPage()
        case "playlist": PlaylistPage(id: PageID.playlistID(argument) ?? 0)
        case "queue": QueuePage()
        case "settings": SettingsPage(section: argument.isEmpty ? "playback" : argument)
        // Every section has a page: `Navigation.go` refuses unknown ones.
        default: EmptyView()
        }
    }
}
