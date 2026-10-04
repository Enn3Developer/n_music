import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A slim scroll bar, shown while scrolling or under the pointer.
ScrollBar {
    id: bar

    implicitWidth: 10
    padding: 2
    minimumSize: 0.04

    background: null
    contentItem: Rectangle {
        implicitWidth: 6
        radius: width / 2
        color: bar.pressed ? Theme.text3 : Theme.border
        opacity: bar.active || bar.hovered ? 1 : 0

        Behavior on opacity {
            NumberAnimation {
                duration: 150
            }
        }
    }
}
