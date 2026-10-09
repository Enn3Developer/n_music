pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A drop-down choice like a form's select. `options` are `{ value, label }`, with an optional
// `short` label for the box, or `{ heading }` titling the options below it.
AbstractButton {
    id: box

    property var options: []
    property string value
    property real radius: 8
    /// Quieter text, for conditions beside the field they apply to.
    property bool muted: false
    property real popupWidth: Math.max(width, 160)

    /// The user picked the option of `value`.
    signal activated(string value)

    readonly property var current: options.find(option => option.heading === undefined && option.value === value) ?? null

    implicitWidth: leftPadding + label.implicitWidth + 6 + 14 + rightPadding
    implicitHeight: 36
    leftPadding: 10
    rightPadding: 10
    hoverEnabled: true
    font.pixelSize: 14
    text: current ? (current.short ?? current.label) : ""
    Accessible.role: Accessible.ComboBox
    onClicked: menu.open()

    background: Rectangle {
        radius: box.radius
        color: Theme.input
        border.width: box.visualFocus ? 2 : 1
        border.color: menu.visible || box.visualFocus ? Theme.accent : box.hovered ? Theme.text3 : Theme.border

        ColorFade on border.color {}
    }

    contentItem: Item {
        Label {
            id: label
            anchors.left: parent.left
            anchors.right: chevron.left
            anchors.rightMargin: 6
            anchors.verticalCenter: parent.verticalCenter
            text: box.text
            elide: Text.ElideRight
            font: box.font
            color: box.muted ? Theme.textSoft : Theme.text
        }
        Icon {
            id: chevron
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            name: menu.visible ? "chevron-up" : "chevron-down"
            size: 14
            stroke: 2
            color: Theme.text2
        }
    }

    PopupMenu {
        id: menu
        y: box.height + 6
        width: box.popupWidth
        onAboutToShow: currentIndex = box.options.indexOf(box.current)

        Instantiator {
            model: box.options
            delegate: MenuEntry {
                required property var modelData
                heading: modelData.heading !== undefined
                text: heading ? modelData.heading : modelData.label
                checked: !heading && modelData.value === box.value
                onTriggered: box.activated(modelData.value)
            }
            onObjectAdded: (index, entry) => menu.insertItem(index, entry)
            onObjectRemoved: (index, entry) => menu.removeItem(entry)
        }
    }
}
