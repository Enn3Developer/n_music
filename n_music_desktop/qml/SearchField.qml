import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A text field with a search icon and a button to clear it; Escape clears it too.
Rectangle {
    id: field

    property alias text: input.text
    property alias placeholder: input.placeholderText

    function focusInput() {
        input.forceActiveFocus();
        input.selectAll();
    }

    implicitHeight: 38
    radius: 10
    color: Theme.field
    border.width: 1
    border.color: input.activeFocus ? Theme.border : Theme.line2

    Icon {
        id: magnifier
        x: 12
        anchors.verticalCenter: parent.verticalCenter
        name: "search"
        size: 16
        stroke: 2
        color: Theme.text3
    }

    TextField {
        id: input
        anchors.left: magnifier.right
        anchors.leftMargin: 8
        anchors.right: clear.visible ? clear.left : parent.right
        anchors.rightMargin: clear.visible ? 4 : 12
        anchors.verticalCenter: parent.verticalCenter
        padding: 0
        background: null
        color: Theme.text
        placeholderTextColor: Theme.text3
        selectionColor: Theme.accent
        selectedTextColor: Theme.accentInk
        font.pixelSize: 14
        Accessible.name: field.placeholder
        Keys.onEscapePressed: event => {
            event.accepted = text !== "";
            text = "";
        }
    }

    AbstractButton {
        id: clear
        anchors.right: parent.right
        anchors.rightMargin: 7
        anchors.verticalCenter: parent.verticalCenter
        width: 24
        height: 24
        visible: input.text !== ""
        hoverEnabled: true
        focusPolicy: Qt.NoFocus
        Accessible.name: Tr.t.clear_search
        onClicked: {
            input.text = "";
            input.forceActiveFocus();
        }

        background: Rectangle {
            radius: 12
            color: clear.hovered ? Theme.raised : "transparent"
        }
        contentItem: Item {
            Icon {
                anchors.centerIn: parent
                name: "close"
                size: 12
                stroke: 2.4
                color: Theme.text2
            }
        }
    }
}
