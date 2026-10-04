pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
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

    /// New playlist was picked for the track of `row`.
    signal newPlaylistRequested(int row)

    /// Opens for `row` at `x`, `y` of `item`.
    function show(row: int, item: Item, x: real, y: real) {
        menu.row = row;
        holding = list.playlistsWith(row);
        popup(item, x, y);
    }

    width: 248

    MenuEntry {
        text: Tr.t.play_next
        onTriggered: menu.list.enqueue(menu.row, true)
    }
    MenuEntry {
        text: Tr.t.add_to_queue
        onTriggered: menu.list.enqueue(menu.row, false)
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
        Line {}

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

    Line {
        shown: menu.playlistName !== ""
    }
    MenuEntry {
        shown: menu.playlistName !== ""
        danger: true
        text: Tr.t.remove_from_playlist.arg(menu.playlistName)
        onTriggered: menu.list.removeFromPlaylist(menu.row)
    }

    component Line: MenuSeparator {
        /// Takes no room while false, unlike `visible` alone in a menu.
        property bool shown: true

        visible: shown
        implicitHeight: shown ? implicitContentHeight + topPadding + bottomPadding : 0
        topPadding: 4
        bottomPadding: 4
        leftPadding: 6
        rightPadding: 6

        contentItem: Rectangle {
            implicitHeight: 1
            color: Theme.border
        }
    }
}
