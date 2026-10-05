import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// A source of the library: its name, kind and location, its tracks, whether it is up to date,
// and a menu updating it or taking it out.
Item {
    id: row

    /// `{ name, kind, location, tracks, available, updating }`, see `Sources.items`.
    required property var source
    /// Draws a line under it, before the next row.
    property bool divider: false

    signal updateRequested
    signal reloadRequested
    signal removeRequested

    implicitHeight: Math.max(64, content.implicitHeight + 20)

    RowLayout {
        id: content
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 10
        anchors.topMargin: 10
        anchors.bottomMargin: 10
        spacing: 12

        Rectangle {
            implicitWidth: 36
            implicitHeight: 36
            radius: 9
            color: Theme.raised

            Icon {
                anchors.centerIn: parent
                name: "folder"
                size: 18
                color: Theme.text2
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2

            Row {
                Layout.fillWidth: true
                spacing: 8

                Label {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.min(implicitWidth, parent.width - kind.width - parent.spacing)
                    text: row.source.name
                    elide: Text.ElideRight
                    color: Theme.text
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                }
                Badge {
                    id: kind
                    anchors.verticalCenter: parent.verticalCenter
                    text: SourceKinds.name(row.source.kind)
                }
            }
            Label {
                Layout.fillWidth: true
                text: row.source.location
                elide: Text.ElideMiddle
                color: Theme.text2
                font.pixelSize: 13
            }
        }
        ColumnLayout {
            spacing: 2

            Label {
                Layout.alignment: Qt.AlignRight
                text: Format.count(row.source.tracks, Tr.t.track_one, Tr.t.tracks_many)
                color: Theme.text
                font.pixelSize: 13
                font.features: {
                    "tnum": 1
                }
            }
            Label {
                Layout.alignment: Qt.AlignRight
                text: !row.source.available ? Tr.t.source_missing : row.source.updating ? Tr.t.source_updating : Tr.t.source_up_to_date
                color: !row.source.available ? Theme.danger : row.source.updating ? Theme.accentText : Theme.text2
                font.pixelSize: 12
            }
        }
        IconButton {
            id: more
            size: 36
            iconSize: 16
            stroke: 1.9
            color: Theme.text3
            iconName: "more"
            text: Tr.t.source_actions.arg(row.source.name)
            onClicked: actions.open()

            PopupMenu {
                id: actions
                x: more.width - width
                y: more.height + 4

                MenuEntry {
                    iconName: "refresh"
                    enabled: !row.source.updating
                    text: Tr.t.update_now
                    onTriggered: row.updateRequested()
                }
                MenuEntry {
                    iconName: "tag"
                    enabled: !row.source.updating
                    text: Tr.t.reload_metadata
                    onTriggered: row.reloadRequested()
                }
                MenuLine {}
                MenuEntry {
                    iconName: "trash"
                    danger: true
                    text: Tr.t.remove
                    onTriggered: row.removeRequested()
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
