pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

ApplicationWindow {
    id: window

    /// The page the content area shows; one of many, like a playlist's, is `<section>:<which>`.
    property string page: "tracks"
    /// The page without what follows its `:`.
    readonly property string section: page.indexOf(":") < 0 ? page : page.slice(0, page.indexOf(":"))
    /// What followed the `:` the last time each section showed, by section.
    property var subpages: ({})
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
        const colon = page.indexOf(":");
        const name = colon < 0 ? page : page.slice(0, colon);
        if (colon >= 0)
            subpages = Object.assign({}, subpages, {
                [name]: page.slice(colon + 1)
            });
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
    onWidthChanged: Shell.width = width
    Component.onCompleted: Shell.width = width
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
            playlistId: Number(window.subpages.playlist ?? 0)
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: albumsPage
        GroupsPage {
            kind: "album"
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: artistsPage
        GroupsPage {
            kind: "artist"
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: genresPage
        GroupsPage {
            kind: "genre"
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: sourcesPage
        GroupsPage {
            kind: "source"
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: albumPage
        CollectionPage {
            kind: "album"
            argument: window.subpages.album ?? ""
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: artistPage
        CollectionPage {
            kind: "artist"
            argument: window.subpages.artist ?? ""
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: genrePage
        CollectionPage {
            kind: "genre"
            argument: window.subpages.genre ?? ""
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: sourcePage
        CollectionPage {
            kind: "source"
            argument: window.subpages.source ?? ""
            onNavigate: to => window.page = to
        }
    }
    Component {
        id: settingsPage
        SettingsPage {
            section: window.subpages.settings ?? "playback"
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
        visible: !Sources.firstRun
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
                    model: ["tracks", "albums", "album", "artists", "artist", "genres", "genre", "sources", "source", "playlist", "queue", "settings"]

                    Loader {
                        required property string modelData

                        anchors.fill: parent
                        active: window.visited[modelData] === true
                        visible: window.section === modelData
                        sourceComponent: ({
                                tracks: tracksPage,
                                albums: albumsPage,
                                album: albumPage,
                                artists: artistsPage,
                                artist: artistPage,
                                genres: genresPage,
                                genre: genrePage,
                                sources: sourcesPage,
                                source: sourcePage,
                                playlist: playlistPage,
                                queue: queuePage,
                                settings: settingsPage
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

            // The queue page shows all of it already.
            NowPlayingPanel {
                Layout.fillHeight: true
                visible: Shell.wide && window.section !== "queue"
                onNavigate: to => window.page = to
            }
        }

        PlayerBar {
            Layout.fillWidth: true
            queueOpen: window.page === "queue"
            onToggleQueue: window.toggleQueue()
        }
    }

    // The first run asks where the music is before the library shows.
    Loader {
        anchors.fill: parent
        active: Sources.firstRun
        sourceComponent: ImportPage {}
    }
}
