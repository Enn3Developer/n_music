pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A quiet button showing a choice, like an order, and a menu picking another.
AbstractButton {
    id: button

    /// The choices, as `{ value, label }`.
    property var options: []
    property string value
    property string iconName
    /// Shows only its icon, like the actions of a NarrowBar.
    property bool iconOnly: false

    /// The user picked the choice of `value`.
    signal activated(string value)

    readonly property var current: options.find(option => option.value === value) ?? null

    implicitHeight: iconOnly ? 44 : 38
    implicitWidth: iconOnly ? 44 : implicitContentWidth + leftPadding + rightPadding
    leftPadding: 12
    rightPadding: 12
    hoverEnabled: true
    font.pixelSize: 13
    font.weight: Font.Medium
    text: current ? current.label : ""
    onClicked: menu.open()

    background: Rectangle {
        radius: button.iconOnly ? 10 : 8
        color: button.down || menu.visible ? Theme.selected : button.hovered ? Theme.hover : Qt.alpha(Theme.hover, 0)
        border.width: button.visualFocus ? 2 : 0
        border.color: Theme.text

        TintFade on color {}
    }

    contentItem: Row {
        spacing: 6

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            visible: button.iconName !== ""
            name: button.iconName
            size: button.iconOnly ? 20 : 15
            stroke: button.iconOnly ? 2 : 1.9
            color: button.iconOnly ? Theme.text : Theme.text2
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            visible: !button.iconOnly
            text: button.text
            font: button.font
            color: Theme.text2
        }
        Icon {
            anchors.verticalCenter: parent.verticalCenter
            visible: !button.iconOnly
            name: "chevron-down"
            size: 14
            stroke: 2
            color: Theme.text2
        }
    }

    PopupMenu {
        id: menu
        x: button.width - width
        y: button.height + 4
        transformOrigin: Popup.TopRight

        Instantiator {
            model: button.options
            delegate: MenuEntry {
                required property var modelData
                text: modelData.label
                checked: modelData.value === button.value
                onTriggered: button.activated(modelData.value)
            }
            onObjectAdded: (index, entry) => menu.insertItem(index, entry)
            onObjectRemoved: (index, entry) => menu.removeItem(entry)
        }
    }
}
