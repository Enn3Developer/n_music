import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A menu in the app's style; fill it with MenuEntry items.
Menu {
    id: menu

    padding: 6
    margins: 8
    overlap: 0

    background: Rectangle {
        implicitWidth: 220
        radius: 10
        color: Theme.menu
        border.width: 1
        border.color: Theme.line2
    }

    delegate: MenuEntry {}
}
