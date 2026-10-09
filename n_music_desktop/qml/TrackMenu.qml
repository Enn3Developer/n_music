pragma ComponentBehavior: Bound
import QtQuick
import NMusic

// What can be done with a track of a list: queue it, add it to a playlist, or take it out of
// the playlist listed.
PopupMenu {
    id: menu

    required property TrackList list
    /// The playlist the list shows, offering to take the track out of it; empty for none.
    property string playlistName
    /// The row the menu is for.
    property int row: -1
    /// The playlists holding the track already.
    property var holding: []
    /// The track and what it belongs to, see `TrackList.about`.
    property var about: ({})
    /// The pages of the track's album and artist, empty when unknown or listed already.
    readonly property string albumPage: about.album ? unlisted(Filters.collectionPage("album", about.album, about.albumArtist)) : ""
    readonly property string artistPage: about.artist ? unlisted(Filters.collectionPage("artist", about.artist, "")) : ""

    /// New playlist was picked for the track of `row`.
    signal newPlaylistRequested(int row)
    /// Asks to show `page`.
    signal navigate(string page)

    function unlisted(page: string): string {
        return page === list.origin ? "" : page;
    }

    /// Opens for `row` at `x`, `y` of `item`.
    function show(row: int, item: Item, x: real, y: real) {
        menu.row = row;
        holding = list.playlistsWith(row);
        about = list.about(row);
        popup(item, x, y);
    }

    width: 248

    MenuEntry {
        text: Tr.t.play_next
        onTriggered: {
            menu.list.enqueue(menu.row, true);
            Shell.queued(menu.about.title ?? "", true);
        }
    }
    MenuEntry {
        text: Tr.t.add_to_queue
        onTriggered: {
            menu.list.enqueue(menu.row, false);
            Shell.queued(menu.about.title ?? "", false);
        }
    }

    PopupMenu {
        id: targets
        title: Tr.t.add_to_playlist
        width: 220

        MenuEntry {
            iconName: "plus"
            text: Tr.t.new_playlist_ellipsis
            onTriggered: menu.newPlaylistRequested(menu.row)
        }
        MenuLine {}

        Instantiator {
            model: Playlists.items.filter(playlist => !playlist.smart)
            delegate: MenuEntry {
                required property var modelData

                text: modelData.name
                checked: menu.holding.includes(modelData.id)
                enabled: !checked
                onTriggered: menu.list.addToPlaylist(menu.row, modelData.id)
            }
            // After New playlist and the line.
            onObjectAdded: (index, entry) => targets.insertItem(index + 2, entry)
            onObjectRemoved: (index, entry) => targets.removeItem(entry)
        }

        MenuEntry {
            note: true
            text: Tr.t.smart_playlists_note
        }
    }

    MenuLine {
        shown: menu.albumPage !== "" || menu.artistPage !== ""
    }
    MenuEntry {
        shown: menu.albumPage !== ""
        text: Tr.t.go_to_album
        onTriggered: menu.navigate(menu.albumPage)
    }
    MenuEntry {
        shown: menu.artistPage !== ""
        text: Tr.t.go_to_artist
        onTriggered: menu.navigate(menu.artistPage)
    }

    MenuLine {
        shown: menu.playlistName !== ""
    }
    MenuEntry {
        shown: menu.playlistName !== ""
        danger: true
        text: Tr.t.remove_from_playlist.arg(menu.playlistName)
        onTriggered: menu.list.removeFromPlaylist(menu.row)
    }
}
