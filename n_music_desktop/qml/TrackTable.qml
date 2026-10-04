pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// The tracks of a TrackList as a table: click selects, double-click or Enter plays from there.
Item {
    id: table

    required property TrackList list
    readonly property alias view: view

    TrackColumns {
        id: columns
        width: table.width - 32 - 24
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
            onClicked: {
                view.currentIndex = index;
                view.forceActiveFocus();
            }
            onActivated: table.list.play(index)
        }

        Keys.onReturnPressed: table.list.play(currentIndex)
        Keys.onEnterPressed: table.list.play(currentIndex)

        ScrollBar.vertical: ThinScrollBar {}
    }
}
