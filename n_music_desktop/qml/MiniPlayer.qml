import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// The player alone in a small window kept over the others, for when the library can hide.
ApplicationWindow {
    id: mini

    /// Asks for the full window back, from its button or by closing this one.
    signal expandRequested

    width: 420
    height: 128
    minimumWidth: 360
    maximumWidth: 640
    minimumHeight: 128
    maximumHeight: 128
    // Its own window, not one tied to the full window, which hides while it shows.
    transientParent: null
    flags: AppState.miniOnTop ? Qt.Window | Qt.WindowStaysOnTopHint : Qt.Window
    title: Player.loaded ? Player.title + " – N Music" : "N Music"
    color: Theme.surface
    font.family: Theme.font
    font.pixelSize: 14
    onClosing: mini.expandRequested()

    Shortcut {
        sequence: "Space"
        onActivated: Player.toggle()
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 14

        Cover {
            size: 104
            radius: 8
            path: Player.cover
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignTop
                    spacing: 2

                    Label {
                        Layout.fillWidth: true
                        text: Player.loaded ? Player.title : Tr.t.not_playing
                        elide: Text.ElideRight
                        color: Player.loaded ? Theme.text : Theme.text3
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: Player.loaded && text !== ""
                        text: Player.artist
                        elide: Text.ElideRight
                        color: Theme.text2
                        font.pixelSize: 13
                    }
                }
                IconButton {
                    Layout.alignment: Qt.AlignTop
                    size: 32
                    iconSize: 16
                    stroke: 2
                    iconName: "expand"
                    text: Tr.t.full_window
                    onClicked: mini.expandRequested()
                }
            }

            Item {
                Layout.fillHeight: true
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Label {
                    text: Format.clock(progress.pressed ? progress.value : Player.position)
                    color: Theme.text2
                    font.pixelSize: 11
                    font.features: {
                        "tnum": 1
                    }
                }
                BarSlider {
                    id: progress
                    Layout.fillWidth: true
                    from: 0
                    to: Math.max(Player.length, 1)
                    enabled: Player.loaded && Player.length > 0
                    knobOnHover: true
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
                    text: Format.clock(Math.round(Player.length))
                    color: Theme.text2
                    font.pixelSize: 11
                    font.features: {
                        "tnum": 1
                    }
                }
            }

            Item {
                Layout.fillHeight: true
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 0

                IconButton {
                    size: 32
                    iconSize: 16
                    stroke: 1.9
                    iconName: "shuffle"
                    color: Player.shuffle ? Theme.accentText : Theme.text2
                    text: Player.shuffle ? Tr.t.shuffle_on : Tr.t.shuffle_off
                    onClicked: Player.toggleShuffle()
                }
                Item {
                    Layout.fillWidth: true
                }
                IconButton {
                    size: 36
                    iconSize: 18
                    stroke: 2
                    iconName: "previous"
                    color: Theme.text
                    text: Tr.t.previous
                    onClicked: Player.previous()
                }
                Item {
                    Layout.fillWidth: true
                }
                AbstractButton {
                    id: toggle
                    implicitWidth: 40
                    implicitHeight: 40
                    hoverEnabled: true
                    text: Player.playing ? Tr.t.pause : Tr.t.play
                    Accessible.name: text
                    onClicked: Player.toggle()

                    background: Rectangle {
                        radius: 20
                        color: toggle.down ? Qt.darker(Theme.text, 1.1) : Theme.text
                        border.width: toggle.visualFocus ? 2 : 0
                        border.color: Theme.accent
                    }
                    contentItem: Item {
                        Icon {
                            anchors.centerIn: parent
                            // The triangle's weight sits left of its box.
                            anchors.horizontalCenterOffset: Player.playing ? 0 : 1
                            name: Player.playing ? "pause" : "play"
                            size: 18
                            color: Theme.surface
                        }
                    }
                }
                Item {
                    Layout.fillWidth: true
                }
                IconButton {
                    size: 36
                    iconSize: 18
                    stroke: 2
                    iconName: "next"
                    color: Theme.text
                    text: Tr.t.next
                    onClicked: Player.next()
                }
                Item {
                    Layout.fillWidth: true
                }
                IconButton {
                    size: 32
                    iconSize: 16
                    stroke: 1.9
                    iconName: Player.repeat === 2 ? "repeat-one" : "repeat"
                    color: Player.repeat > 0 ? Theme.accentText : Theme.text2
                    text: [Tr.t.repeat_off, Tr.t.repeat_all, Tr.t.repeat_one][Player.repeat]
                    onClicked: Player.cycleRepeat()
                }
            }
        }
    }
}
