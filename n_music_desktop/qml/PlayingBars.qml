pragma ComponentBehavior: Bound
import QtQuick
import NMusic

// The equalizer icon's bars, bouncing while `playing` and easing back into the icon over 200 ms
// once it stops; drawn in `color` at `size` pixels, like an `Icon`. Each bar walks at random,
// easing to a new height every 180 to 320 ms. They hold still under reduced motion, and rest
// when the window has been in the background for 10 s.
Item {
    id: bars

    property bool playing: false
    property color color: Theme.text
    property real size: 18

    /// The window has been in the background for a while.
    property bool idle: false
    /// The bars bounce: playing, seen, and not held still.
    readonly property bool moving: playing && visible && !idle && !Motion.reduced && Window.visibility !== Window.Minimized && Window.visibility !== Window.Hidden

    implicitWidth: size
    implicitHeight: size

    Timer {
        running: !bars.Window.active
        interval: 10000
        onTriggered: bars.idle = true
        onRunningChanged: {
            if (!running)
                bars.idle = false;
        }
    }

    Bar {
        at: 4
        rest: 10
    }
    Bar {
        at: 10.25
        rest: 16
    }
    Bar {
        at: 16.5
        rest: 12
    }

    // A bar on the icon's grid of 24, standing on its baseline at 20: from 4 to 15 tall while it
    // bounces, `rest` when still.
    component Bar: Rectangle {
        id: bar

        /// Its left edge, and its height in the icon.
        required property real at
        required property real rest
        property real tall: rest

        readonly property real unit: bars.size / 24

        /// Eases to a new height, then to another.
        function bounce() {
            settle.stop();
            step.to = 4 + Math.random() * 11;
            step.duration = 180 + Math.random() * 140;
            step.restart();
        }

        function still() {
            step.stop();
            settle.restart();
        }

        x: at * unit
        y: (20 - tall) * unit
        width: 3.5 * unit
        height: tall * unit
        color: bars.color
        antialiasing: true
        Component.onCompleted: {
            if (bars.moving)
                bounce();
        }

        Connections {
            target: bars

            function onMovingChanged() {
                if (bars.moving)
                    bar.bounce();
                else
                    bar.still();
            }
        }

        NumberAnimation {
            id: step
            target: bar
            property: "tall"
            easing.type: Easing.InOutSine
            onFinished: {
                if (bars.moving)
                    bar.bounce();
                else
                    bar.still();
            }
        }
        NumberAnimation {
            id: settle
            target: bar
            property: "tall"
            to: bar.rest
            duration: Motion.move
            easing.bezierCurve: Motion.standard
        }
    }
}
