import QtQuick
import QtQuick.Controls.Basic
import NMusic

// Tells that the library refused a playlist change, and why, until dismissed or for a while.
Toast {
    id: toast

    property string message

    function show(text: string) {
        message = text;
        pop();
    }

    stay: 10000
    width: Math.min(420, parent.width - 48)
    height: content.implicitHeight + 24
    radius: 10
    color: Theme.dangerBg
    border.width: 1
    border.color: Theme.dangerLine
    Accessible.role: Accessible.AlertMessage
    Accessible.name: Tr.t.playlist_not_saved + ". " + message

    Icon {
        x: 14
        y: 13
        name: "alert"
        size: 18
        stroke: 2
        color: Theme.danger
    }

    Column {
        id: content
        x: 14 + 18 + 12
        y: 12
        width: dismiss.x - 12 - x
        spacing: 3

        Label {
            width: parent.width
            text: Tr.t.playlist_not_saved
            wrapMode: Text.Wrap
            color: Theme.dangerTitle
            font.pixelSize: 13
            font.weight: Font.DemiBold
        }
        Label {
            width: parent.width
            text: toast.message
            wrapMode: Text.Wrap
            color: Theme.dangerText
            font.pixelSize: 13
        }
    }

    IconButton {
        id: dismiss
        x: parent.width - 12 - width
        y: 10
        size: 28
        radius: 6
        iconSize: 12
        stroke: 2.4
        iconName: "close"
        color: Theme.dangerText
        text: Tr.t.dismiss
        onClicked: toast.dismiss()
    }
}
