import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An entry of the sidebar. Pressing shrinks its icon.
AbstractButton {
    id: item

    property string iconName
    property bool active: false
    /// A NavPill under the entries grounds the active one, rather than the entry itself.
    property bool pilled: false

    implicitHeight: 36
    leftPadding: 12
    rightPadding: 12
    hoverEnabled: true
    Accessible.role: Accessible.Button

    background: Rectangle {
        radius: 8
        color: item.active && !item.pilled ? Theme.raised : !item.active && item.hovered ? Theme.hover : Qt.alpha(Theme.hover, 0)

        TintFade on color {}
    }

    contentItem: Row {
        spacing: 12

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            name: item.iconName
            color: item.active ? Theme.accentText : item.hovered ? Theme.text : Theme.text2
            scale: item.down ? 0.84 : 1

            ColorFade on color {}
            PressScale on scale {}
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            width: item.availableWidth - 30
            text: item.text
            elide: Text.ElideRight
            font.pixelSize: 14
            font.weight: Font.Medium
            color: item.active || item.hovered ? Theme.text : Theme.text2

            ColorFade on color {}
        }
    }
}
