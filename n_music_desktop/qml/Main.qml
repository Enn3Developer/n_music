pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

ApplicationWindow {
    id: window

    /// The page the content area shows; a playlist's is `playlist:<id>`.
    property string page: "tracks"
    /// The page without what follows its `:`.
    readonly property string section: page.split(":")[0]
    /// The playlist shown last.
    property real playlistId: 0
    /// The page shown before the queue, to go back to.
    property string beforeQueue: "tracks"
    /// Pages shown so far; they stay loaded, keeping their search and scroll position.
    property var visited: ({
            tracks: true
        })

    // The saved size, shrunk to fit smaller screens.
    width: Math.min(AppState.windowWidth, Screen.desktopAvailableWidth)
    height: Math.min(AppState.windowHeight, Screen.desktopAvailableHeight)
    minimumWidth: 360
    minimumHeight: 480
    visible: true
    title: "N Music"
    color: Theme.bg
    font.family: Theme.font
    font.pixelSize: 14

    onPageChanged: {
        // Not `section`: its binding may not have seen this change yet.
        const [name, argument] = page.split(":");
        if (name === "playlist")
            playlistId = Number(argument);
        visited = Object.assign({}, visited, {
            [name]: true
        });
    }

    function toggleQueue() {
        if (page === "queue") {
            page = beforeQueue;
        } else {
            beforeQueue = page;
            page = "queue";
        }
    }

    // Space plays and pauses, unless a text field takes it.
    Shortcut {
        sequence: "Space"
        onActivated: Player.toggle()
    }
    onClosing: AppState.windowClosing(width, height)
    onVisibilityChanged: AppState.setVisible(visibility !== Window.Minimized && visibility !== Window.Hidden)

    FontLoader {
        source: "../assets/fonts/Figtree-Regular.ttf"
    }
    FontLoader {
        source: "../assets/fonts/Figtree-Medium.ttf"
    }
    FontLoader {
        source: "../assets/fonts/Figtree-SemiBold.ttf"
    }
    FontLoader {
        source: "../assets/fonts/Figtree-Bold.ttf"
    }

    Component {
        id: tracksPage
        TracksPage {
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: queuePage
        QueuePage {
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: playlistPage
        PlaylistPage {
            playlistId: window.playlistId
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: placeholderPage
        PlaceholderPage {}
    }

    Connections {
        target: Playlists

        function onCreated(id: real) {
            window.page = "playlist:" + id;
        }
        function onRejected(message: string) {
            rejected.show(message);
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Sidebar {
                Layout.fillHeight: true
                page: window.page
                onNavigate: to => window.page = to
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Repeater {
                    model: ["tracks", "albums", "artists", "genres", "sources", "playlist", "queue", "settings"]

                    Loader {
                        required property string modelData

                        anchors.fill: parent
                        active: window.visited[modelData] === true
                        visible: window.section === modelData
                        sourceComponent: ({
                                tracks: tracksPage,
                                playlist: playlistPage,
                                queue: queuePage
                            })[modelData] ?? placeholderPage
                        onLoaded: {
                            if (item instanceof PlaceholderPage)
                                item.title = Qt.binding(() => Tr.t[modelData]);
                        }
                    }
                }

                RejectedToast {
                    id: rejected
                    anchors.right: parent.right
                    anchors.rightMargin: 24
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 20
                }
            }
        }

        PlayerBar {
            Layout.fillWidth: true
            queueOpen: window.page === "queue"
            onToggleQueue: window.toggleQueue()
        }
    }
}
