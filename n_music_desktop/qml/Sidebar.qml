pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// Navigation between the library views, playlists, the queue and the settings. A pill under
// each group marks its active entry.
Rectangle {
    id: sidebar

    property string page

    signal navigate(string page)

    implicitWidth: 240
    color: Theme.side

    Rectangle {
        anchors.right: parent.right
        width: 1
        height: parent.height
        color: Theme.line
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: 18
        anchors.leftMargin: 12
        anchors.rightMargin: 13
        anchors.bottomMargin: 14
        spacing: 22

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 10
            spacing: 10

            Logo {
                size: 28
                radius: 8
            }
            Label {
                Layout.fillWidth: true
                text: "N Music"
                elide: Text.ElideRight
                color: Theme.text
                font.pixelSize: 17
                font.weight: Font.Bold
            }
            HistoryButtons {}
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2

            Label {
                Layout.leftMargin: 12
                Layout.bottomMargin: 6
                text: Tr.t.library.toUpperCase()
                color: Theme.text3
                font.pixelSize: 11
                font.weight: Font.DemiBold
                font.letterSpacing: 0.9
            }
            Item {
                Layout.fillWidth: true
                implicitHeight: library.implicitHeight

                NavPill {
                    width: parent.width
                    height: 36
                    index: [tracks.active, albums.active, artists.active, genres.active, sources.active].indexOf(true)
                }
                Column {
                    id: library
                    width: parent.width
                    spacing: 2

                    NavItem {
                        id: tracks
                        width: parent.width
                        pilled: true
                        iconName: "note"
                        text: Tr.t.tracks
                        active: sidebar.page === "tracks"
                        onClicked: sidebar.navigate("tracks")
                    }
                    NavItem {
                        id: albums
                        width: parent.width
                        pilled: true
                        iconName: "disc"
                        text: Tr.t.albums
                        active: sidebar.page === "albums" || sidebar.page.startsWith("album:")
                        onClicked: sidebar.navigate("albums")
                    }
                    NavItem {
                        id: artists
                        width: parent.width
                        pilled: true
                        iconName: "artist"
                        text: Tr.t.artists
                        active: sidebar.page === "artists" || sidebar.page.startsWith("artist:")
                        onClicked: sidebar.navigate("artists")
                    }
                    NavItem {
                        id: genres
                        width: parent.width
                        pilled: true
                        iconName: "tag"
                        text: Tr.t.genres
                        active: sidebar.page === "genres" || sidebar.page.startsWith("genre:")
                        onClicked: sidebar.navigate("genres")
                    }
                    NavItem {
                        id: sources
                        width: parent.width
                        pilled: true
                        iconName: "folder"
                        text: Tr.t.sources
                        active: sidebar.page === "sources" || sidebar.page.startsWith("source:")
                        onClicked: sidebar.navigate("sources")
                    }
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 2

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 12
                Layout.rightMargin: 4
                Layout.bottomMargin: 6

                Label {
                    Layout.fillWidth: true
                    text: Tr.t.playlists.toUpperCase()
                    color: Theme.text3
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.9
                }
                IconButton {
                    size: 28
                    radius: 6
                    iconSize: 16
                    stroke: 2
                    iconName: "plus"
                    text: Tr.t.new_playlist
                    onClicked: naming.ask("")
                }
            }

            ListView {
                id: playlists
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                spacing: 2
                boundsBehavior: Flickable.StopAtBounds
                model: Playlists.items
                Accessible.name: Tr.t.playlists

                // Below the entries, scrolling with them.
                NavPill {
                    width: playlists.width
                    height: 36
                    index: Playlists.items.findIndex(playlist => sidebar.page === "playlist:" + playlist.id)
                }

                delegate: NavItem {
                    required property var modelData

                    width: ListView.view.width
                    pilled: true
                    iconName: modelData.smart ? "filter" : "playlist"
                    text: modelData.name
                    active: sidebar.page === "playlist:" + modelData.id
                    onClicked: sidebar.navigate("playlist:" + modelData.id)
                }

                ScrollBar.vertical: ThinScrollBar {}
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2

            Rectangle {
                Layout.fillWidth: true
                Layout.bottomMargin: 10
                implicitHeight: 1
                color: Theme.line
            }
            Item {
                Layout.fillWidth: true
                implicitHeight: bottom.implicitHeight

                NavPill {
                    width: parent.width
                    height: 36
                    index: [queue.active, settings.active].indexOf(true)
                }
                Column {
                    id: bottom
                    width: parent.width
                    spacing: 2

                    NavItem {
                        id: queue
                        width: parent.width
                        pilled: true
                        iconName: "queue"
                        text: Tr.t.queue
                        active: sidebar.page === "queue"
                        onClicked: sidebar.navigate("queue")
                    }
                    NavItem {
                        id: settings
                        width: parent.width
                        pilled: true
                        iconName: "settings"
                        text: Tr.t.settings
                        active: sidebar.page === "settings" || sidebar.page.startsWith("settings:")
                        onClicked: sidebar.navigate("settings")
                    }
                }
            }

            // How far a scan got; opens the sources.
            AbstractButton {
                id: progress

                readonly property string count: Scan.found > 0 ? Tr.t.read_of_found.arg(Format.number(Scan.read)).arg(Format.number(Scan.found)) : ""

                Layout.fillWidth: true
                Layout.topMargin: 8
                visible: Scan.running
                topPadding: 10
                bottomPadding: 10
                leftPadding: 12
                rightPadding: 12
                hoverEnabled: true
                text: Tr.t.updating_library
                Accessible.name: text + (count === "" ? "" : ", " + count)
                onClicked: sidebar.navigate("sources")

                background: Rectangle {
                    radius: 8
                    color: progress.down ? Theme.selected : progress.hovered ? Theme.raised : Theme.surface
                    border.width: progress.visualFocus ? 2 : 0
                    border.color: Theme.text

                    TintFade on color {}
                }
                contentItem: ColumnLayout {
                    spacing: 6

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Label {
                            Layout.fillWidth: true
                            text: progress.text
                            elide: Text.ElideRight
                            color: Theme.text
                            font.pixelSize: 12
                        }
                        Label {
                            text: progress.count
                            color: Theme.text2
                            font.pixelSize: 12
                            font.features: {
                                "tnum": 1
                            }
                        }
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 4
                        radius: 2
                        color: Theme.line2

                        Rectangle {
                            width: Scan.found > 0 ? parent.width * Math.min(1, Scan.read / Scan.found) : 0
                            height: parent.height
                            radius: 2
                            color: Theme.accent
                        }
                    }
                }
            }
        }
    }

    PromptDialog {
        id: naming
        title: Tr.t.new_playlist
        asksText: true
        placeholder: Tr.t.playlist_name
        confirmText: Tr.t.create
        onConfirmed: name => Playlists.create(name)
    }
}
