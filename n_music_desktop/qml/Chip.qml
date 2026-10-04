import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A small rounded label for a fact, like the audio format.
Label {
    height: 26
    leftPadding: 10
    rightPadding: 10
    verticalAlignment: Text.AlignVCenter
    color: Theme.text2
    font.pixelSize: 12

    background: Rectangle {
        radius: 13
        color: Theme.field
        border.width: 1
        border.color: Theme.line2
    }
}
