import QtQuick
import QtQuick.Controls.Basic
import NMusic

// The current track and the playback controls, along the bottom of the window.
Rectangle {
    id: bar

    /// The queue page is open.
    property bool queueOpen: false

    signal toggleQueue

    /// Width of the side columns; the middle one is 1.6 times as wide.
    readonly property real unit: Math.max(0, width - 40 - 48) / 3.6

    implicitHeight: 88
    color: Theme.surface

    Rectangle {
        width: parent.width
        height: 1
        color: Theme.line
    }

    Item {
        x: 20
        width: bar.unit
        height: parent.height

        Cover {
            id: art
            anchors.verticalCenter: parent.verticalCenter
            size: 56
            radius: 6
            path: Player.cover
        }
        Column {
            anchors.left: art.right
            anchors.leftMargin: 14
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: 3

            Label {
                width: parent.width
                text: Player.loaded ? Player.title : Tr.t.not_playing
                elide: Text.ElideRight
                color: Player.loaded ? Theme.text : Theme.text3
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }
            Label {
                width: parent.width
                visible: Player.loaded
                text: [Player.artist, Player.album].filter(part => part !== "").join(" · ")
                elide: Text.ElideRight
                color: Theme.text2
                font.pixelSize: 13
            }
        }
    }

    Column {
        x: 20 + bar.unit + 24
        width: bar.unit * 1.6
        anchors.verticalCenter: parent.verticalCenter
        spacing: 6

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 10

            IconButton {
                anchors.verticalCenter: parent.verticalCenter
                size: 36
                iconSize: 18
                iconName: "shuffle"
                color: Player.shuffle ? Theme.accentText : Theme.text2
                text: Player.shuffle ? Tr.t.shuffle_on : Tr.t.shuffle_off
                onClicked: Player.toggleShuffle()
            }
            IconButton {
                anchors.verticalCenter: parent.verticalCenter
                iconName: "previous"
                stroke: 2
                color: Theme.text
                text: Tr.t.previous
                onClicked: Player.previous()
            }
            AbstractButton {
                id: play
                anchors.verticalCenter: parent.verticalCenter
                implicitWidth: 44
                implicitHeight: 44
                hoverEnabled: true
                scale: play.down ? 0.96 : play.hovered ? 1.04 : 1
                text: Player.playing ? Tr.t.pause : Tr.t.play
                Accessible.name: text
                onClicked: Player.toggle()

                Behavior on scale {
                    NumberAnimation {
                        duration: 90
                    }
                }

                background: Rectangle {
                    radius: 22
                    color: Theme.text
                    border.width: play.visualFocus ? 2 : 0
                    border.color: Theme.accent
                }
                contentItem: Item {
                    Icon {
                        anchors.centerIn: parent
                        // The triangle's weight sits left of its box.
                        anchors.horizontalCenterOffset: Player.playing ? 0 : 1
                        name: Player.playing ? "pause" : "play"
                        size: 20
                        color: Theme.surface
                    }
                }
            }
            IconButton {
                anchors.verticalCenter: parent.verticalCenter
                iconName: "next"
                stroke: 2
                color: Theme.text
                text: Tr.t.next
                onClicked: Player.next()
            }
            IconButton {
                anchors.verticalCenter: parent.verticalCenter
                size: 36
                iconSize: 18
                iconName: Player.repeat === 2 ? "repeat-one" : "repeat"
                color: Player.repeat > 0 ? Theme.accentText : Theme.text2
                text: [Tr.t.repeat_off, Tr.t.repeat_all, Tr.t.repeat_one][Player.repeat]
                onClicked: Player.cycleRepeat()
            }
        }

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(parent.width, 560)
            spacing: 10

            Label {
                width: 36
                anchors.verticalCenter: parent.verticalCenter
                horizontalAlignment: Text.AlignRight
                text: Format.clock(progress.pressed ? progress.value : Player.position)
                color: Theme.text2
                font.pixelSize: 12
                font.features: {
                    "tnum": 1
                }
            }
            BarSlider {
                id: progress
                width: parent.width - 2 * 36 - 2 * parent.spacing
                anchors.verticalCenter: parent.verticalCenter
                from: 0
                to: Math.max(Player.length, 1)
                enabled: Player.loaded && Player.length > 0
                Accessible.name: Tr.t.position
                // Pressing seeks on release; keys seek at once.
                onPressedChanged: {
                    if (!pressed)
                        Player.seek(value);
                }
                onMoved: {
                    if (!pressed)
                        Player.seek(value);
                }

                Binding on value {
                    when: !progress.pressed
                    value: Player.position
                    restoreMode: Binding.RestoreNone
                }
            }
            Label {
                width: 36
                anchors.verticalCenter: parent.verticalCenter
                text: Format.clock(Math.round(Player.length))
                color: Theme.text2
                font.pixelSize: 12
                font.features: {
                    "tnum": 1
                }
            }
        }
    }

    Row {
        anchors.right: parent.right
        anchors.rightMargin: 20
        anchors.verticalCenter: parent.verticalCenter
        spacing: 6

        IconButton {
            anchors.verticalCenter: parent.verticalCenter
            iconName: "queue"
            color: bar.queueOpen ? Theme.accentText : Theme.text2
            text: Tr.t.queue
            onClicked: bar.toggleQueue()
        }
        IconButton {
            anchors.verticalCenter: parent.verticalCenter
            iconName: Player.volume > 0 ? "volume" : "mute"
            text: Player.volume > 0 ? Tr.t.mute : Tr.t.unmute
            onClicked: Player.toggleMute()
        }
        BarSlider {
            id: volume
            width: 110
            anchors.verticalCenter: parent.verticalCenter
            from: 0
            to: 1
            fill: Theme.text2
            knobOnHover: true
            Accessible.name: Tr.t.volume
            onMoved: Player.changeVolume(value)

            Binding on value {
                when: !volume.pressed
                value: Player.volume
                restoreMode: Binding.RestoreNone
            }
        }
    }
}
