pragma Singleton
import QtQuick
import NMusic

// Colours and type of the interface, dark or light. A new theme or accent fades every colour
// over 250 ms.
QtObject {
    id: theme

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

    property color accent: preset.fill
    property color accentInk: preset.ink
    property color accentText: dark ? preset.fill : preset.lightText

    property color bg: dark ? "#111316" : "#F6F7F9"
    property color side: dark ? "#0C0E10" : "#ECEEF2"
    property color panel: dark ? "#0E1013" : "#EEF0F3"
    property color surface: dark ? "#16191D" : "#FFFFFF"
    property color field: dark ? "#1A1D21" : "#FFFFFF"
    /// Inputs and selects on a surface.
    property color input: dark ? "#1F2328" : "#FFFFFF"
    property color raised: dark ? "#23272D" : "#E4E7EC"
    property color hover: dark ? "#181B1F" : "#EEF0F3"
    property color selected: dark ? "#1F2329" : "#E6EAF0"
    property color menu: dark ? "#23272D" : "#FFFFFF"
    property color menuHover: dark ? "#2F343C" : "#EEF0F3"
    property color line: dark ? "#22262C" : "#DCE0E6"
    property color line2: dark ? "#2A2E35" : "#D3D8DF"
    property color border: dark ? "#343941" : "#C4CAD3"
    property color track: dark ? "#2E333A" : "#D3D8DF"
    property color shadow: dark ? "#8C000000" : "#33000000"

    property color text: dark ? "#ECEEF1" : "#15171A"
    /// Text a step quieter, like a condition beside its field.
    property color textSoft: dark ? "#C9CED6" : "#2F353D"
    property color text2: dark ? "#A4ABB6" : "#4D5560"
    property color text3: dark ? "#868E9A" : "#626A76"

    property color danger: dark ? "#FF8A80" : "#B3261E"
    property color dangerBg: dark ? "#2A1F1F" : "#FCEEEE"
    property color dangerLine: dark ? "#5A3434" : "#E8B4B0"
    /// Text on a danger background.
    property color dangerTitle: dark ? "#FFD9D5" : "#7A1C16"
    property color dangerText: dark ? "#E2C4C0" : "#8C3B35"
    property color success: dark ? "#7FD1AE" : "#1E7A55"

    /// The colours are fading to a new theme or accent.
    property bool changing: false
    onDarkChanged: startFading()
    onPresetChanged: startFading()

    function startFading() {
        changing = true;
        settle.restart();
    }

    property Timer settle: Timer {
        interval: Motion.enter
        onTriggered: theme.changing = false
    }

    // Each colour fades on its own: none derives from another, so none chases another.
    ThemeFade on accent {}
    ThemeFade on accentInk {}
    ThemeFade on accentText {}
    ThemeFade on bg {}
    ThemeFade on side {}
    ThemeFade on panel {}
    ThemeFade on surface {}
    ThemeFade on field {}
    ThemeFade on input {}
    ThemeFade on raised {}
    ThemeFade on hover {}
    ThemeFade on selected {}
    ThemeFade on menu {}
    ThemeFade on menuHover {}
    ThemeFade on line {}
    ThemeFade on line2 {}
    ThemeFade on border {}
    ThemeFade on track {}
    ThemeFade on shadow {}
    ThemeFade on text {}
    ThemeFade on textSoft {}
    ThemeFade on text2 {}
    ThemeFade on text3 {}
    ThemeFade on danger {}
    ThemeFade on dangerBg {}
    ThemeFade on dangerLine {}
    ThemeFade on dangerTitle {}
    ThemeFade on dangerText {}
    ThemeFade on success {}
}
