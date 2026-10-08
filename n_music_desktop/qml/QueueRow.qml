import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An item of the queue: cover, title and artist, length; queued ones can be taken out, the
// current one shows it plays. It and those still to play can be picked up with the mouse and
// dragged to another place, which a grip before them shows.
Rectangle {
    id: row

    required property int index
    required property string title
    required property string artist
    required property string length
    required property string cover
    required property string section
    required property bool queued
    required property bool current
    /// It is being dragged, shown `lift` away from its place.
    property bool dragged: false
    property real lift: 0

    signal activated
    signal remove
    /// It was picked up `offset` below its top, the pointer then at `sceneY`.
    signal pickedUp(real offset, real sceneY)
    /// The pointer carrying it moved to `sceneY`.
    signal carried(real sceneY)
    signal dropped
    /// The drag ended without a drop.
    signal dragCanceled

    /// Smaller covers and tighter rows, from the settings.
    readonly property bool dense: AppState.compactRows
    /// One dragged among those played stays held.
    readonly property bool movable: section === "next" || dragged

    implicitHeight: dense ? 42 : 54
    radius: 8
    z: dragged ? 3 : 1
    color: dragged ? Theme.raised : current ? Theme.selected : queued ? (mouse.containsMouse ? Theme.raised : Theme.surface) : mouse.containsMouse ? Theme.hover : "transparent"
    border.width: dragged ? 1 : 0
    border.color: Theme.line2
    transform: Translate {
        y: row.lift
    }

    MouseArea {
        id: mouse

        /// Where the press went down in the row.
        property real pressY: 0
        /// This press picked the row up, whether or not the drag went on.
        property bool lifted: false
        /// Touch scrolls the list rather than dragging rows.
        property bool touch: false

        anchors.fill: parent
        hoverEnabled: true
        // The list would take a drag over as a scroll.
        preventStealing: row.movable && !touch
        cursorShape: row.dragged ? Qt.ClosedHandCursor : Qt.ArrowCursor
        onPressed: event => {
            mouse.touch = event.source !== Qt.MouseEventNotSynthesized;
            mouse.pressY = event.y;
            mouse.lifted = false;
        }
        onPositionChanged: event => {
            if (!mouse.pressed || !row.movable || mouse.touch)
                return;
            const sceneY = mouse.mapToItem(null, event.x, event.y).y;
            if (mouse.lifted) {
                if (row.dragged)
                    row.carried(sceneY);
            } else if (Math.abs(event.y - mouse.pressY) >= Application.styleHints.startDragDistance) {
                mouse.lifted = true;
                row.pickedUp(mouse.pressY, sceneY);
            }
        }
        onReleased: {
            if (row.dragged)
                row.dropped();
        }
        onCanceled: {
            if (row.dragged)
                row.dragCanceled();
        }
        onDoubleClicked: row.activated()
    }

    Icon {
        x: 4
        anchors.verticalCenter: parent.verticalCenter
        visible: row.movable
        name: "grip"
        size: 16
        color: row.dragged || mouse.containsMouse ? Theme.text2 : Theme.text3

        HoverHandler {
            cursorShape: row.dragged ? Qt.ClosedHandCursor : Qt.OpenHandCursor
        }
    }
    Cover {
        id: art
        x: 24
        anchors.verticalCenter: parent.verticalCenter
        size: row.dense ? 32 : 40
        path: row.cover
        opacity: row.section === "history" ? 0.6 : 1
    }
    Column {
        anchors.left: art.right
        anchors.leftMargin: 12
        anchors.right: time.left
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        spacing: row.dense ? 0 : 2

        Label {
            width: parent.width
            text: row.title
            elide: Text.ElideRight
            color: row.current ? Theme.accentText : row.section === "history" ? Theme.text2 : Theme.text
            font.pixelSize: 14
            font.weight: Font.DemiBold
        }
        Label {
            width: parent.width
            text: row.artist === "" ? Tr.t.unknown_artist : row.artist
            elide: Text.ElideRight
            color: row.artist === "" ? Theme.text3 : Theme.text2
            font.pixelSize: row.dense ? 12 : 13
        }
    }
    Label {
        id: time
        anchors.right: action.left
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        width: 52
        horizontalAlignment: Text.AlignRight
        text: row.length
        color: Theme.text2
        font.pixelSize: 13
        font.features: {
            "tnum": 1
        }
    }
    Item {
        id: action
        anchors.right: parent.right
        anchors.rightMargin: 8
        width: 36
        height: parent.height

        PlayingBars {
            anchors.centerIn: parent
            visible: row.current
            playing: row.current && Player.playing
            size: 14
            color: Theme.accentText
            Accessible.name: Tr.t.now_playing
        }
        IconButton {
            anchors.centerIn: parent
            visible: row.queued && !row.current
            size: 32
            iconSize: 14
            stroke: 2.2
            iconName: "close"
            color: Theme.text3
            text: Tr.t.remove_from_up_next.arg(row.title)
            onClicked: row.remove()
        }
    }
}
