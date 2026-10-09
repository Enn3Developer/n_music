import QtQuick
import NMusic

// Fades a colour changing in place over 150 ms, as `ColorFade on color`. Theme and accent
// changes pass straight through: the theme fades them already.
Behavior {
    enabled: !Theme.changing

    ColorAnimation {
        duration: Motion.fade
    }
}
