import QtQuick
import NMusic

// Fades a hover tint: in over 80 ms, out over 150 ms. For `opacity`, as `HoverFade on opacity`.
Behavior {
    id: behavior

    OpacityAnimator {
        duration: behavior.targetValue > 0 ? Motion.exit : Motion.fade
    }
}
