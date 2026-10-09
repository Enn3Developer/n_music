import QtQuick
import NMusic

// A note in a corner of the window, for a while: it fades in over 150 ms as it rises 8 px over
// 200 ms, and fades out in 80 ms. New words while it shows fade in where the old ones were. It
// stays `stay` ms, and while the pointer is over it, then 2.5 s more.
Rectangle {
    id: toast

    /// How long it stays, in ms.
    property int stay: 4000
    property bool shown: false
    /// What it holds, which fades in again for new words.
    default property alias body: holder.data

    /// Shows it, or shows new words in it, for `stay` ms.
    function pop() {
        if (shown) {
            renew.restart();
        } else {
            leave.stop();
            shown = true;
            arrive.restart();
        }
        timer.interval = stay;
        timer.restart();
    }

    function dismiss() {
        if (!shown)
            return;
        shown = false;
        timer.stop();
        arrive.stop();
        leave.restart();
    }

    visible: shown || leave.running
    opacity: 0
    transform: Translate {
        id: rise
    }

    // It holds while the pointer is over it.
    HoverHandler {
        id: hover
        onHoveredChanged: {
            if (hovered) {
                timer.stop();
            } else if (toast.shown) {
                timer.interval = 2500;
                timer.restart();
            }
        }
    }

    Timer {
        id: timer
        onTriggered: {
            if (!hover.hovered)
                toast.dismiss();
        }
    }

    ParallelAnimation {
        id: arrive

        OpacityAnimator {
            target: toast
            from: 0
            to: 1
            duration: Motion.fade
            easing.bezierCurve: Motion.standard
        }
        NumberAnimation {
            target: rise
            property: "y"
            from: 8 * Motion.travel
            to: 0
            duration: Motion.move
            easing.bezierCurve: Motion.standard
        }
    }
    OpacityAnimator {
        id: leave
        target: toast
        to: 0
        duration: Motion.exit
    }
    OpacityAnimator {
        id: renew
        target: holder
        from: 0
        to: 1
        duration: Motion.fade
    }

    Item {
        id: holder
        anchors.fill: parent
    }
}
