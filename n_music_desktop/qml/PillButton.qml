import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A rounded action button; `primary` fills it with the accent.
AbstractButton {
    id: button

    property string iconName
    property bool primary: false

    implicitHeight: 40
    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    leftPadding: primary ? 18 : 16
    rightPadding: leftPadding
    hoverEnabled: true
    opacity: enabled ? 1 : 0.45
    font.pixelSize: 14
    font.weight: Font.DemiBold

    background: Rectangle {
        radius: height / 2
        color: {
            if (button.primary)
                return button.down ? Qt.darker(Theme.accent, 1.08) : button.hovered ? Qt.lighter(Theme.accent, 1.06) : Theme.accent;
            return button.down ? Theme.selected : button.hovered ? Theme.hover : "transparent";
        }
        border.width: button.visualFocus ? 2 : button.primary ? 0 : 1
        border.color: button.visualFocus ? Theme.text : Theme.border
    }

    contentItem: Row {
        spacing: 8

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            visible: button.iconName !== ""
            name: button.iconName
            size: 16
            stroke: 1.9
            color: button.primary ? Theme.accentInk : Theme.text
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            text: button.text
            font: button.font
            color: button.primary ? Theme.accentInk : Theme.text
        }
    }
}
