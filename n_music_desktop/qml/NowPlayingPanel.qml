pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// Beside the page in wide windows: the current track, large, and what plays after it.
Rectangle {
    id: panel

    signal navigate(string page)

    /// Whether the place in Up next of a track queued now shows: first when it plays `next`,
    /// else after the queued ones.
    function shows(next: bool): bool {
        if (!visible)
            return false;
        if (list.count === 0)
            return true;
        const row = list.itemAtIndex(Math.min(next ? 0 : queue.upNextCount, list.count - 1));
        return row !== null && row.y >= list.contentY && row.y + row.height <= list.contentY + list.height;
    }

    implicitWidth: 360
    color: Theme.panel

    QueueList {
        id: queue
        fades: true
        onReplacing: rowMotion.fadeThrough()
    }

    RowTransitions {
        id: rowMotion
        view: list
        queue: queue
        pitch: (AppState.compactRows ? 42 : 52) + list.spacing
        slidesOnSkip: true
        // A new list shows from its start.
        onSwapped: list.positionViewAtBeginning()
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

        CoverSwap {
            Layout.alignment: Qt.AlignHCenter
            // Leaves room for a few tracks after it in short windows.
            size: Math.min(panel.width - 40, Math.max(120, panel.height - 400))
            radius: 12
        }

        TrackSwap {
            Layout.fillWidth: true

            delegate: ColumnLayout {
                id: words

                required property var track

                spacing: 4

                Label {
                    Layout.fillWidth: true
                    text: words.track.loaded ? words.track.title : Tr.t.not_playing
                    wrapMode: Text.Wrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    color: words.track.loaded ? Theme.text : Theme.text3
                    font.pixelSize: 20
                    font.weight: Font.Bold
                }
                Label {
                    Layout.fillWidth: true
                    visible: text !== ""
                    text: [words.track.artist, words.track.album, words.track.year].filter(part => part !== "").join(" · ")
                    elide: Text.ElideRight
                    color: Theme.text2
                    font.pixelSize: 14
                }
                Label {
                    Layout.fillWidth: true
                    Layout.topMargin: 4
                    visible: text !== ""
                    text: [Format.audio(words.track.codec, words.track.sampleRate, words.track.bits), Format.gain(words.track.albumGain, words.track.trackGain)].filter(part => part !== "").join(" · ")
                    elide: Text.ElideRight
                    color: Theme.text3
                    font.pixelSize: 12
                }
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

                    ColorFade on color {}
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
                id: list
                anchors.fill: parent
                clip: true
                spacing: 2
                bottomMargin: 16
                model: queue
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds
                Accessible.name: Tr.t.up_next
                add: rowMotion.add
                remove: rowMotion.remove
                addDisplaced: rowMotion.addDisplaced
                removeDisplaced: rowMotion.removeDisplaced
                moveDisplaced: rowMotion.moveDisplaced
                move: rowMotion.move
                populate: rowMotion.populate

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
                    // A row taken out faded; back from the pool it shows again.
                    ListView.onReused: entry.opacity = 1

                    /// Tints the row for a moment, so the eye finds it: it holds for 200 ms
                    /// and fades over 400 ms.
                    function tint() {
                        flash.restart();
                    }

                    SequentialAnimation {
                        id: flash

                        PropertyAction {
                            target: tinted
                            property: "opacity"
                            value: 1
                        }
                        PauseAnimation {
                            duration: Motion.move
                        }
                        OpacityAnimator {
                            target: tinted
                            to: 0
                            duration: 400
                        }
                    }

                    background: Rectangle {
                        radius: 8
                        color: entry.hovered ? Theme.hover : Qt.alpha(Theme.hover, 0)

                        TintFade on color {}

                        Rectangle {
                            id: tinted
                            anchors.fill: parent
                            radius: parent.radius
                            color: Qt.alpha(Theme.accent, 0.12)
                            opacity: 0
                        }
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
                visible: list.count === 0
                text: Tr.t.nothing_up_next
                color: Theme.text3
                font.pixelSize: 13
            }
        }
    }
}
