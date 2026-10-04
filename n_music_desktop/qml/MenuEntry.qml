import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An item of a PopupMenu, with an optional icon, and a check mark while `checked` or an arrow
// opening its submenu.
MenuItem {
    id: entry

    property string iconName
    property bool danger: false
    /// Titles the entries below it instead of being one.
    property bool heading: false
    /// Explains the entries above it instead of being one.
    property bool note: false
    /// Takes no room while false, unlike `visible` alone in a menu.
    property bool shown: true

    visible: shown
    implicitHeight: !shown ? 0 : note ? contentItem.implicitHeight + topPadding + bottomPadding : heading ? 28 : 34
    topPadding: note ? 6 : 0
    bottomPadding: note ? 4 : 0
    enabled: !heading && !note
    leftPadding: 10
    rightPadding: 10
    font.pixelSize: 13
    font.weight: Font.Medium
    indicator: null
    arrow: null

    background: Rectangle {
        radius: 6
        color: (entry.highlighted || (entry.subMenu && entry.subMenu.visible)) && entry.enabled ? Theme.menuHover : "transparent"
    }

    contentItem: Item {
        implicitHeight: entry.note ? text.implicitHeight : 20

        Icon {
            id: glyph
            anchors.verticalCenter: parent.verticalCenter
            visible: entry.iconName !== ""
            name: entry.iconName
            size: 16
            color: entry.danger ? Theme.danger : Theme.text2
        }
        Label {
            id: text
            anchors.left: glyph.visible ? glyph.right : parent.left
            anchors.leftMargin: glyph.visible ? 10 : 0
            anchors.right: mark.visible ? mark.left : parent.right
            anchors.rightMargin: mark.visible ? 10 : 0
            anchors.verticalCenter: parent.verticalCenter
            topPadding: entry.heading ? 6 : 0
            text: entry.text
            elide: entry.note ? Text.ElideNone : Text.ElideRight
            wrapMode: entry.note ? Text.Wrap : Text.NoWrap
            font.pixelSize: entry.heading ? 11 : entry.note ? 12 : entry.font.pixelSize
            font.weight: entry.heading ? Font.DemiBold : entry.note ? Font.Normal : entry.font.weight
            font.letterSpacing: entry.heading ? 0.88 : 0
            font.capitalization: entry.heading ? Font.AllUppercase : Font.MixedCase
            color: entry.heading || entry.note || !entry.enabled ? Theme.text3 : entry.danger ? Theme.danger : entry.checked ? Theme.accentText : Theme.text
        }
        Icon {
            id: mark
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            visible: entry.checked || entry.subMenu !== null
            name: entry.subMenu ? "chevron-right" : "check"
            size: entry.subMenu ? 14 : 16
            stroke: 2
            color: entry.subMenu ? Theme.text2 : entry.enabled ? Theme.accentText : Theme.text3
        }
    }
}
