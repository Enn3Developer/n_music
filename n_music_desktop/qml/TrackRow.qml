import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A track in a table: its number (a playing mark while current), cover, title and artist,
// album, then the values its columns show and the length.
Rectangle {
    id: row

    required property int index
    required property string title
    required property string artist
    required property string album
    required property string year
    required property int plays
    required property string length
    required property string cover
    required property bool current
    required property real lastPlayed
    required property real added
    required property TrackColumns columns
    property bool selected: false

    signal clicked
    signal activated

    implicitHeight: 52
    radius: 8
    color: selected ? Theme.raised : current ? Theme.selected : mouse.containsMouse ? Theme.hover : "transparent"

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: row.clicked()
        onDoubleClicked: row.activated()
    }

    Row {
        x: 12
        height: parent.height

        Item {
            width: row.columns.number
            height: parent.height

            Label {
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                visible: !row.current
                text: row.index + 1
                color: Theme.text3
                font.pixelSize: 13
                font.features: {
                    "tnum": 1
                }
            }
            Icon {
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                visible: row.current
                name: "equalizer"
                size: 14
                color: Theme.accentText
                Accessible.name: Tr.t.now_playing
            }
        }

        Item {
            width: row.columns.title
            height: parent.height

            Cover {
                id: art
                anchors.verticalCenter: parent.verticalCenter
                path: row.cover
            }
            Column {
                anchors.left: art.right
                anchors.leftMargin: 12
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Label {
                    width: parent.width
                    text: row.title
                    elide: Text.ElideRight
                    color: row.current ? Theme.accentText : Theme.text
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
        }

        Label {
            width: row.columns.album
            rightPadding: 12
            anchors.verticalCenter: parent.verticalCenter
            text: row.album
            elide: Text.ElideRight
            color: Theme.text2
            font.pixelSize: 13
        }
        Value {
            width: row.columns.year
            text: row.year
        }
        Value {
            width: row.columns.added
            horizontalAlignment: Text.AlignLeft
            text: row.added > 0 ? Format.ago(row.added) : ""
        }
        Value {
            width: row.columns.plays
            text: row.plays
        }
        Value {
            width: row.columns.lastPlayed
            text: row.lastPlayed > 0 ? Format.ago(row.lastPlayed) : Tr.t.never_played
        }
        Value {
            width: row.columns.time
            text: row.length
        }
    }

    component Value: Label {
        visible: width > 0
        anchors.verticalCenter: parent.verticalCenter
        horizontalAlignment: Text.AlignRight
        color: Theme.text2
        font.pixelSize: 13
        font.features: {
            "tnum": 1
        }
    }
}
