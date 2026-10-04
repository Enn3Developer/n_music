import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An item of a PopupMenu, with an optional icon, and a check mark while `checked`.
MenuItem {
    id: entry

    property string iconName
    property bool danger: false
    /// Titles the entries below it instead of being one.
    property bool heading: false

    implicitHeight: heading ? 28 : 34
    enabled: !heading
    leftPadding: 10
    rightPadding: 10
    font.pixelSize: 13
    font.weight: Font.Medium
    indicator: null
    arrow: null

    background: Rectangle {
        radius: 6
        color: entry.highlighted && !entry.heading ? Theme.menuHover : "transparent"
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
            topPadding: entry.heading ? 6 : 0
            text: entry.text
            elide: Text.ElideRight
            font.pixelSize: entry.heading ? 11 : entry.font.pixelSize
            font.weight: entry.heading ? Font.DemiBold : entry.font.weight
            font.letterSpacing: entry.heading ? 0.88 : 0
            font.capitalization: entry.heading ? Font.AllUppercase : Font.MixedCase
            color: entry.heading || !entry.enabled ? Theme.text3 : entry.danger ? Theme.danger : entry.checked ? Theme.accentText : Theme.text
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
