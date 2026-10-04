import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An entry of the sidebar.
AbstractButton {
    id: item

    property string iconName
    property bool active: false

    implicitHeight: 36
    leftPadding: 12
    rightPadding: 12
    hoverEnabled: true
    Accessible.role: Accessible.Button

    background: Rectangle {
        radius: 8
        color: item.active ? Theme.raised : item.hovered ? Theme.hover : "transparent"
    }

    contentItem: Row {
        spacing: 12

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            name: item.iconName
            color: item.active ? Theme.accentText : item.hovered ? Theme.text : Theme.text2
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            width: item.availableWidth - 30
            text: item.text
            elide: Text.ElideRight
            font.pixelSize: 14
            font.weight: Font.Medium
            color: item.active || item.hovered ? Theme.text : Theme.text2
        }
    }
}
