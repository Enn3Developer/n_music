import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A line between groups of a PopupMenu's entries.
MenuSeparator {
    /// Takes no room while false, unlike `visible` alone in a menu.
    property bool shown: true

    visible: shown
    implicitHeight: shown ? implicitContentHeight + topPadding + bottomPadding : 0
    topPadding: 4
    bottomPadding: 4
    leftPadding: 6
    rightPadding: 6

    contentItem: Rectangle {
        implicitHeight: 1
        color: Theme.border
    }
}
