import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

ApplicationWindow {
    id: window

    // The page the content area shows.
    property string page: "tracks"

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

    onClosing: AppState.windowClosing(width, height)
    onVisibilityChanged: AppState.setVisible(visibility !== Window.Minimized && visibility !== Window.Hidden)

    FontLoader { source: "../assets/fonts/Figtree-Regular.ttf" }
    FontLoader { source: "../assets/fonts/Figtree-Medium.ttf" }
    FontLoader { source: "../assets/fonts/Figtree-SemiBold.ttf" }
    FontLoader { source: "../assets/fonts/Figtree-Bold.ttf" }

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

            Label {
                x: 28
                y: 22
                text: Tr.t[window.page === "locations" ? "sources" : window.page] || ""
                color: Theme.text
                font.pixelSize: 26
                font.weight: Font.Bold
                font.letterSpacing: -0.5
            }
        }
    }
}
