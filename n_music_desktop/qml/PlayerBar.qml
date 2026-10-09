import QtQuick
import QtQuick.Controls.Basic
import NMusic

// The current track and the playback controls, along the bottom of the window; narrow windows
// get a mini player, its track opening the queue.
Rectangle {
    id: bar

    /// The queue page is open.
    property bool queueOpen: false

    /// Lights the queue's button up for a moment, after a track was queued out of sight: it
    /// turns the accent over 150 ms, holds and turns back.
    function ping() {
        pinged.restart();
    }

    signal toggleQueue
    /// The mini player was asked for.
    signal miniRequested

    /// Width of the side columns; the middle one is 1.6 times as wide.
    readonly property real unit: Math.max(0, width - 40 - 48) / 3.6

    implicitHeight: Shell.narrow ? 72 : 88
    color: Theme.surface

    // How far the track got, smoothly between the player's reports.
    PlayClock {
        id: clock
        running: bar.Window.visibility !== Window.Minimized && bar.Window.visibility !== Window.Hidden
    }

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

        CoverSwap {
            id: art
            anchors.verticalCenter: parent.verticalCenter
            size: 56
            radius: 6
        }
        TrackSwap {
            anchors.left: art.right
            anchors.leftMargin: 14
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter

            delegate: Column {
                id: words

                required property var track

                spacing: 3

                Label {
                    width: words.width
                    text: words.track.loaded ? words.track.title : Tr.t.not_playing
                    elide: Text.ElideRight
                    color: words.track.loaded ? Theme.text : Theme.text3
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                }
                Label {
                    width: words.width
                    visible: words.track.loaded
                    text: [words.track.artist, words.track.album].filter(part => part !== "").join(" · ")
                    elide: Text.ElideRight
                    color: Theme.text2
                    font.pixelSize: 13
                }
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
                scale: play.down ? 0.9 : 1
                text: Player.playing ? Tr.t.pause : Tr.t.play
                Accessible.name: text
                onClicked: Player.toggle()

                PressScale on scale {}

                background: Rectangle {
                    radius: 22
                    color: Theme.text
                    border.width: play.visualFocus ? 2 : 0
                    border.color: Theme.accent
                }
                contentItem: Item {
                    PlayPauseIcon {
                        anchors.centerIn: parent
                        playing: Player.playing
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
                    value: clock.position
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
            color: bar.queueOpen || pinged.running ? Theme.accentText : Theme.text2
            text: Tr.t.queue
            onClicked: bar.toggleQueue()

            Timer {
                id: pinged
                interval: 750
            }
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
                width: Player.length > 0 ? parent.width * Math.min(1, clock.position / Player.length) : 0
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
                color: current.down ? Theme.selected : current.hovered ? Theme.hover : Qt.alpha(Theme.hover, 0)
                border.width: current.visualFocus ? 2 : 0
                border.color: Theme.text

                TintFade on color {}
            }
            contentItem: Row {
                spacing: 10

                CoverSwap {
                    anchors.verticalCenter: parent.verticalCenter
                    size: 44
                    radius: 5
                }
                TrackSwap {
                    anchors.verticalCenter: parent.verticalCenter
                    width: current.width - 44 - 10

                    delegate: Column {
                        id: brief

                        required property var track

                        spacing: 2

                        Label {
                            width: brief.width
                            text: brief.track.loaded ? brief.track.title : Tr.t.not_playing
                            elide: Text.ElideRight
                            color: brief.track.loaded ? Theme.text : Theme.text3
                            font.pixelSize: 14
                            font.weight: Font.DemiBold
                        }
                        Label {
                            width: brief.width
                            visible: brief.track.loaded && text !== ""
                            text: brief.track.artist
                            elide: Text.ElideRight
                            color: Theme.text2
                            font.pixelSize: 12
                        }
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
                scale: toggle.down ? 0.9 : 1
                text: Player.playing ? Tr.t.pause : Tr.t.play
                Accessible.name: text
                onClicked: Player.toggle()

                PressScale on scale {}

                background: Rectangle {
                    radius: 22
                    color: Theme.text
                    border.width: toggle.visualFocus ? 2 : 0
                    border.color: Theme.accent
                }
                contentItem: Item {
                    PlayPauseIcon {
                        anchors.centerIn: parent
                        playing: Player.playing
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
