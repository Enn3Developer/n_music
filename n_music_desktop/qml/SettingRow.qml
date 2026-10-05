import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// A row of a SettingsGroup: what it is, what it does, and its control at the end.
Item {
    id: row

    property string title
    property string description
    /// An icon before the description, like a check mark for a good state.
    property string descriptionIcon
    property color descriptionIconColor: Theme.text2
    /// Draws a line under it, before the next row.
    property bool divider: false
    default property alias control: content.data

    Layout.fillWidth: true
    implicitHeight: content.implicitHeight + 28

    RowLayout {
        id: content
        anchors.fill: parent
        anchors.leftMargin: 16
        anchors.rightMargin: 16
        anchors.topMargin: 14
        anchors.bottomMargin: 14
        spacing: 16

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3

            Label {
                Layout.fillWidth: true
                visible: row.title !== ""
                text: row.title
                wrapMode: Text.Wrap
                color: Theme.text
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }
            RowLayout {
                Layout.fillWidth: true
                visible: row.description !== ""
                spacing: 6

                Icon {
                    Layout.alignment: Qt.AlignTop
                    Layout.topMargin: 1
                    visible: row.descriptionIcon !== ""
                    name: row.descriptionIcon
                    size: 14
                    stroke: 2.4
                    color: row.descriptionIconColor
                }
                Label {
                    Layout.fillWidth: true
                    text: row.description
                    wrapMode: Text.Wrap
                    color: Theme.text2
                    font.pixelSize: 13
                }
            }
        }
    }

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        visible: row.divider
        color: Theme.line
    }
}
