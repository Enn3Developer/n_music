import Foundation
import Observation

/// The main window's navigation: the history, the page shown, what each section showed last,
/// and the requests one part of the window makes of another (open the New playlist dialog,
/// focus the Tracks search…). One for the app: `Navigation.shared`.
///
/// Pages call `go(_:)` with ids from `PageID`; back/forward are the sidebar buttons, ⌘[ ⌘],
/// the mouse's back/forward buttons and the Go menu.
@MainActor
@Observable
final class Navigation {
    static let shared = Navigation()

    private(set) var history = History()

    /// The page shown, like `album:%7B…%7D`.
    var page: String { history.page }
    /// The page's section, like `album`.
    var section: String { PageID.section(of: page) }
    /// The page's argument; empty without one.
    var argument: String { PageID.argument(of: page) }
    var canGoBack: Bool { history.canGoBack }
    var canGoForward: Bool { history.canGoForward }

    /// What followed the `:` the last time each section showed, by section: each section has
    /// one live page, which shows this.
    private(set) var subpages: [String: String] = [:]
    /// The sections shown so far, in order: their pages stay alive, keeping their state.
    private(set) var visited: [String] = [PageID.tracks]
    /// Goes up when a page arrives with the arrival transition (fade + rise): another
    /// section, or another argument in any section but Settings.
    private(set) var arrival = 0

    // MARK: Window layout

    /// The main window's width, which the shell reports.
    var windowWidth: Double = 1280
    /// ≥ 1600: the Now Playing panel shows, except on the queue page (spec §1.4).
    var isWide: Bool { windowWidth >= 1600 }
    /// The Now Playing panel shows now.
    var nowPlayingPanelShown: Bool { isWide && section != PageID.queue }

    // MARK: Requests between parts of the window

    /// The New playlist dialog is open (sidebar "+", File › New Playlist, the track menu).
    var newPlaylistDialog: NewPlaylistRequest?
    /// Goes up to ask the Sources page to open its Add source menu (File › Add Source…).
    private(set) var addSourceRequest = 0
    /// Goes up to ask the Tracks page to focus its search (⌘F).
    private(set) var focusSearchRequest = 0
    /// The filter drawer (over the page area's right edge) and what it edits; `nil` while closed.
    var filterDrawer: FilterDrawerContext?
    /// The queue button pings in the accent for 750 ms after a track is queued out of sight.
    private(set) var queuePing = 0

    /// Whether the place in Up next of a track queued now shows on screen (first when `next`,
    /// else after the queued ones). The queue page and the Now Playing panel may refine it;
    /// by default it shows on the queue page and in the panel.
    @ObservationIgnored var upNextShows: (_ next: Bool) -> Bool = { _ in
        let navigation = Navigation.shared
        return navigation.section == PageID.queue || navigation.nowPlayingPanelShown
    }

    private init() {}

    // MARK: Moving

    /// Shows `page`, after the one shown in the history. `PageID.settings` opens the
    /// section Settings showed last.
    func go(_ page: String) {
        let to = page == PageID.settings ? PageID.settings(subpages["settings"] ?? "playback") : page
        guard PageID.sections.contains(PageID.section(of: to)) else { return }
        let before = history.page
        history.go(to)
        shown(before)
    }

    func back() {
        let before = history.page
        history.back()
        shown(before)
    }

    func forward() {
        let before = history.page
        history.forward()
        shown(before)
    }

    /// The queue page, or back from it (to Tracks without history), as the player bar's queue
    /// button does.
    func toggleQueue() {
        if page != PageID.queue {
            go(PageID.queue)
        } else if canGoBack {
            back()
        } else {
            go(PageID.tracks)
        }
    }

    /// Forgets the pages of deleted playlists; the store calls it when the playlists change.
    func prune(playlists: Set<PlaylistId>) {
        history.prune(playlists: playlists)
    }

    /// Forgets the pages of removed sources; the store calls it when the roots change.
    func prune(roots: [Locator]) {
        history.prune(roots: roots)
    }

    // MARK: Requests

    func requestNewPlaylist(with tracks: [Locator] = []) {
        newPlaylistDialog = NewPlaylistRequest(tracks: tracks)
    }

    func requestAddSource() {
        go(PageID.sources)
        addSourceRequest += 1
    }

    func requestFocusSearch() {
        if section != PageID.tracks { go(PageID.tracks) }
        focusSearchRequest += 1
    }

    /// Opens the filter drawer on Tracks (View › Filter).
    func openFilterDrawer(_ context: FilterDrawerContext = .tracks) {
        if case .tracks = context, section != PageID.tracks { go(PageID.tracks) }
        filterDrawer = context
    }

    /// Tells that `title` was queued (`next`: to play next), as the track menu does: a toast and
    /// a ping of the queue button when its place in Up next is out of sight. `count` > 1: `title`
    /// names several tracks ("3 tracks"), which "play next" in the plural.
    func queued(_ title: String, next: Bool, count: Int = 1) {
        guard !upNextShows(next) else { return }
        let text = if next && count > 1 {
            tr("plays_next_many", fallback: "%1 play next", title)
        } else {
            tr(next ? "plays_next" : "added_to_queue", title)
        }
        Toasts.shared.showQueued(text)
        queuePing += 1
    }

    // MARK: Private

    private func shown(_ before: String) {
        let page = history.page
        guard page != before else { return }
        let name = PageID.section(of: page)
        if page.contains(":") {
            subpages[name] = PageID.argument(of: page)
        }
        if !visited.contains(name) {
            visited.append(name)
        }
        // The queue shows where a queued track went.
        if name == PageID.queue {
            Toasts.shared.dismissQueued()
        }
        let beforeName = PageID.section(of: before)
        if name != beforeName || name != "settings" {
            arrival += 1
        }
    }
}

/// The New playlist dialog's request: the tracks the new playlist starts with (from the track
/// menu's Add to playlist › New playlist…), none from the sidebar or the File menu.
struct NewPlaylistRequest: Identifiable, Sendable {
    let id = UUID()
    var tracks: [Locator]
}
