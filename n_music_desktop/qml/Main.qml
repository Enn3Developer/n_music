pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

ApplicationWindow {
    id: window

    /// The page the content area shows; one of many, like a playlist's, is `<section>:<which>`.
    readonly property string page: History.page
    /// The page without what follows its `:`.
    readonly property string section: page.indexOf(":") < 0 ? page : page.slice(0, page.indexOf(":"))
    /// What followed the `:` the last time each section showed, by section.
    property var subpages: ({})
    /// Pages shown so far; they stay loaded, keeping their search and scroll position.
    property var visited: ({
            tracks: true
        })

    /// The window was maximized when it last showed, to reopen that way.
    property bool maximized: AppState.windowMaximized
    /// The window's size when it is not maximized, to reopen at.
    property size normalSize: Qt.size(AppState.windowWidth, AppState.windowHeight)

    // The saved size, shrunk to fit smaller screens.
    width: Math.min(AppState.windowWidth, Screen.desktopAvailableWidth)
    height: Math.min(AppState.windowHeight, Screen.desktopAvailableHeight)
    minimumWidth: 360
    minimumHeight: 480
    visibility: AppState.windowMaximized ? Window.Maximized : Window.Windowed
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

    /// Shows the page `to`, after the one shown in the history.
    function go(to: string) {
        // The settings remember their section: going back shows the one shown then.
        History.go(to === "settings" ? "settings:" + (subpages.settings ?? "playback") : to);
    }

    function toggleQueue() {
        if (page !== "queue")
            go("queue");
        else if (History.canGoBack)
            History.back();
        else
            go("tracks");
    }

    // Space plays and pauses, unless a text field takes it.
    Shortcut {
        sequence: "Space"
        onActivated: Player.toggle()
    }
    Shortcut {
        sequences: [StandardKey.Back]
        enabled: !Sources.firstRun
        onActivated: History.back()
    }
    Shortcut {
        sequences: [StandardKey.Forward]
        enabled: !Sources.firstRun
        onActivated: History.forward()
    }
    onClosing: {
        if (visibility === Window.Windowed)
            normalSize = Qt.size(width, height);
        AppState.windowClosing(normalSize.width, normalSize.height, maximized);
    }
    onWidthChanged: {
        Shell.width = width;
        normalSizeTimer.restart();
    }
    onHeightChanged: normalSizeTimer.restart()
    Component.onCompleted: Shell.width = width
    onVisibilityChanged: {
        AppState.setVisible(visibility !== Window.Minimized && visibility !== Window.Hidden);
        // Minimized or hidden, it stays maximized or not as it was.
        if (visibility === Window.Windowed || visibility === Window.Maximized)
            maximized = visibility === Window.Maximized;
    }

    // Maximizing may resize the window just before it says it is maximized: a new size is its
    // size when not maximized only once it held a moment.
    Timer {
        id: normalSizeTimer
        interval: 500
        onTriggered: {
            if (window.visibility === Window.Windowed)
                window.normalSize = Qt.size(window.width, window.height);
        }
    }

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
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: queuePage
        QueuePage {
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: playlistPage
        PlaylistPage {
            playlistId: Number(window.subpages.playlist ?? 0)
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: albumsPage
        GroupsPage {
            kind: "album"
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: artistsPage
        GroupsPage {
            kind: "artist"
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: genresPage
        GroupsPage {
            kind: "genre"
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: sourcesPage
        GroupsPage {
            kind: "source"
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: albumPage
        CollectionPage {
            kind: "album"
            argument: window.subpages.album ?? ""
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: artistPage
        CollectionPage {
            kind: "artist"
            argument: window.subpages.artist ?? ""
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: genrePage
        CollectionPage {
            kind: "genre"
            argument: window.subpages.genre ?? ""
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: sourcePage
        CollectionPage {
            kind: "source"
            argument: window.subpages.source ?? ""
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: settingsPage
        SettingsPage {
            section: window.subpages.settings ?? "playback"
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: placeholderPage
        PlaceholderPage {}
    }

    Connections {
        target: Playlists

        function onCreated(id: real) {
            window.go("playlist:" + id);
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
                visible: Shell.regular || Shell.wide
                page: window.page
                onNavigate: to => window.go(to)
            }
            Rail {
                Layout.fillHeight: true
                visible: Shell.compact
                page: window.page
                onNavigate: to => window.go(to)
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
                onNavigate: to => window.go(to)
            }
        }

        PlayerBar {
            Layout.fillWidth: true
            queueOpen: window.page === "queue"
            onToggleQueue: window.toggleQueue()
            onMiniRequested: {
                window.hide();
                mini.show();
                mini.raise();
                mini.requestActivate();
            }
        }
    }

    // The mouse's back and forward buttons, wherever they are pressed; other buttons reach what
    // is under it.
    MouseArea {
        anchors.fill: parent
        enabled: !Sources.firstRun
        acceptedButtons: Qt.BackButton | Qt.ForwardButton
        onPressed: mouse => {
            if (mouse.button === Qt.BackButton)
                History.back();
            else
                History.forward();
        }
    }

    // Stands in for this window while it hides.
    MiniPlayer {
        id: mini
        onExpandRequested: {
            mini.hide();
            window.show();
            window.raise();
            window.requestActivate();
        }
    }

    // Narrow windows keep the navigation here, sliding in from the left.
    Drawer {
        id: navigation
        width: Math.min(280, window.width - 56)
        height: window.height
        edge: Qt.LeftEdge
        interactive: Shell.narrow
        padding: 0

        Overlay.modal: Rectangle {
            color: Theme.shadow
        }
        background: null

        Sidebar {
            anchors.fill: parent
            page: window.page
            onNavigate: to => {
                window.go(to);
                navigation.close();
            }
        }
    }

    Connections {
        target: History

        // Going back or forward from it closes it too.
        function onPageChanged() {
            navigation.close();
        }
    }

    Connections {
        target: Shell

        function onNavigationRequested() {
            navigation.open();
        }
        function onNarrowChanged() {
            if (!Shell.narrow)
                navigation.close();
        }
    }

    // The first run asks where the music is before the library shows.
    Loader {
        anchors.fill: parent
        active: Sources.firstRun
        sourceComponent: ImportPage {}
    }
}
