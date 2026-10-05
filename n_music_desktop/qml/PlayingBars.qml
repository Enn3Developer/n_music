pragma ComponentBehavior: Bound
import QtQuick
import NMusic

// The equalizer icon's bars, bouncing while `playing` and settling back into the icon once it
// stops; drawn in `color` at `size` pixels, like an `Icon`.
Item {
    id: bars

    property bool playing: false
    property color color: Theme.text
    property real size: 18

    /// How far the bars stray from the icon: 1 while playing, easing back to 0 once stopped.
    property real energy: playing ? 1 : 0
    /// How far the motion is through its loop, from 0 to 1.
    property real phase: 0

    implicitWidth: size
    implicitHeight: size

    Behavior on energy {
        NumberAnimation {
            duration: 260
            easing.type: Easing.OutCubic
        }
    }

    // Nothing moves while the bars rest or cannot be seen.
    NumberAnimation on phase {
        running: bars.energy > 0 && bars.visible && bars.Window.visibility !== Window.Minimized && bars.Window.visibility !== Window.Hidden
        from: 0
        to: 1
        duration: 3000
        loops: Animation.Infinite
    }

    Bar {
        at: 4
        rest: 10
        fast: 5
        slow: 2
        shift: 0.15
    }
    Bar {
        at: 10.25
        rest: 16
        fast: 6
        slow: 3
        shift: 0.55
    }
    Bar {
        at: 16.5
        rest: 12
        fast: 4
        slow: 3
        shift: 0.8
    }

    // A bar on the icon's grid of 24, standing on its baseline at 20. It rides two waves that
    // fit the loop a whole number of times, so the loop never jumps.
    component Bar: Rectangle {
        /// Its left edge, and its height in the icon.
        required property real at
        required property real rest
        /// How many times each wave rises in a loop.
        required property int fast
        required property int slow
        /// Where in its waves it starts, in turns.
        required property real shift

        readonly property real unit: bars.size / 24
        readonly property real swing: 0.5 + 0.3 * Math.sin(2 * Math.PI * (fast * bars.phase + shift)) + 0.2 * Math.sin(2 * Math.PI * (slow * bars.phase + 2 * shift))
        /// From 3 to 16 tall while playing, `rest` when still.
        readonly property real tall: rest + bars.energy * (3 + 13 * swing - rest)

        x: at * unit
        y: (20 - tall) * unit
        width: 3.5 * unit
        height: tall * unit
        color: bars.color
        antialiasing: true
    }
}
