import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An item of the queue: cover, title and artist, length; queued ones can be taken out.
Rectangle {
    id: row

    required property int index
    required property string title
    required property string artist
    required property string length
    required property string cover
    required property string section

    signal activated
    signal remove

    implicitHeight: 54
    radius: 8
    color: section === "next" ? (mouse.containsMouse ? Theme.raised : Theme.surface) : mouse.containsMouse ? Theme.hover : "transparent"

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        onDoubleClicked: row.activated()
    }

    Cover {
        id: art
        x: 8
        anchors.verticalCenter: parent.verticalCenter
        size: 40
        path: row.cover
        opacity: row.section === "history" ? 0.6 : 1
    }
    Column {
        anchors.left: art.right
        anchors.leftMargin: 12
        anchors.right: time.left
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2

        Label {
            width: parent.width
            text: row.title
            elide: Text.ElideRight
            color: row.section === "history" ? Theme.text2 : Theme.text
            font.pixelSize: 14
            font.weight: Font.DemiBold
        }
        Label {
            width: parent.width
            text: row.artist === "" ? Tr.t.unknown_artist : row.artist
            elide: Text.ElideRight
            color: row.artist === "" ? Theme.text3 : Theme.text2
            font.pixelSize: 13
        }
    }
    Label {
        id: time
        anchors.right: action.left
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        width: 52
        horizontalAlignment: Text.AlignRight
        text: row.length
        color: Theme.text2
        font.pixelSize: 13
        font.features: {
            "tnum": 1
        }
    }
    Item {
        id: action
        anchors.right: parent.right
        anchors.rightMargin: 8
        width: 36
        height: parent.height

        IconButton {
            anchors.centerIn: parent
            visible: row.section === "next"
            size: 32
            iconSize: 14
            stroke: 2.2
            iconName: "close"
            color: Theme.text3
            text: Tr.t.remove_from_up_next.arg(row.title)
            onClicked: row.remove()
        }
    }
}
