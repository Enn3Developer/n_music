pragma Singleton
import QtQuick
import NMusic

// The interface strings by key, in the chosen language: `Tr.t.key`.
QtObject {
    readonly property var t: Translations.strings
}
