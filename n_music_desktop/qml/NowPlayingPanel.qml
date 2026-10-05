pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// Beside the page in wide windows: the current track, large, and what plays after it.
Rectangle {
    id: panel

    signal navigate(string page)

    implicitWidth: 360
    color: Theme.panel

    QueueList {
        id: queue
    }

    Rectangle {
        width: 1
        height: parent.height
        color: Theme.line
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 20
        anchors.rightMargin: 20
        anchors.topMargin: 22
        spacing: 18

        Cover {
            Layout.alignment: Qt.AlignHCenter
            // Leaves room for a few tracks after it in short windows.
            size: Math.min(panel.width - 40, Math.max(120, panel.height - 400))
            radius: 12
            path: Player.cover
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4

            Label {
                Layout.fillWidth: true
                text: Player.loaded ? Player.title : Tr.t.not_playing
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                color: Player.loaded ? Theme.text : Theme.text3
                font.pixelSize: 20
                font.weight: Font.Bold
            }
            Label {
                Layout.fillWidth: true
                visible: text !== ""
                text: [Player.artist, Player.album, Player.year].filter(part => part !== "").join(" · ")
                elide: Text.ElideRight
                color: Theme.text2
                font.pixelSize: 14
            }
            Label {
                Layout.fillWidth: true
                Layout.topMargin: 4
                visible: text !== ""
                text: [Format.audio(Player.codec, Player.sampleRate, Player.bits), Format.gain(Player.albumGain, Player.trackGain)].filter(part => part !== "").join(" · ")
                elide: Text.ElideRight
                color: Theme.text3
                font.pixelSize: 12
            }
        }

        RowLayout {
            Layout.fillWidth: true

            Label {
                Layout.fillWidth: true
                text: Tr.t.up_next
                color: Theme.text
                font.pixelSize: 15
                font.weight: Font.Bold
            }
            AbstractButton {
                id: open
                hoverEnabled: true
                text: Tr.t.open_queue
                Accessible.role: Accessible.Link
                onClicked: panel.navigate("queue")

                contentItem: Label {
                    text: open.text
                    color: open.hovered ? Theme.text : Theme.text2
                    font.pixelSize: 13
                    font.underline: true
                }
                background: Rectangle {
                    color: "transparent"
                    radius: 4
                    border.width: open.visualFocus ? 2 : 0
                    border.color: Theme.text
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: -6
            Layout.rightMargin: -6

            ListView {
                id: next
                anchors.fill: parent
                clip: true
                spacing: 2
                bottomMargin: 16
                model: queue
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds
                Accessible.name: Tr.t.up_next

                delegate: AbstractButton {
                    id: entry

                    required property int index
                    required property string title
                    required property string artist
                    required property string length
                    required property string cover
                    required property bool queued

                    width: ListView.view.width
                    height: AppState.compactRows ? 42 : 52
                    leftPadding: 6
                    rightPadding: 6
                    hoverEnabled: true
                    text: title
                    onDoubleClicked: queue.play(index)

                    background: Rectangle {
                        radius: 8
                        color: entry.hovered ? Theme.hover : "transparent"
                    }
                    contentItem: RowLayout {
                        spacing: 10

                        Cover {
                            size: AppState.compactRows ? 32 : 40
                            radius: 4
                            path: entry.cover
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: AppState.compactRows ? 0 : 2

                            Label {
                                Layout.fillWidth: true
                                text: entry.title
                                elide: Text.ElideRight
                                color: Theme.text
                                font.pixelSize: 14
                                font.weight: Font.DemiBold
                            }
                            Label {
                                Layout.fillWidth: true
                                text: entry.artist === "" ? Tr.t.unknown_artist : entry.artist
                                elide: Text.ElideRight
                                color: Theme.text2
                                font.pixelSize: 12
                            }
                        }
                        Label {
                            text: entry.queued ? Tr.t.queued : entry.length
                            color: entry.queued ? Theme.accentText : Theme.text3
                            font.pixelSize: entry.queued ? 11 : 12
                            font.weight: entry.queued ? Font.DemiBold : Font.Normal
                            font.capitalization: entry.queued ? Font.AllUppercase : Font.MixedCase
                            font.features: {
                                "tnum": 1
                            }
                        }
                    }
                }

                ScrollBar.vertical: ThinScrollBar {}
            }

            Label {
                anchors.horizontalCenter: parent.horizontalCenter
                y: 12
                visible: next.count === 0
                text: Tr.t.nothing_up_next
                color: Theme.text3
                font.pixelSize: 13
            }
        }
    }
}
