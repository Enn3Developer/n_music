import QtQuick
import NMusic

// Fades a colour of the theme to a new theme or accent over 250 ms, as `ThemeFade on bg`.
Behavior {
    ColorAnimation {
        duration: Motion.enter
    }
}
