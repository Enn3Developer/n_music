import QtQuick

// Column widths of a track table `width` wide: fixed number and value columns, the rest
// shared by title and album.
QtObject {
    property real width

    readonly property real number: 44
    readonly property real year: 56
    readonly property real plays: 56
    readonly property real time: 60
    readonly property real more: 40
    readonly property real flexible: Math.max(0, width - number - year - plays - time - more)
    readonly property real title: Math.max(220, flexible * 2.2 / 3.6)
    readonly property real album: Math.max(140, flexible - title)
}
