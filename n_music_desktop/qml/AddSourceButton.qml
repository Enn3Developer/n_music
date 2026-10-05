pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// Adds a source: a menu of the kinds there are, then a folder picker or a prompt for a
// playlist's address.
AbstractButton {
    id: button

    /// Shows only its icon, like the actions of a NarrowBar.
    property bool iconOnly: false

    /// Opens the menu of kinds.
    function open() {
        kinds.open();
    }

    implicitHeight: iconOnly ? 44 : 34
    implicitWidth: iconOnly ? 44 : implicitContentWidth + leftPadding + rightPadding
    leftPadding: iconOnly ? 0 : 12
    rightPadding: leftPadding
    hoverEnabled: true
    text: Tr.t.add_source
    font.pixelSize: 13
    font.weight: Font.DemiBold
    Accessible.role: Accessible.ButtonMenu
    Accessible.name: text
    onClicked: kinds.open()

    background: Rectangle {
        radius: button.iconOnly ? 10 : height / 2
        color: button.down || kinds.visible ? Theme.selected : button.hovered ? Theme.raised : button.iconOnly ? "transparent" : Theme.input
        border.width: button.visualFocus ? 2 : button.iconOnly ? 0 : 1
        border.color: button.visualFocus ? Theme.text : Theme.accent
    }
    contentItem: Item {
        implicitWidth: content.implicitWidth
        implicitHeight: content.implicitHeight

        Row {
            id: content
            anchors.centerIn: parent
            spacing: 6

            Icon {
                anchors.verticalCenter: parent.verticalCenter
                name: "plus"
                size: button.iconOnly ? 20 : 14
                stroke: button.iconOnly ? 2 : 2.2
                color: Theme.text
            }
            Label {
                anchors.verticalCenter: parent.verticalCenter
                visible: !button.iconOnly
                text: button.text
                font: button.font
                color: Theme.text
            }
        }
    }

    PopupMenu {
        id: kinds
        x: button.width - width
        y: button.height + 8
        width: 300

        Instantiator {
            model: SourceKinds.all
            delegate: SourceKindEntry {
                required property var modelData

                text: modelData.name
                detail: modelData.detail
                iconName: modelData.icon
                later: modelData.later
                onTriggered: {
                    if (modelData.value === "folder")
                        picker.open();
                    else if (modelData.value === "web")
                        webPrompt.ask("");
                }
            }
            onObjectAdded: (index, entry) => kinds.insertItem(index, entry)
            onObjectRemoved: (index, entry) => kinds.removeItem(entry)
        }
    }

    FolderPicker {
        id: picker
        taken: Sources.items.map(source => source.location)
        onChosen: path => Sources.addFolder(path)
    }

    WebSourceDialog {
        id: webPrompt
        onChosen: address => Sources.addWeb(address)
    }
}
