import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// A kind of source in a menu: its icon, its name and what it brings, with a Later badge while
// the core cannot read it.
MenuItem {
    id: entry

    property string iconName
    property string detail
    property bool later: false

    implicitHeight: Math.max(50, implicitContentHeight + topPadding + bottomPadding)
    topPadding: 8
    bottomPadding: 8
    leftPadding: 10
    rightPadding: 10
    indicator: null
    arrow: null

    /// What stands out of the entry's ground, inverted while highlighted.
    readonly property color well: highlighted ? Theme.menu : Theme.menuHover

    background: Rectangle {
        radius: 8
        color: entry.highlighted ? Theme.menuHover : "transparent"
    }

    contentItem: RowLayout {
        spacing: 12

        Rectangle {
            implicitWidth: 32
            implicitHeight: 32
            radius: 8
            color: entry.well

            Icon {
                anchors.centerIn: parent
                name: entry.iconName
                size: 17
                color: entry.later ? Theme.text3 : Theme.accentText
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            Label {
                Layout.fillWidth: true
                text: entry.text
                elide: Text.ElideRight
                color: entry.later ? Theme.text2 : Theme.text
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }
            Label {
                Layout.fillWidth: true
                text: entry.detail
                wrapMode: Text.Wrap
                color: Theme.text2
                font.pixelSize: 12
            }
        }
        Badge {
            visible: entry.later
            text: Tr.t.later
            fill: entry.well
        }
    }
}
