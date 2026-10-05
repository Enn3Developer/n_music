import QtQuick
import QtQuick.Controls.Basic
import NMusic

// The current track and the playback controls, along the bottom of the window; narrow windows
// get a mini player, its track opening the queue.
Rectangle {
    id: bar

    /// The queue page is open.
    property bool queueOpen: false

    signal toggleQueue
    /// The mini player was asked for.
    signal miniRequested

    /// Width of the side columns; the middle one is 1.6 times as wide.
    readonly property real unit: Math.max(0, width - 40 - 48) / 3.6

    implicitHeight: Shell.narrow ? 72 : 88
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
        visible: !Shell.narrow

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
        visible: !Shell.narrow
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
        visible: !Shell.narrow
        spacing: 6

        IconButton {
            anchors.verticalCenter: parent.verticalCenter
            iconName: "mini-player"
            color: Theme.text2
            text: Tr.t.mini_player
            onClicked: bar.miniRequested()
        }
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

    Item {
        anchors.fill: parent
        anchors.topMargin: 1
        visible: Shell.narrow

        // How far the track got.
        Rectangle {
            width: parent.width
            height: 3
            color: Theme.track

            Rectangle {
                width: Player.length > 0 ? parent.width * Math.min(1, Player.position / Player.length) : 0
                height: parent.height
                color: Theme.accent
            }
        }

        AbstractButton {
            id: current
            x: 12
            y: 3
            width: mini.x - 10 - x
            height: parent.height - 3
            hoverEnabled: true
            text: Tr.t.queue
            Accessible.name: (Player.loaded ? Player.title + ", " : "") + text
            onClicked: bar.toggleQueue()

            background: Rectangle {
                anchors.fill: parent
                anchors.topMargin: 8
                anchors.bottomMargin: 8
                anchors.leftMargin: -6
                radius: 8
                color: current.down ? Theme.selected : current.hovered ? Theme.hover : "transparent"
                border.width: current.visualFocus ? 2 : 0
                border.color: Theme.text
            }
            contentItem: Row {
                spacing: 10

                Cover {
                    anchors.verticalCenter: parent.verticalCenter
                    size: 44
                    radius: 5
                    path: Player.cover
                }
                Column {
                    anchors.verticalCenter: parent.verticalCenter
                    width: current.width - 44 - 10
                    spacing: 2

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
                        visible: Player.loaded && text !== ""
                        text: Player.artist
                        elide: Text.ElideRight
                        color: Theme.text2
                        font.pixelSize: 12
                    }
                }
            }
        }

        Row {
            id: mini
            anchors.right: parent.right
            anchors.rightMargin: 8
            anchors.verticalCenter: current.verticalCenter
            spacing: 10

            AbstractButton {
                id: toggle
                implicitWidth: 44
                implicitHeight: 44
                hoverEnabled: true
                text: Player.playing ? Tr.t.pause : Tr.t.play
                Accessible.name: text
                onClicked: Player.toggle()

                background: Rectangle {
                    radius: 22
                    color: toggle.down ? Qt.darker(Theme.text, 1.1) : Theme.text
                    border.width: toggle.visualFocus ? 2 : 0
                    border.color: Theme.accent
                }
                contentItem: Item {
                    Icon {
                        anchors.centerIn: parent
                        anchors.horizontalCenterOffset: Player.playing ? 0 : 1
                        name: Player.playing ? "pause" : "play"
                        size: 18
                        color: Theme.surface
                    }
                }
            }
            IconButton {
                size: 44
                radius: 10
                iconName: "next"
                stroke: 2
                color: Theme.text
                text: Tr.t.next
                onClicked: Player.next()
            }
        }
    }
}
