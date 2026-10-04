import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// Every track of the library: search it, sort it, play it.
Item {
    id: page

    signal navigate(string page)

    /// How many tracks are listed, of how many, and how long they last.
    readonly property string summary: {
        if (!tracks.ready)
            return "";
        const count = tracks.count === tracks.total ? Format.count(tracks.total, Tr.t.track_one, Tr.t.tracks_many) : Tr.t.tracks_filtered.arg(Format.number(tracks.count)).arg(Format.number(tracks.total));
        return tracks.count > 0 ? count + " · " + Format.duration(tracks.duration) : count;
    }

    TrackList {
        id: tracks
        search: search.text
        sort: "artist,album"
    }

    Shortcut {
        sequences: [StandardKey.Find]
        onActivated: search.focusInput()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 28
            Layout.rightMargin: 28
            Layout.topMargin: 22
            Layout.bottomMargin: 14
            spacing: 14

            RowLayout {
                Layout.fillWidth: true
                spacing: 16

                ColumnLayout {
                    Layout.alignment: Qt.AlignBottom
                    spacing: 4

                    Label {
                        text: Tr.t.tracks
                        color: Theme.text
                        font.pixelSize: 26
                        font.weight: Font.Bold
                        font.letterSpacing: -0.52
                    }
                    Label {
                        text: page.summary
                        color: Theme.text2
                        font.pixelSize: 13
                        font.features: {
                            "tnum": 1
                        }
                    }
                }
                Item {
                    Layout.fillWidth: true
                }
                RowLayout {
                    Layout.alignment: Qt.AlignBottom
                    spacing: 8

                    PillButton {
                        primary: true
                        iconName: "play"
                        text: Tr.t.play
                        enabled: tracks.count > 0
                        onClicked: tracks.playAll(false)
                    }
                    PillButton {
                        iconName: "shuffle"
                        text: Tr.t.shuffle
                        enabled: tracks.count > 0
                        onClicked: tracks.playAll(true)
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                SearchField {
                    id: search
                    Layout.fillWidth: true
                    Layout.minimumWidth: 200
                    Layout.preferredWidth: 360
                    Layout.maximumWidth: 360
                    placeholder: Tr.t.search_tracks
                }
                Item {
                    Layout.fillWidth: true
                }
                SortButton {
                    list: tracks
                }
            }
        }

        TrackTable {
            id: table
            Layout.fillWidth: true
            Layout.fillHeight: true
            list: tracks
            visible: tracks.count > 0
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !table.visible

            EmptyState {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -40
                visible: tracks.ready
                readonly property bool empty: tracks.total === 0
                iconName: empty ? (Scan.running ? "refresh" : "folder") : "search"
                title: empty ? (Scan.running ? Tr.t.scanning_library : Tr.t.empty_library) : Tr.t.no_results
                message: empty && !Scan.running ? Tr.t.empty_library_hint : ""
                action: empty && !Scan.running ? Tr.t.open_sources : ""
                onTriggered: page.navigate("sources")
            }
        }
    }
}
