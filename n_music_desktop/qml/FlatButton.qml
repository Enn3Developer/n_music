import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A quiet text button with an optional icon; `filled` gives it a surface.
AbstractButton {
    id: button

    property string iconName
    property bool filled: false
    property color color: Theme.text2

    implicitHeight: 32
    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    leftPadding: 12
    rightPadding: 12
    hoverEnabled: true
    opacity: enabled ? 1 : 0.45
    font.pixelSize: 13
    font.weight: Font.Medium

    background: Rectangle {
        radius: 8
        color: button.down ? Theme.selected : button.hovered ? Theme.raised : button.filled ? Theme.surface : Qt.alpha(Theme.raised, 0)
        border.width: button.visualFocus ? 2 : 0
        border.color: Theme.text

        TintFade on color {}
    }

    contentItem: Row {
        spacing: 8

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            visible: button.iconName !== ""
            name: button.iconName
            size: 14
            stroke: 2
            color: button.color
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            text: button.text
            font: button.font
            color: button.hovered ? Theme.text : button.color

            ColorFade on color {}
        }
    }
}
