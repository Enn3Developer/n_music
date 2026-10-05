pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A small modal dialog confirming an action, optionally asking for a name first.
Popup {
    id: dialog

    property string title
    property string message
    /// Asks for a name in a text field.
    property bool asksText: false
    property alias text: field.text
    property alias placeholder: field.placeholderText
    property string confirmText
    /// The action destroys something.
    property bool danger: false
    /// Tells what is wrong with the text typed, or "" when nothing is; null takes any text.
    property var check: null
    /// What is wrong with the text, shown under it until it changes.
    property string problem

    /// Confirmed, with the name typed when asked for one.
    signal confirmed(string text)

    /// Opens with `text` in the field, selected.
    function ask(text: string) {
        field.text = text;
        problem = "";
        open();
    }

    function confirm() {
        const text = field.text.trim();
        if (asksText && text === "")
            return;
        if (asksText && check !== null) {
            problem = check(text);
            if (problem !== "")
                return;
        }
        close();
        confirmed(text);
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(400, parent.width - 32)
    modal: true
    focus: true
    padding: 20
    onOpened: {
        if (asksText) {
            field.forceActiveFocus();
            field.selectAll();
        } else {
            confirmButton.forceActiveFocus();
        }
    }

    Overlay.modal: Rectangle {
        color: Theme.dark ? "#99000000" : "#55000000"
    }

    background: Rectangle {
        radius: 12
        color: Theme.surface
        border.width: 1
        border.color: Theme.line2
    }

    contentItem: Column {
        spacing: 14

        Label {
            width: parent.width
            text: dialog.title
            wrapMode: Text.Wrap
            color: Theme.text
            font.pixelSize: 17
            font.weight: Font.Bold
        }
        Label {
            width: parent.width
            visible: dialog.message !== ""
            text: dialog.message
            wrapMode: Text.Wrap
            color: Theme.text2
            font.pixelSize: 14
        }
        TextBox {
            id: field
            width: parent.width
            visible: dialog.asksText
            Accessible.name: dialog.title
            onAccepted: dialog.confirm()
            onEdited: dialog.problem = ""
        }
        Label {
            width: parent.width
            visible: dialog.problem !== ""
            text: dialog.problem
            wrapMode: Text.Wrap
            color: Theme.danger
            font.pixelSize: 13
        }
        Row {
            anchors.right: parent.right
            topPadding: 4
            spacing: 8

            PillButton {
                implicitHeight: 38
                text: Tr.t.cancel
                onClicked: dialog.close()
            }
            PillButton {
                id: confirmButton
                implicitHeight: 38
                primary: !dialog.danger
                danger: dialog.danger
                enabled: !dialog.asksText || field.text.trim() !== ""
                text: dialog.confirmText
                onClicked: dialog.confirm()
            }
        }
    }
}
