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

    /// The user picked the choice of `value`.
    signal activated(string value)

    readonly property var current: options.find(option => option.value === value) ?? null

    implicitHeight: 38
    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    leftPadding: 12
    rightPadding: 12
    hoverEnabled: true
    font.pixelSize: 13
    font.weight: Font.Medium
    text: current ? current.label : ""
    onClicked: menu.open()

    background: Rectangle {
        radius: 8
        color: button.down || menu.visible ? Theme.selected : button.hovered ? Theme.hover : "transparent"
        border.width: button.visualFocus ? 2 : 0
        border.color: Theme.text
    }

    contentItem: Row {
        spacing: 6

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            visible: button.iconName !== ""
            name: button.iconName
            size: 15
            stroke: 1.9
            color: Theme.text2
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            text: button.text
            font: button.font
            color: Theme.text2
        }
        Icon {
            anchors.verticalCenter: parent.verticalCenter
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
