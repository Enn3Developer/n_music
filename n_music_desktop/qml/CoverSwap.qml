import QtQuick
import NMusic

// The current track's cover. Once a new track's cover has loaded it fades in over 200 ms on top
// of the old one, which stays whole under it until covered, so the ground never shows through.
// A change within 250 ms of the one before swaps the new cover in at once. Under reduced motion
// the fade takes 150 ms.
Item {
    id: swap

    property real size: 36
    property real radius: 4

    /// The cover showing the current track; the other holds the one before.
    property Cover front: first
    readonly property Cover back: front === first ? second : first
    /// When the last change came, in ms.
    property real changed: 0
    /// The new cover waits to load before it shows: at once when `quick`, else fading in.
    property bool waiting: false
    property bool quick: false
    readonly property bool ready: front.shown || front.missing

    function change() {
        const now = Date.now();
        quick = now - changed < Motion.quick;
        changed = now;
        reveal.stop();
        const leaving = front, coming = back;
        // Still the old track's: the player tells the new one right after.
        leaving.path = Player.cover;
        leaving.opacity = 1;
        leaving.z = 0;
        coming.path = Qt.binding(() => Player.cover);
        coming.opacity = 0;
        coming.z = 1;
        front = coming;
        waiting = true;
        late.restart();
    }

    /// Shows the new cover once it has loaded, or `anyway`.
    function show(anyway: bool) {
        if (!waiting || !(ready || anyway))
            return;
        waiting = false;
        late.stop();
        if (quick) {
            front.opacity = 1;
            back.opacity = 0;
        } else {
            reveal.start();
        }
    }

    implicitWidth: size
    implicitHeight: size
    // Checked again later: the cover may change again before then.
    onReadyChanged: {
        if (ready)
            Qt.callLater(swap.show, false);
    }

    Connections {
        target: Player

        function onTrackChanging() {
            swap.change();
            // A cover in the cache is there already.
            Qt.callLater(swap.show, false);
        }
    }

    // A cover slow to load fades in as it comes.
    Timer {
        id: late
        interval: 300
        onTriggered: swap.show(true)
    }

    SequentialAnimation {
        id: reveal

        OpacityAnimator {
            target: swap.front
            from: 0
            to: 1
            duration: Motion.reduced ? Motion.fade : Motion.move
        }
        ScriptAction {
            script: swap.back.opacity = 0
        }
    }

    Cover {
        id: first
        anchors.fill: parent
        size: swap.size
        radius: swap.radius
        path: Player.cover
    }
    Cover {
        id: second
        anchors.fill: parent
        size: swap.size
        radius: swap.radius
        opacity: 0
    }
}
