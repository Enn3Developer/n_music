pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// The tracks of a TrackList as a table: click selects, double-click or Enter plays from there.
Item {
    id: table

    required property TrackList list
    readonly property alias view: view
    /// Which value columns show, see TrackColumns.
    property string layout: "library"
    /// The playlist the list shows, whose tracks can be taken out of it; empty for none.
    property string playlistName

    /// Opens the menu of `row` from `x`, `y` of `item`, rightwards or leftwards.
    function openMenu(row: int, item: Item, x: real, y: real, leftwards: bool) {
        menu.show(row, item, leftwards ? x - menu.width : x, y);
    }

    TrackColumns {
        id: columns
        width: table.width - 32 - 24
        layout: table.layout
    }

    TrackHeader {
        id: header
        x: 16
        width: parent.width - 32
        list: table.list
        columns: columns
    }

    ListView {
        id: view
        anchors.top: header.bottom
        anchors.bottom: parent.bottom
        x: 16
        width: parent.width - 32
        bottomMargin: 12
        clip: true
        model: table.list
        reuseItems: true
        currentIndex: -1
        highlightMoveDuration: 0
        boundsBehavior: Flickable.StopAtBounds
        activeFocusOnTab: true
        Accessible.role: Accessible.Table
        Accessible.name: Tr.t.tracks

        delegate: TrackRow {
            width: ListView.view.width
            columns: columns
            selected: ListView.isCurrentItem
            menuOpen: menu.visible && menu.row === index
            onClicked: {
                view.currentIndex = index;
                view.forceActiveFocus();
            }
            onActivated: table.list.play(index)
            onMenuRequested: (item, x, y, leftwards) => table.openMenu(index, item, x, y, leftwards)
        }

        Keys.onReturnPressed: table.list.play(currentIndex)
        Keys.onEnterPressed: table.list.play(currentIndex)
        // The menu key, or Shift+F10, opens the menu of the selected track.
        Keys.onPressed: event => {
            const asked = event.key === Qt.Key_Menu || (event.key === Qt.Key_F10 && event.modifiers & Qt.ShiftModifier);
            if (!asked || currentItem === null)
                return;
            event.accepted = true;
            table.openMenu(currentIndex, currentItem, 48, currentItem.height / 2, false);
        }

        ScrollBar.vertical: ThinScrollBar {}
    }

    TrackMenu {
        id: menu
        list: table.list
        playlistName: table.playlistName
        onNewPlaylistRequested: row => {
            naming.row = row;
            naming.ask("");
        }
    }

    PromptDialog {
        id: naming

        property int row: -1

        title: Tr.t.new_playlist
        asksText: true
        placeholder: Tr.t.playlist_name
        confirmText: Tr.t.create
        onConfirmed: name => table.list.addToNewPlaylist(row, name)
    }
}
