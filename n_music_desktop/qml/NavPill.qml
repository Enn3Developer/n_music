import QtQuick
import NMusic

// The ground under the active entry of a group of the navigation, one per group. It slides
// between the group's entries over 200 ms. When the page moves to another group it fades out
// where it is, and the other group's fades in at its entry from 96 %. Under reduced motion it
// moves at once and fades in.
Rectangle {
    id: pill

    /// The active entry's place in the group; -1 when the page belongs to none of them.
    property int index: -1
    /// How far apart the entries stand.
    property real pitch: 38

    /// The place it showed at last, or -1 while out.
    property int shown: -1

    function place() {
        if (index >= 0 && shown >= 0) {
            if (Motion.reduced) {
                glide.stop();
                y = index * pitch;
                arrive.restart();
            } else {
                glide.to = index * pitch;
                glide.restart();
            }
        } else if (index >= 0) {
            glide.stop();
            leave.stop();
            y = index * pitch;
            arrive.restart();
        } else if (shown >= 0) {
            arrive.stop();
            leave.restart();
        }
        shown = index;
    }

    width: parent.width
    radius: 8
    color: Theme.raised
    opacity: 0
    scale: 0.96
    onIndexChanged: place()
    // Laid out once, it shows at its entry at once.
    Component.onCompleted: {
        if (index >= 0) {
            y = index * pitch;
            opacity = 1;
            scale = 1;
        }
        shown = index;
    }

    YAnimator {
        id: glide
        target: pill
        duration: Motion.move
        easing.bezierCurve: Motion.standard
    }
    ParallelAnimation {
        id: arrive

        OpacityAnimator {
            target: pill
            from: 0
            to: 1
            duration: Motion.fade
            easing.bezierCurve: Motion.standard
        }
        ScaleAnimator {
            target: pill
            from: Motion.reduced ? 1 : 0.96
            to: 1
            duration: Motion.move
            easing.bezierCurve: Motion.standard
        }
    }
    ParallelAnimation {
        id: leave

        OpacityAnimator {
            target: pill
            to: 0
            duration: Motion.exit
        }
        ScaleAnimator {
            target: pill
            to: Motion.reduced ? 1 : 0.96
            duration: Motion.move
            easing.bezierCurve: Motion.standard
        }
    }
}
