import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An item of the queue: cover, title and artist, length; queued ones can be taken out. The
// page's highlight grounds the current one and marks it playing. It and those still to play can
// be picked up with the mouse and dragged to another place, which a grip before them shows.
// Played rows grey over 150 ms; a row picked up lifts, standing out and growing to 102 %.
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
    /// It looks picked up: while dragged, and after it is let go until half way to its place.
    property bool lifted: dragged
    /// Back from the view's pool: what it shows changes at once, rather than fading.
    property bool pooled: false

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

    /// Tints the row for a moment, so the eye finds it: it holds for 200 ms and fades over
    /// 400 ms.
    function tint() {
        flash.restart();
    }

    implicitHeight: dense ? 42 : 54
    radius: 8
    z: lifted ? 3 : 1
    color: lifted ? Theme.raised : current ? Qt.alpha(Theme.hover, 0) : queued ? (mouse.containsMouse ? Theme.raised : Theme.surface) : mouse.containsMouse ? Theme.hover : Qt.alpha(Theme.hover, 0)
    border.width: 1
    border.color: lifted ? Theme.line2 : Qt.alpha(Theme.line2, 0)
    scale: lifted ? 1 + 0.02 * Motion.travel : 1
    transform: Translate {
        y: row.lift
    }
    // A row taken out faded; back from the pool it shows again.
    ListView.onPooled: pooled = true
    ListView.onReused: {
        opacity = 1;
        Qt.callLater(() => row.pooled = false);
    }

    TintFade on color {
        enabled: !row.pooled && !Theme.changing
    }
    ColorFade on border.color {
        enabled: !row.pooled && !Theme.changing
    }
    Behavior on scale {
        enabled: !row.pooled

        ScaleAnimator {
            duration: Motion.fade
            easing.bezierCurve: Motion.standard
        }
    }

    SequentialAnimation {
        id: flash

        PropertyAction {
            target: tinted
            property: "opacity"
            value: 1
        }
        PauseAnimation {
            duration: Motion.move
        }
        OpacityAnimator {
            target: tinted
            to: 0
            duration: 400
        }
    }

    Rectangle {
        id: tinted
        anchors.fill: parent
        radius: parent.radius
        color: Qt.alpha(Theme.accent, 0.12)
        opacity: 0
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

    // Played, it greys: the grip fades out, the cover to 60 %, the title to the quieter colour.
    Icon {
        x: 4
        anchors.verticalCenter: parent.verticalCenter
        opacity: row.movable ? 1 : 0
        name: "grip"
        size: 16
        color: row.dragged || mouse.containsMouse ? Theme.text2 : Theme.text3

        Behavior on opacity {
            enabled: !row.pooled

            OpacityAnimator {
                duration: Motion.fade
            }
        }

        HoverHandler {
            enabled: row.movable
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

        Behavior on opacity {
            enabled: !row.pooled

            OpacityAnimator {
                duration: Motion.fade
            }
        }
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

            ColorFade on color {
                enabled: !row.pooled && !Theme.changing
            }
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

        // The highlight draws them, but not under the row in hand.
        PlayingBars {
            anchors.centerIn: parent
            visible: row.current && row.lifted
            playing: row.current && Player.playing
            size: 14
            color: Theme.accentText
        }
        Item {
            anchors.centerIn: parent
            width: 14
            height: 14
            visible: row.current
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
