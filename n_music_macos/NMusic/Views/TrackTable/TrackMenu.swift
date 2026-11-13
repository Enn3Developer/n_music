import AppKit

/// The track menu (spec §3.2, `qml/TrackMenu.qml`), native: Play next, Add to queue, Add to
/// playlist › (New playlist…, the regular playlists, checked and disabled when they hold the
/// tracks already, the smart playlists note), Go to album, Go to artist, and on playlist pages
/// Remove from “name”. For several tracks it acts on all of them (Go to entries only for one).
///
/// Any list may use it: `TrackMenu.fill(menu, tracks:…)` in an `NSMenuDelegate`, or
/// `TrackMenu.make(…)` for a menu of its own.
@MainActor
enum TrackMenu {
    static func make(tracks: [TrackRow], origin: String, playlist: (id: PlaylistId, name: String)? = nil) -> NSMenu {
        let menu = NSMenu()
        fill(menu, tracks: tracks, origin: origin, playlist: playlist)
        return menu
    }

    static func fill(_ menu: NSMenu, tracks: [TrackRow], origin: String, playlist: (id: PlaylistId, name: String)?) {
        menu.removeAllItems()
        menu.autoenablesItems = false
        menu.minimumWidth = 248
        guard !tracks.isEmpty else { return }
        let store = Store.shared
        let locators = tracks.map(\.locator)
        let title = tracks.count == 1 ? tracks[0].title : Format.count(tracks.count, "track_one", "tracks_many")

        menu.addItem(TrackActionItem(tr("play_next")) {
            store.enqueue(locators, next: true, title: title)
        })
        menu.addItem(TrackActionItem(tr("add_to_queue")) {
            store.enqueue(locators, next: false, title: title)
        })

        // Add to playlist ›
        let targets = NSMenu()
        targets.autoenablesItems = false
        targets.minimumWidth = 220
        let new = TrackActionItem(tr("new_playlist_ellipsis")) {
            Navigation.shared.requestNewPlaylist(with: locators)
        }
        new.image = NSImage(systemSymbolName: "plus", accessibilityDescription: nil)
        targets.addItem(new)
        targets.addItem(.separator())
        var playlistItems: [(PlaylistId, NSMenuItem)] = []
        for summary in store.playlists where !summary.smart {
            let id = summary.id
            let item = TrackActionItem(summary.name) {
                store.addToPlaylist(id, tracks: locators)
            }
            targets.addItem(item)
            playlistItems.append((id, item))
        }
        let note = NSMenuItem(title: "", action: nil, keyEquivalent: "")
        note.attributedTitle = NSAttributedString(string: tr("smart_playlists_note"), attributes: [
            .font: NSFont.systemFont(ofSize: 12),
            .foregroundColor: NSColor(Theme.text3),
        ])
        note.isEnabled = false
        targets.addItem(note)
        let add = NSMenuItem(title: tr("add_to_playlist"), action: nil, keyEquivalent: "")
        add.submenu = targets
        menu.addItem(add)
        if !playlistItems.isEmpty {
            markHolding(playlistItems, locators: locators)
        }

        // Go to album / artist.
        if tracks.count == 1 {
            let track = tracks[0]
            var places: [NSMenuItem] = []
            if let album = track.album, !album.isEmpty {
                let page = PageID.album(name: album, artist: track.albumArtist ?? track.artists.first)
                if page != origin {
                    places.append(TrackActionItem(tr("go_to_album")) { Navigation.shared.go(page) })
                }
            }
            if let artist = track.artists.first, !artist.isEmpty {
                let page = PageID.artist(artist)
                if page != origin {
                    places.append(TrackActionItem(tr("go_to_artist")) { Navigation.shared.go(page) })
                }
            }
            if !places.isEmpty {
                menu.addItem(.separator())
                places.forEach(menu.addItem)
            }
        }

        if let playlist {
            menu.addItem(.separator())
            let remove = TrackActionItem(tr("remove_from_playlist", playlist.name)) {
                store.removeFromPlaylist(playlist.id, tracks: locators)
            }
            remove.attributedTitle = NSAttributedString(string: remove.title, attributes: [
                .font: NSFont.menuFont(ofSize: 0),
                .foregroundColor: NSColor(Theme.danger),
            ])
            menu.addItem(remove)
        }
    }

    /// Checks (and disables) the playlists holding every one of `locators`, once read.
    private static func markHolding(_ items: [(PlaylistId, NSMenuItem)], locators: [Locator]) {
        let ids = items.map(\.0)
        Task {
            let holding: Set<PlaylistId> = await Store.shared.query { core in
                if locators.count == 1 {
                    return Set(core.playlistsWith(locator: locators[0]))
                }
                return Set(ids.filter { core.playlistHolds(id: $0, tracks: locators) >= UInt32(locators.count) })
            }
            for (id, item) in items where holding.contains(id) {
                item.state = .on
                item.isEnabled = false
            }
        }
    }
}

/// A menu item running a closure.
final class TrackActionItem: NSMenuItem {
    private let handler: @MainActor () -> Void

    init(_ title: String, handler: @escaping @MainActor () -> Void) {
        self.handler = handler
        super.init(title: title, action: #selector(run), keyEquivalent: "")
        target = self
    }

    required init(coder: NSCoder) { fatalError() }

    @objc private func run() {
        // AppKit runs menu actions on the main thread.
        let handler = self.handler
        MainActor.assumeIsolated { handler() }
    }
}
