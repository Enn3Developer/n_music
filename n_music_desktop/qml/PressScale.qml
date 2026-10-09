import QtQuick
import NMusic

// Scales a pressed control: down in 80 ms, back over 200 ms. For `scale`, as
// `PressScale on scale`.
Behavior {
    id: behavior

    ScaleAnimator {
        duration: behavior.targetValue < 1 ? Motion.exit : Motion.move
        easing.bezierCurve: Motion.standard
    }
}
