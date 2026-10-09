import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A rounded action button; `primary` fills it with the accent, `danger` warns, and `small`
// fits it in a row of settings. The pointer tints it, and pressing shrinks it a little.
AbstractButton {
    id: button

    property string iconName
    property bool primary: false
    property bool danger: false
    property bool small: false

    implicitHeight: small ? 36 : 40
    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    // Round around an icon alone.
    leftPadding: text === "" ? (implicitHeight - 16) / 2 : small ? 14 : primary ? 18 : 16
    rightPadding: leftPadding
    hoverEnabled: true
    opacity: enabled ? 1 : 0.45
    font.pixelSize: small ? 13 : 14
    font.weight: Font.DemiBold
    scale: down ? 0.95 : 1

    PressScale on scale {}

    background: Rectangle {
        radius: height / 2
        color: {
            if (button.primary)
                return button.down ? Qt.darker(Theme.accent, 1.08) : button.hovered ? Qt.lighter(Theme.accent, 1.06) : Theme.accent;
            if (button.danger)
                return button.down || button.hovered ? Qt.lighter(Theme.dangerBg, 1.15) : Theme.dangerBg;
            return button.down ? Theme.selected : button.hovered ? Theme.hover : Qt.alpha(Theme.hover, 0);
        }
        border.width: button.visualFocus ? 2 : button.primary ? 0 : 1
        border.color: button.visualFocus ? Theme.text : button.danger ? Theme.dangerLine : Theme.border

        TintFade on color {}
    }

    contentItem: Row {
        spacing: 8

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            visible: button.iconName !== ""
            name: button.iconName
            size: 16
            stroke: 1.9
            color: button.primary ? Theme.accentInk : button.danger ? Theme.danger : Theme.text
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            visible: button.text !== ""
            text: button.text
            font: button.font
            color: button.primary ? Theme.accentInk : button.danger ? Theme.danger : Theme.text
        }
    }
}
