pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Dialogs
import QtQuick.Layouts
import NMusic

// The first run: where the music is, before the library shows.
Rectangle {
    id: page

    /// The sources to start with, as `{ location, name, suggested }`.
    property var added: []
    /// `added` holds what the library suggested.
    property bool seeded: false

    /// Takes the library's sources, the system's music folder, as suggestions once they come.
    function seed() {
        if (seeded || !Sources.loaded)
            return;
        seeded = true;
        added = Sources.items.map(source => ({
                    location: source.location,
                    name: source.name,
                    suggested: true
                }));
    }

    function addFolder(location: string) {
        if (location === "" || added.some(source => source.location === location))
            return;
        const parts = location.split(/[\\/]/).filter(part => part !== "");
        added = added.concat([
            {
                location: location,
                name: parts.length > 0 ? parts[parts.length - 1] : location,
                suggested: false
            }
        ]);
    }

    function removeAt(index: int) {
        added = added.filter((source, other) => other !== index);
    }

    color: Theme.bg
    Component.onCompleted: seed()

    Connections {
        target: Sources

        function onLoadedChanged() {
            page.seed();
        }
    }

    Flickable {
        id: scroller
        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: Math.max(height, column.implicitHeight + 64)
        boundsBehavior: Flickable.StopAtBounds

        ColumnLayout {
            id: column
            x: Math.round((scroller.width - width) / 2)
            y: Math.max(32, Math.round((scroller.height - implicitHeight) / 2))
            width: Math.min(720, scroller.width - 32)
            spacing: 26

            RowLayout {
                spacing: 10

                Logo {
                    size: 36
                    radius: 10
                }
                Label {
                    text: "n_music"
                    color: Theme.text
                    font.pixelSize: 20
                    font.weight: Font.Bold
                    font.letterSpacing: -0.2
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10

                Label {
                    Layout.fillWidth: true
                    text: Tr.t.import_title
                    wrapMode: Text.Wrap
                    color: Theme.text
                    font.pixelSize: 34
                    font.weight: Font.Bold
                    font.letterSpacing: -0.68
                }
                Label {
                    Layout.fillWidth: true
                    text: Tr.t.import_intro
                    wrapMode: Text.Wrap
                    lineHeight: 1.2
                    color: Theme.text2
                    font.pixelSize: 16
                }
            }

            GridLayout {
                id: kinds
                Layout.fillWidth: true
                columns: Math.max(1, Math.floor((column.width + columnSpacing) / (200 + columnSpacing)))
                rowSpacing: 10
                columnSpacing: 10

                Repeater {
                    model: SourceKinds.all

                    AbstractButton {
                        id: card

                        required property var modelData

                        // Of the column, as the grid's own width follows its cards.
                        Layout.preferredWidth: (column.width - (kinds.columns - 1) * kinds.columnSpacing) / kinds.columns
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        padding: 14
                        hoverEnabled: true
                        text: modelData.name
                        Accessible.description: modelData.detail
                        onClicked: {
                            if (modelData.value === "folder")
                                picker.open();
                            else
                                Sources.add(modelData.value);
                        }

                        background: Rectangle {
                            radius: 12
                            color: card.modelData.later ? (card.hovered ? Theme.hover : "transparent") : card.down ? Theme.selected : card.hovered ? Theme.hover : Theme.surface
                            border.width: card.visualFocus ? 2 : 1
                            border.color: card.visualFocus ? Theme.text : card.modelData.later ? Theme.line : Theme.accent
                        }
                        contentItem: ColumnLayout {
                            spacing: 8

                            RowLayout {
                                Layout.fillWidth: true

                                Rectangle {
                                    implicitWidth: 36
                                    implicitHeight: 36
                                    radius: 9
                                    color: Theme.raised

                                    Icon {
                                        anchors.centerIn: parent
                                        name: card.modelData.icon
                                        size: 19
                                        color: card.modelData.later ? Theme.text3 : Theme.accentText
                                    }
                                }
                                Item {
                                    Layout.fillWidth: true
                                }
                                Badge {
                                    visible: card.modelData.later
                                    text: Tr.t.later
                                }
                            }
                            Label {
                                Layout.fillWidth: true
                                text: card.text
                                elide: Text.ElideRight
                                color: card.modelData.later ? Theme.text2 : Theme.text
                                font.pixelSize: 15
                                font.weight: Font.DemiBold
                            }
                            Label {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                verticalAlignment: Text.AlignTop
                                text: card.modelData.detail
                                wrapMode: Text.Wrap
                                lineHeight: 1.1
                                color: Theme.text2
                                font.pixelSize: 13
                            }
                        }
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                visible: page.added.length > 0
                spacing: 8

                Label {
                    Layout.bottomMargin: 2
                    text: Tr.t.added
                    color: Theme.text3
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.78
                    font.capitalization: Font.AllUppercase
                }
                Repeater {
                    model: page.added

                    Rectangle {
                        id: source

                        required property var modelData
                        required property int index

                        Layout.fillWidth: true
                        implicitHeight: Math.max(60, entry.implicitHeight + 20)
                        radius: 12
                        color: Theme.surface
                        border.width: 1
                        border.color: Theme.line2

                        RowLayout {
                            id: entry
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
                                    color: Theme.accentText
                                }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Label {
                                    Layout.fillWidth: true
                                    text: source.modelData.name
                                    elide: Text.ElideRight
                                    color: Theme.text
                                    font.pixelSize: 15
                                    font.weight: Font.DemiBold
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: SourceKinds.name("folder") + " · " + source.modelData.location
                                    elide: Text.ElideMiddle
                                    color: Theme.text2
                                    font.pixelSize: 13
                                }
                            }
                            Badge {
                                visible: source.modelData.suggested
                                implicitHeight: 24
                                leftPadding: 8
                                rightPadding: 8
                                text: Tr.t.suggested
                                font.pixelSize: 12
                                font.weight: Font.Normal
                            }
                            IconButton {
                                size: 36
                                iconSize: 14
                                stroke: 2.2
                                color: Theme.text3
                                iconName: "close"
                                text: Tr.t.remove_source.arg(source.modelData.name)
                                onClicked: page.removeAt(source.index)
                            }
                        }
                    }
                }
            }

            AbstractButton {
                id: build
                Layout.alignment: Qt.AlignRight
                implicitHeight: 46
                implicitWidth: implicitContentWidth + leftPadding + rightPadding
                leftPadding: 24
                rightPadding: 24
                hoverEnabled: true
                text: Tr.t.build_library
                font.pixelSize: 15
                font.weight: Font.Bold
                onClicked: Sources.setFolders(page.added.map(source => source.location))

                background: Rectangle {
                    radius: height / 2
                    color: build.down ? Qt.darker(Theme.accent, 1.08) : build.hovered ? Qt.lighter(Theme.accent, 1.06) : Theme.accent
                    border.width: build.visualFocus ? 2 : 0
                    border.color: Theme.text
                }
                contentItem: Row {
                    spacing: 8

                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: build.text
                        font: build.font
                        color: Theme.accentInk
                    }
                    Icon {
                        anchors.verticalCenter: parent.verticalCenter
                        name: "arrow-right"
                        size: 16
                        stroke: 2.2
                        color: Theme.accentInk
                    }
                }
            }
        }

        ScrollBar.vertical: ThinScrollBar {}
    }

    FolderDialog {
        id: picker
        title: Tr.t.choose_folder
        onAccepted: page.addFolder(Catalog.folder(selectedFolder))
    }
}
