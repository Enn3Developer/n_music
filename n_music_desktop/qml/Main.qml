pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

ApplicationWindow {
    id: window

    /// The page the content area shows.
    property string page: "tracks"
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

    onPageChanged: visited = Object.assign({}, visited, {
        [page]: true
    })
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
        id: placeholderPage
        PlaceholderPage {}
    }

    RowLayout {
        anchors.fill: parent
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
                model: ["tracks", "albums", "artists", "genres", "sources", "queue", "settings"]

                Loader {
                    required property string modelData

                    anchors.fill: parent
                    active: window.visited[modelData] === true
                    visible: window.page === modelData
                    sourceComponent: modelData === "tracks" ? tracksPage : placeholderPage
                    onLoaded: {
                        if (item instanceof PlaceholderPage)
                            item.title = Qt.binding(() => Tr.t[modelData]);
                    }
                }
            }
        }
    }
}
