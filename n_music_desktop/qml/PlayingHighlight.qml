pragma ComponentBehavior: Bound
import QtQuick
import NMusic

// The playing row's ground in a list, drawn under its rows, with the playing bars. When another
// track starts it slides to its row over 200 ms, if that is six rows away or fewer; farther, or
// under reduced motion, it fades out at the old row and in at the new one over 150 ms. A sort or
// a search, or a track change within 250 ms of the one before, takes it there at once. While
// rows make way for one being dragged it rides along with `follow`, its row; while `hidden` it
// fades out, coming back over 150 ms after `revealDelay`.
Item {
    id: highlight

    /// The playing row; -1 when the list does not hold the current track.
    property int row: -1
    /// Where the first row's top is, and how far apart rows stand.
    property real firstTop: 0
    property real pitch: 52
    property real rowHeight: 52
    property real radius: 8
    /// Where the bars stand in the row; a width of 0 leaves them out.
    property real barsX: 0
    property real barsSize: 14
    /// The playing row's delegate to ride along with; null to stand at `row`.
    property Item follow: null
    property bool hidden: false
    /// The playing row is the one picked too: its ground is a step brighter.
    property bool raised: false
    /// How long it waits before it fades back in, in ms.
    property int revealDelay: 0

    /// The layer on the playing row; the other fades out where it was.
    property Layer front: first
    readonly property Layer back: front === first ? second : first
    /// The row it shows on, -1 for none.
    property int shown: -1
    /// A track change came since it last moved, and when the last two came.
    property bool armed: false
    property real changed: 0
    property real changedBefore: 0

    function yOf(row: int): real {
        return firstTop + row * pitch;
    }

    // Moves to `row`, once the player and the list both told of a change.
    function settle() {
        const from = shown, to = row;
        if (from === to)
            return;
        shown = to;
        const followsTrack = armed;
        armed = false;
        const quick = followsTrack && changed - changedBefore < Motion.quick;
        glide.stop();
        swap.stop();
        appear.stop();
        if (to < 0) {
            front.opacity = 0;
            return;
        }
        if (from < 0) {
            front.y = yOf(to);
            if (quick) {
                front.opacity = 1;
            } else {
                appear.target = front;
                appear.start();
            }
            return;
        }
        if (!followsTrack || quick) {
            front.y = yOf(to);
            front.opacity = 1;
            back.opacity = 0;
        } else if (Motion.reduced || Math.abs(to - from) > 6) {
            const old = front;
            front = back;
            front.y = yOf(to);
            fadeOut.target = old;
            fadeIn.target = front;
            swap.start();
        } else {
            glide.target = front;
            glide.to = yOf(to);
            glide.start();
        }
    }

    // Puts it on its row at once, as when rows change size.
    function snap() {
        glide.stop();
        if (shown >= 0)
            front.y = yOf(shown);
    }

    /// Tints the playing row for a moment, so the eye finds it again: it holds for 200 ms and
    /// fades over 400 ms.
    function tint() {
        flash.stop();
        flash.start();
    }

    onRowChanged: Qt.callLater(highlight.settle)
    onFirstTopChanged: snap()
    onPitchChanged: snap()
    // Back from riding along, it stands at its row.
    onFollowChanged: {
        if (follow === null)
            snap();
    }
    opacity: hidden ? 0 : 1

    Behavior on opacity {
        id: reveal

        SequentialAnimation {
            PauseAnimation {
                duration: reveal.targetValue > 0 ? highlight.revealDelay : 0
            }
            OpacityAnimator {
                duration: Motion.fade
            }
        }
    }

    Binding {
        target: highlight.front
        property: "y"
        value: highlight.follow ? highlight.follow.y : 0
        when: highlight.follow !== null && !glide.running
        restoreMode: Binding.RestoreNone
    }

    // A track change that moved no row leaves it at rest after a moment.
    Timer {
        id: disarm
        interval: 1000
        onTriggered: highlight.armed = false
    }
    Component.onCompleted: {
        shown = row;
        if (row >= 0) {
            front.y = yOf(row);
            front.opacity = 1;
        }
    }

    Connections {
        target: Player

        function onTrackChanging() {
            highlight.armed = true;
            disarm.restart();
            highlight.changedBefore = highlight.changed;
            highlight.changed = Date.now();
            Qt.callLater(highlight.settle);
        }
    }

    YAnimator {
        id: glide
        duration: Motion.move
        easing.bezierCurve: Motion.standard
    }
    OpacityAnimator {
        id: appear
        from: 0
        to: 1
        duration: Motion.fade
    }
    ParallelAnimation {
        id: swap

        OpacityAnimator {
            id: fadeOut
            to: 0
            duration: Motion.fade
        }
        OpacityAnimator {
            id: fadeIn
            from: 0
            to: 1
            duration: Motion.fade
        }
    }
    SequentialAnimation {
        id: flash

        PropertyAction {
            target: highlight.front.tint
            property: "opacity"
            value: 1
        }
        PauseAnimation {
            duration: Motion.move
        }
        OpacityAnimator {
            target: highlight.front.tint
            to: 0
            duration: 400
        }
    }

    Layer {
        id: first
    }
    Layer {
        id: second
    }

    component Layer: Rectangle {
        id: layer

        /// A sort's tint, over the ground.
        property alias tint: tint

        width: highlight.width
        height: highlight.rowHeight
        radius: highlight.radius
        color: highlight.raised ? Theme.raised : Theme.selected
        opacity: 0

        ColorFade on color {}

        Rectangle {
            id: tint
            anchors.fill: parent
            radius: highlight.radius
            color: Qt.alpha(Theme.accent, 0.12)
            opacity: 0
        }

        PlayingBars {
            x: highlight.barsX
            y: (highlight.rowHeight - height) / 2
            visible: highlight.barsX > 0
            playing: highlight.front === layer && Player.playing
            size: highlight.barsSize
            color: Theme.accentText
        }
    }
}
