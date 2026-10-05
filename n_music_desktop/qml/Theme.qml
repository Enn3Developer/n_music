pragma Singleton
import QtQuick
import NMusic

// Colours and type of the interface, dark or light.
QtObject {
    // The system's scheme decides when it is known; dark otherwise.
    readonly property bool dark: AppState.theme === 2
        || (AppState.theme === 0 && Application.styleHints.colorScheme !== Qt.ColorScheme.Light)

    readonly property string font: "Figtree"

    /// The accent colours to pick from, by hue: the fill, what goes on it, and text in the
    /// accent on light grounds, where the fill fails contrast. Dark grounds take the fill.
    readonly property var accents: [
        {
            name: "amber",
            fill: "#F0A13A",
            ink: "#1A1206",
            lightText: "#9A5800"
        },
        {
            name: "green",
            fill: "#4FC98E",
            ink: "#04150D",
            lightText: "#176E46"
        },
        {
            name: "teal",
            fill: "#3CC4C0",
            ink: "#031615",
            lightText: "#0D6C69"
        },
        {
            name: "blue",
            fill: "#6AA6FF",
            ink: "#06111F",
            lightText: "#1F5CC2"
        },
        {
            name: "violet",
            fill: "#A68BFA",
            ink: "#120A26",
            lightText: "#6B45D1"
        },
        {
            name: "rose",
            fill: "#F2779B",
            ink: "#22070F",
            lightText: "#B52A5B"
        }
    ]
    readonly property var preset: accents.find(accent => accent.name === AppState.accent) ?? accents[0]

    readonly property color accent: preset.fill
    readonly property color accentInk: preset.ink
    readonly property color accentText: dark ? accent : preset.lightText

    readonly property color bg: dark ? "#111316" : "#F6F7F9"
    readonly property color side: dark ? "#0C0E10" : "#ECEEF2"
    readonly property color panel: dark ? "#0E1013" : "#EEF0F3"
    readonly property color surface: dark ? "#16191D" : "#FFFFFF"
    readonly property color field: dark ? "#1A1D21" : "#FFFFFF"
    /// Inputs and selects on a surface.
    readonly property color input: dark ? "#1F2328" : "#FFFFFF"
    readonly property color raised: dark ? "#23272D" : "#E4E7EC"
    readonly property color hover: dark ? "#181B1F" : "#EEF0F3"
    readonly property color selected: dark ? "#1F2329" : "#E6EAF0"
    readonly property color menu: dark ? "#23272D" : "#FFFFFF"
    readonly property color menuHover: dark ? "#2F343C" : "#EEF0F3"
    readonly property color line: dark ? "#22262C" : "#DCE0E6"
    readonly property color line2: dark ? "#2A2E35" : "#D3D8DF"
    readonly property color border: dark ? "#343941" : "#C4CAD3"
    readonly property color track: dark ? "#2E333A" : "#D3D8DF"
    readonly property color shadow: dark ? "#8C000000" : "#33000000"

    readonly property color text: dark ? "#ECEEF1" : "#15171A"
    /// Text a step quieter, like a condition beside its field.
    readonly property color textSoft: dark ? "#C9CED6" : "#2F353D"
    readonly property color text2: dark ? "#A4ABB6" : "#4D5560"
    readonly property color text3: dark ? "#868E9A" : "#626A76"

    readonly property color danger: dark ? "#FF8A80" : "#B3261E"
    readonly property color dangerBg: dark ? "#2A1F1F" : "#FCEEEE"
    readonly property color dangerLine: dark ? "#5A3434" : "#E8B4B0"
    /// Text on a danger background.
    readonly property color dangerTitle: dark ? "#FFD9D5" : "#7A1C16"
    readonly property color dangerText: dark ? "#E2C4C0" : "#8C3B35"
    readonly property color success: dark ? "#7FD1AE" : "#1E7A55"
}
