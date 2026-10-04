pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// Navigation between the library views, playlists, the queue and the settings.
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
            Layout.leftMargin: 10
            spacing: 10

            Rectangle {
                implicitWidth: 28
                implicitHeight: 28
                radius: 8
                color: Theme.accent

                Icon {
                    anchors.centerIn: parent
                    name: "note"
                    size: 16
                    stroke: 2.2
                    color: Theme.accentInk
                }
            }
            Label {
                text: "n_music"
                color: Theme.text
                font.pixelSize: 17
                font.weight: Font.Bold
            }
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
            NavItem {
                Layout.fillWidth: true
                iconName: "note"
                text: Tr.t.tracks
                active: sidebar.page === "tracks"
                onClicked: sidebar.navigate("tracks")
            }
            NavItem {
                Layout.fillWidth: true
                iconName: "disc"
                text: Tr.t.albums
                active: sidebar.page === "albums"
                onClicked: sidebar.navigate("albums")
            }
            NavItem {
                Layout.fillWidth: true
                iconName: "artist"
                text: Tr.t.artists
                active: sidebar.page === "artists"
                onClicked: sidebar.navigate("artists")
            }
            NavItem {
                Layout.fillWidth: true
                iconName: "tag"
                text: Tr.t.genres
                active: sidebar.page === "genres"
                onClicked: sidebar.navigate("genres")
            }
            NavItem {
                Layout.fillWidth: true
                iconName: "folder"
                text: Tr.t.sources
                active: sidebar.page === "sources"
                onClicked: sidebar.navigate("sources")
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

                delegate: NavItem {
                    required property var modelData

                    width: ListView.view.width
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
            NavItem {
                Layout.fillWidth: true
                iconName: "queue"
                text: Tr.t.queue
                active: sidebar.page === "queue"
                onClicked: sidebar.navigate("queue")
            }
            NavItem {
                Layout.fillWidth: true
                iconName: "settings"
                text: Tr.t.settings
                active: sidebar.page === "settings"
                onClicked: sidebar.navigate("settings")
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
