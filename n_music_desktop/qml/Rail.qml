pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// The sidebar folded to a rail of icons for compact windows; the playlists open from a menu.
Rectangle {
    id: rail

    property string page

    signal navigate(string page)

    implicitWidth: 64
    color: Theme.side

    Rectangle {
        anchors.right: parent.right
        width: 1
        height: parent.height
        color: Theme.line
    }

    Flickable {
        anchors.fill: parent
        anchors.rightMargin: 1
        contentHeight: column.height
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ColumnLayout {
            id: column
            width: parent.width
            // Keeps the queue and the settings at the bottom, unless the window is too short.
            height: Math.max(rail.height, implicitHeight)
            spacing: 4

            Logo {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 14
                Layout.bottomMargin: 6
                size: 32
                radius: 9
            }
            HistoryButtons {
                Layout.alignment: Qt.AlignHCenter
                Layout.bottomMargin: 10
                size: 26
            }

            RailItem {
                iconName: "note"
                text: Tr.t.tracks
                active: rail.page === "tracks"
                onClicked: rail.navigate("tracks")
            }
            RailItem {
                iconName: "disc"
                text: Tr.t.albums
                active: rail.page === "albums" || rail.page.startsWith("album:")
                onClicked: rail.navigate("albums")
            }
            RailItem {
                iconName: "artist"
                text: Tr.t.artists
                active: rail.page === "artists" || rail.page.startsWith("artist:")
                onClicked: rail.navigate("artists")
            }
            RailItem {
                iconName: "tag"
                text: Tr.t.genres
                active: rail.page === "genres" || rail.page.startsWith("genre:")
                onClicked: rail.navigate("genres")
            }
            RailItem {
                iconName: "folder"
                text: Tr.t.sources
                active: rail.page === "sources" || rail.page.startsWith("source:")
                onClicked: rail.navigate("sources")
            }
            RailItem {
                id: lists
                iconName: "playlist"
                text: Tr.t.playlists
                active: rail.page.startsWith("playlist:") || menu.visible
                tipShown: hovered && !used && !menu.visible
                onClicked: menu.open()

                PopupMenu {
                    id: menu
                    x: lists.width + 8
                    width: 240

                    Instantiator {
                        model: Playlists.items
                        delegate: MenuEntry {
                            required property var modelData

                            iconName: modelData.smart ? "filter" : "playlist"
                            text: modelData.name
                            checked: rail.page === "playlist:" + modelData.id
                            onTriggered: rail.navigate("playlist:" + modelData.id)
                        }
                        onObjectAdded: (index, entry) => menu.insertItem(index, entry)
                        onObjectRemoved: (index, entry) => menu.removeItem(entry)
                    }

                    MenuLine {
                        shown: Playlists.items.length > 0
                    }
                    MenuEntry {
                        iconName: "plus"
                        text: Tr.t.new_playlist_ellipsis
                        onTriggered: naming.ask("")
                    }
                }
            }

            Item {
                Layout.fillHeight: true
            }

            // How far a scan got; opens the sources.
            RailItem {
                visible: Scan.running
                iconName: "refresh"
                iconColor: Theme.accentText
                spinning: true
                text: Tr.t.updating_library + (Scan.found > 0 ? " · " + Tr.t.read_of_found.arg(Format.number(Scan.read)).arg(Format.number(Scan.found)) : "")
                onClicked: rail.navigate("settings:sources")
            }
            RailItem {
                iconName: "queue"
                text: Tr.t.queue
                active: rail.page === "queue"
                onClicked: rail.navigate("queue")
            }
            RailItem {
                Layout.bottomMargin: 14
                iconName: "settings"
                text: Tr.t.settings
                active: rail.page === "settings" || rail.page.startsWith("settings:")
                onClicked: rail.navigate("settings")
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

    // An entry of the rail: its icon, named by a tip beside it on hover.
    component RailItem: AbstractButton {
        id: item

        property string iconName
        property bool active: false
        property color iconColor: active ? Theme.accentText : hovered ? Theme.text : Theme.text2
        /// Turns the icon round and round while it shows.
        property bool spinning: false
        /// Pressed since the pointer came, so the tip stays away until it leaves.
        property bool used: false
        property bool tipShown: hovered && !used

        Layout.alignment: Qt.AlignHCenter
        implicitWidth: 44
        implicitHeight: 44
        padding: 0
        hoverEnabled: true
        Accessible.role: Accessible.Button
        Accessible.name: text
        onHoveredChanged: used = false
        onPressed: used = true

        background: Rectangle {
            radius: 10
            color: item.active ? Theme.raised : item.down ? Theme.selected : item.hovered ? Theme.hover : "transparent"
            border.width: item.visualFocus ? 2 : 0
            border.color: Theme.text
        }
        contentItem: Item {
            Icon {
                anchors.centerIn: parent
                name: item.iconName
                size: 20
                color: item.iconColor

                RotationAnimator on rotation {
                    running: item.spinning && item.visible
                    from: 0
                    to: 360
                    duration: 1400
                    loops: Animation.Infinite
                }
            }
        }

        Tip {
            visible: item.tipShown
            x: item.width + 10
            y: (item.height - height) / 2
            text: item.text
        }
    }
}
