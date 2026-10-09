import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A modal dialog in the middle of the window, over a scrim. It fades in over 150 ms as it grows
// from 96 % and rises 12 px over 250 ms, the scrim fading in over 150 ms; it closes in 80 ms.
Popup {
    id: dialog

    /// How far below its place it shows while it opens.
    property real lift: 0

    parent: Overlay.overlay
    anchors.centerIn: parent
    modal: true
    // The ground and what it holds move together; the dialog keeps its place and size.
    topInset: lift
    bottomInset: -lift
    topPadding: padding + lift
    bottomPadding: padding - lift

    enter: Transition {
        NumberAnimation {
            property: "opacity"
            from: 0
            to: 1
            duration: Motion.fade
            easing.bezierCurve: Motion.standard
        }
        NumberAnimation {
            property: "scale"
            from: 1 - 0.04 * Motion.travel
            to: 1
            duration: Motion.enter
            easing.bezierCurve: Motion.standard
        }
        NumberAnimation {
            property: "lift"
            from: 12 * Motion.travel
            to: 0
            duration: Motion.enter
            easing.bezierCurve: Motion.standard
        }
    }
    exit: Transition {
        NumberAnimation {
            property: "opacity"
            to: 0
            duration: Motion.exit
        }
        NumberAnimation {
            property: "scale"
            to: 1 - 0.04 * Motion.travel
            duration: Motion.exit
            easing.bezierCurve: Motion.leaving
        }
        NumberAnimation {
            property: "lift"
            to: 12 * Motion.travel
            duration: Motion.exit
            easing.bezierCurve: Motion.leaving
        }
    }

    // The popup turns it fully on as it opens and off as it closes.
    Overlay.modal: Rectangle {
        color: Theme.dark ? "#99000000" : "#55000000"

        Behavior on opacity {
            id: dim

            OpacityAnimator {
                duration: dim.targetValue > 0 ? Motion.fade : Motion.exit
            }
        }
    }

    background: Rectangle {
        radius: 12
        color: Theme.surface
        border.width: 1
        border.color: Theme.line2
    }
}
