pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Dialogs
import QtQuick.Layouts
import NMusic

// Settings › Sources: what the library is made of, how far a scan got, and scanning again.
ColumnLayout {
    id: section

    spacing: 28

    ColumnLayout {
        Layout.fillWidth: true
        Layout.topMargin: 6
        spacing: 4

        Label {
            text: Tr.t.sources
            color: Theme.text
            font.pixelSize: 20
            font.weight: Font.Bold
        }
        Label {
            Layout.fillWidth: true
            text: Tr.t.sources_intro
            wrapMode: Text.Wrap
            color: Theme.text2
            font.pixelSize: 14
        }
    }

    Rectangle {
        Layout.fillWidth: true
        visible: Scan.running
        implicitHeight: progress.implicitHeight + 32
        radius: 12
        color: Theme.surface
        border.width: 1
        border.color: Theme.line2
        Accessible.role: Accessible.ProgressBar
        Accessible.name: Tr.t.updating_library

        ColumnLayout {
            id: progress
            anchors.fill: parent
            anchors.leftMargin: 18
            anchors.rightMargin: 18
            anchors.topMargin: 16
            anchors.bottomMargin: 16
            spacing: 12

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Icon {
                    name: "refresh"
                    size: 18
                    stroke: 2
                    color: Theme.accentText
                }
                Label {
                    Layout.fillWidth: true
                    text: Tr.t.updating_library
                    elide: Text.ElideRight
                    color: Theme.text
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                }
                Label {
                    text: Scan.found > 0 ? Tr.t.tracks_read.arg(Format.number(Scan.read)).arg(Format.number(Scan.found)) : Tr.t.scanning_library
                    color: Theme.text2
                    font.pixelSize: 14
                    font.features: {
                        "tnum": 1
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 6
                radius: 3
                color: Theme.line2

                Rectangle {
                    width: Scan.found > 0 ? parent.width * Math.min(1, Scan.read / Scan.found) : 0
                    height: parent.height
                    radius: 3
                    color: Theme.accent
                }
            }
            Label {
                Layout.fillWidth: true
                text: Tr.t.updating_library_hint
                wrapMode: Text.Wrap
                color: Theme.text3
                font.pixelSize: 13
            }
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 10

        RowLayout {
            Layout.fillWidth: true

            Label {
                Layout.fillWidth: true
                text: Tr.t.your_sources
                color: Theme.text
                font.pixelSize: 15
                font.weight: Font.Bold
            }
            AbstractButton {
                id: adding
                implicitHeight: 34
                implicitWidth: implicitContentWidth + leftPadding + rightPadding
                leftPadding: 12
                rightPadding: 12
                hoverEnabled: true
                text: Tr.t.add_source
                font.pixelSize: 13
                font.weight: Font.DemiBold
                Accessible.role: Accessible.ButtonMenu
                onClicked: kinds.open()

                background: Rectangle {
                    radius: height / 2
                    color: adding.down || kinds.visible ? Theme.selected : adding.hovered ? Theme.raised : Theme.input
                    border.width: adding.visualFocus ? 2 : 1
                    border.color: adding.visualFocus ? Theme.text : Theme.accent
                }
                contentItem: Row {
                    spacing: 6

                    Icon {
                        anchors.verticalCenter: parent.verticalCenter
                        name: "plus"
                        size: 14
                        stroke: 2.2
                        color: Theme.text
                    }
                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: adding.text
                        font: adding.font
                        color: Theme.text
                    }
                }

                PopupMenu {
                    id: kinds
                    x: adding.width - width
                    y: adding.height + 8
                    width: 300

                    Instantiator {
                        model: SourceKinds.all
                        delegate: SourceKindEntry {
                            required property var modelData

                            text: modelData.name
                            detail: modelData.detail
                            iconName: modelData.icon
                            later: modelData.later
                            onTriggered: {
                                if (modelData.value === "folder")
                                    picker.open();
                                else
                                    Sources.add(modelData.value);
                            }
                        }
                        onObjectAdded: (index, entry) => kinds.insertItem(index, entry)
                        onObjectRemoved: (index, entry) => kinds.removeItem(entry)
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: list.implicitHeight + 2
            radius: 12
            color: Theme.surface
            border.width: 1
            border.color: Theme.line2

            ColumnLayout {
                id: list
                anchors.fill: parent
                anchors.margins: 1
                spacing: 0

                Repeater {
                    model: Sources.items

                    SourceRow {
                        required property var modelData
                        required property int index

                        Layout.fillWidth: true
                        source: modelData
                        divider: index < Sources.items.length - 1
                        onUpdateRequested: Sources.refresh(index)
                        onReloadRequested: Sources.reload(index)
                        onRemoveRequested: removal.askFor(index, modelData.name)
                    }
                }
                Label {
                    Layout.fillWidth: true
                    Layout.margins: 16
                    visible: Sources.items.length === 0
                    text: Tr.t.no_sources
                    wrapMode: Text.Wrap
                    color: Theme.text2
                    font.pixelSize: 13
                }
            }
        }
        Label {
            Layout.fillWidth: true
            text: Tr.t.remove_source_note
            wrapMode: Text.Wrap
            color: Theme.text3
            font.pixelSize: 13
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 10

        Label {
            text: Tr.t.maintenance
            color: Theme.text
            font.pixelSize: 15
            font.weight: Font.Bold
        }
        SettingsGroup {
            Layout.fillWidth: true

            SettingRow {
                title: Tr.t.update_library
                description: {
                    if (Scan.updated <= 0)
                        return Tr.t.update_library_hint;
                    const date = new Date(Scan.updated * 1000);
                    const time = date.toLocaleTimeString(Qt.locale(), Locale.ShortFormat);
                    const days = Format.daysSince(Scan.updated);
                    const when = days === 0 ? Tr.t.today_at.arg(time) : days === 1 ? Tr.t.yesterday_at.arg(time) : date.toLocaleString(Qt.locale(), Locale.ShortFormat);
                    return Tr.t.update_library_hint + " " + Tr.t.last_update.arg(when);
                }
                divider: true

                PillButton {
                    small: true
                    text: Tr.t.update_now
                    enabled: !Scan.running
                    onClicked: Scan.refresh()
                }
            }
            SettingRow {
                title: Tr.t.reload_metadata_title
                description: Tr.t.reload_metadata_hint

                PillButton {
                    small: true
                    text: Tr.t.reload_metadata
                    enabled: !Scan.running
                    onClicked: Scan.reload()
                }
            }
        }
    }

    FolderDialog {
        id: picker
        title: Tr.t.choose_folder
        onAccepted: Sources.addFolder(Catalog.folder(selectedFolder))
    }

    PromptDialog {
        id: removal

        property int index: -1

        function askFor(index: int, name: string) {
            removal.index = index;
            title = Tr.t.remove_source_title.arg(name);
            open();
        }

        message: Tr.t.remove_source_message
        confirmText: Tr.t.remove
        danger: true
        onConfirmed: Sources.remove(index)
    }
}
