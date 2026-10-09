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

    /// Asks to show `page`, like a track's album.
    signal navigate(string page)

    /// Room left and right of the rows.
    readonly property real inset: Shell.narrow ? 6 : Shell.compact ? 10 : 16

    /// Opens the menu of `row` from `x`, `y` of `item`, rightwards or leftwards.
    function openMenu(row: int, item: Item, x: real, y: real, leftwards: bool) {
        menu.transformOrigin = leftwards ? Popup.TopRight : Popup.TopLeft;
        menu.show(row, item, leftwards ? x - menu.width : x, y);
    }

    /// The list was sorted anew and waits for its rows.
    property bool sorting: false

    // A sort shows the new order at once: the rows fade up from 60 % and the playing row is
    // tinted for a moment, so the eye finds it again.
    function sorted() {
        if (!sorting)
            return;
        sorting = false;
        settle.stop();
        sortFade.restart();
        highlight.tint();
    }

    TrackColumns {
        id: columns
        width: table.width - 2 * table.inset - 2 * columns.padding
        layout: table.layout
        compact: Shell.compact
        narrow: Shell.narrow
        dense: AppState.compactRows
        hidden: AppState.hiddenColumns
    }

    Connections {
        target: table.list

        function onSortChanged() {
            table.sorting = true;
            settle.restart();
        }
        function onDataChanged() {
            table.sorted();
        }
        function onModelReset() {
            table.sorted();
        }
    }

    // A sort that leaves the order as it was brings no rows.
    Timer {
        id: settle
        interval: 1500
        onTriggered: table.sorting = false
    }

    // Compact and narrow windows leave the column titles out.
    TrackHeader {
        id: header
        x: table.inset
        width: parent.width - 2 * table.inset
        height: visible ? implicitHeight : 0
        visible: !columns.slim
        list: table.list
        columns: columns
    }

    ListView {
        id: view
        anchors.top: header.bottom
        anchors.bottom: parent.bottom
        x: table.inset
        width: parent.width - 2 * table.inset
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

        PlayingHighlight {
            id: highlight
            // Under the rows, scrolling with them.
            parent: view.contentItem
            z: -1
            width: view.width
            row: table.list.currentRow
            raised: view.currentIndex >= 0 && view.currentIndex === table.list.currentRow
            pitch: columns.rowHeight
            rowHeight: columns.rowHeight
            // Where the number would be.
            barsX: columns.number > 0 ? columns.padding + columns.number - 28 : 0
        }

        OpacityAnimator {
            id: sortFade
            target: view.contentItem
            from: 0.6
            to: 1
            duration: Motion.fade
        }

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
        onNavigate: to => table.navigate(to)
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
