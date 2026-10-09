import QtQuick
import NMusic

// Fades a background as the pointer comes and goes, as `TintFade on color`: in over 80 ms, out
// over 150 ms, going out when the colour gets more see-through. A tint fading to nothing goes to
// its own colour without alpha rather than to "transparent", which is black and would darken it
// on its way. Theme and accent changes pass straight through: the theme fades them already.
Behavior {
    id: behavior

    enabled: !Theme.changing

    ColorAnimation {
        duration: behavior.targetValue.a < behavior.targetProperty.object[behavior.targetProperty.name].a ? Motion.fade : Motion.exit
    }
}
