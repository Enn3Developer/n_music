import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A tooltip in the app's style, wrapping text too long for a line.
ToolTip {
    id: tip

    delay: 500
    topPadding: 5
    bottomPadding: 5
    leftPadding: 9
    rightPadding: 9

    contentItem: Label {
        text: tip.text
        wrapMode: Text.WrapAnywhere
        color: Theme.text
        font.pixelSize: 12
        font.weight: Font.Medium
    }
    background: Rectangle {
        radius: 6
        color: Theme.menu
        border.width: 1
        border.color: Theme.line2
    }
}
