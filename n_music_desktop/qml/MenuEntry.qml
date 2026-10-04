import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An item of a PopupMenu, with an optional icon, and a check mark while `checked`.
MenuItem {
    id: entry

    property string iconName
    property bool danger: false

    implicitHeight: 34
    leftPadding: 10
    rightPadding: 10
    font.pixelSize: 13
    font.weight: Font.Medium
    indicator: null
    arrow: null

    background: Rectangle {
        radius: 6
        color: entry.highlighted ? Theme.menuHover : "transparent"
    }

    contentItem: Item {
        implicitHeight: 20

        Icon {
            id: glyph
            anchors.verticalCenter: parent.verticalCenter
            visible: entry.iconName !== ""
            name: entry.iconName
            size: 16
            color: entry.danger ? Theme.danger : Theme.text2
        }
        Label {
            anchors.left: glyph.visible ? glyph.right : parent.left
            anchors.leftMargin: glyph.visible ? 10 : 0
            anchors.right: mark.visible ? mark.left : parent.right
            anchors.rightMargin: mark.visible ? 10 : 0
            anchors.verticalCenter: parent.verticalCenter
            text: entry.text
            elide: Text.ElideRight
            font: entry.font
            color: !entry.enabled ? Theme.text3 : entry.danger ? Theme.danger : Theme.text
        }
        Icon {
            id: mark
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            visible: entry.checked
            name: "check"
            size: 16
            stroke: 2
            color: Theme.accentText
        }
    }
}
