import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A button outlined with dashes, for adding something.
AbstractButton {
    id: button

    property string iconName
    property real radius: 8
    property color color: Theme.text

    implicitHeight: 34
    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    leftPadding: 12
    rightPadding: 12
    hoverEnabled: true
    opacity: enabled ? 1 : 0.45
    font.pixelSize: 13
    font.weight: Font.Medium

    background: Item {
        Rectangle {
            anchors.fill: parent
            radius: button.radius
            color: button.down ? Theme.selected : button.hovered ? Theme.hover : "transparent"
            border.width: button.visualFocus ? 2 : 0
            border.color: Theme.text
        }
        DashedFrame {
            anchors.fill: parent
            radius: button.radius
            color: button.hovered ? Theme.text3 : Theme.border
        }
    }

    contentItem: Row {
        spacing: 6

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            name: button.iconName
            size: 14
            stroke: 2
            color: button.color
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            text: button.text
            font: button.font
            color: button.color
        }
    }
}
