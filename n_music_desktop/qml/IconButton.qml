import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A square button showing only an icon; `text` names it for assistive technology.
AbstractButton {
    id: button

    property string iconName
    property real size: 40
    property real iconSize: 20
    property real stroke: 1.8
    property color color: Theme.text2

    implicitWidth: size
    implicitHeight: size
    padding: 0
    hoverEnabled: true
    opacity: enabled ? 1 : 0.45
    Accessible.name: text

    background: Rectangle {
        radius: 8
        color: button.down ? Theme.selected : button.hovered ? Theme.raised : "transparent"
        border.width: button.visualFocus ? 2 : 0
        border.color: Theme.text
    }

    contentItem: Item {
        Icon {
            anchors.centerIn: parent
            name: button.iconName
            size: button.iconSize
            stroke: button.stroke
            color: button.color
        }
    }
}
