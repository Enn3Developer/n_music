import QtQuick

// Column widths of a track table `width` wide: fixed number and value columns, the rest
// shared by title and album. Columns a layout leaves out are 0 wide.
QtObject {
    property real width
    /// `library` shows year and plays, `playlist` when tracks were added, `smart` plays and
    /// when they were last played.
    property string layout: "library"

    readonly property real number: 44
    readonly property real year: layout === "library" ? 56 : 0
    readonly property real plays: layout === "library" ? 56 : layout === "smart" ? 64 : 0
    readonly property real lastPlayed: layout === "smart" ? 120 : 0
    readonly property real added: layout === "playlist" ? 110 : 0
    readonly property real time: 60
    readonly property real more: 40
    readonly property real flexible: Math.max(0, width - number - year - plays - lastPlayed - added - time - more)
    readonly property real title: Math.max(220, flexible * 2.2 / 3.6)
    readonly property real album: Math.max(140, flexible - title)
}
