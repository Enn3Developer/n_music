pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// The play session: the current track and how playback ends beside what plays next.
Item {
    id: page

    signal navigate(string page)

    readonly property real nowWidth: Math.max(300, Math.min(360, width - 56 - 32 - 420))

    readonly property string format: Format.audio(Player.codec, Player.sampleRate, Player.bits)
    readonly property string gain: Format.gain(Player.albumGain, Player.trackGain)

    readonly property string contextDescription: [queue.contextLabel === "" ? Tr.t.library : queue.contextLabel, queue.contextDetail, Player.shuffle ? Tr.t.shuffled : "", Tr.t.left_count.arg(Format.number(queue.leftCount))].filter(part => part !== "").join(" · ")

    QueueList {
        id: queue
    }

    Flickable {
        x: 28
        width: page.nowWidth
        height: parent.height
        contentHeight: now.height + 28 + 24
        clip: true
        interactive: contentHeight > height
        boundsBehavior: Flickable.StopAtBounds

        Column {
            id: now
            y: 28
            width: parent.width
            spacing: 18

            Cover {
                size: now.width
                radius: 12
                path: Player.cover
            }

            Column {
                width: now.width
                spacing: 6

                Label {
                    text: Tr.t.now_playing
                    color: Theme.accentText
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.96
                    font.capitalization: Font.AllUppercase
                }
                Label {
                    width: now.width
                    text: Player.loaded ? Player.title : Tr.t.not_playing
                    wrapMode: Text.Wrap
                    maximumLineCount: 3
                    elide: Text.ElideRight
                    color: Player.loaded ? Theme.text : Theme.text3
                    font.pixelSize: 26
                    font.weight: Font.Bold
                    font.letterSpacing: -0.26
                }
                Label {
                    width: now.width
                    visible: text !== ""
                    text: [Player.artist, Player.album, Player.year].filter(part => part !== "").join(" · ")
                    wrapMode: Text.Wrap
                    color: Theme.text2
                    font.pixelSize: 15
                }
            }

            Flow {
                width: now.width
                spacing: 6
                visible: Player.loaded

                Chip {
                    visible: text !== ""
                    text: page.format
                }
                Chip {
                    visible: text !== ""
                    text: page.gain
                }
                Chip {
                    text: Tr.t.played_times.arg(Format.number(Player.plays))
                }
            }

            Column {
                width: now.width
                topPadding: 4
                spacing: 10

                Label {
                    text: Tr.t.at_the_end
                    color: Theme.text3
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.72
                    font.capitalization: Font.AllUppercase
                }
                SegmentedControl {
                    width: now.width
                    options: [Tr.t.end_stop, Tr.t.end_repeat_all, Tr.t.end_repeat_one]
                    current: Player.repeat
                    Accessible.name: Tr.t.at_the_end
                    onActivated: index => Player.changeRepeat(index)
                }
                Item {
                    width: now.width
                    height: 40

                    Label {
                        x: 4
                        anchors.verticalCenter: parent.verticalCenter
                        text: Tr.t.shuffle
                        color: Theme.text
                        font.pixelSize: 14
                    }
                    ToggleSwitch {
                        anchors.right: parent.right
                        anchors.rightMargin: 4
                        anchors.verticalCenter: parent.verticalCenter
                        checked: Player.shuffle
                        Accessible.name: Tr.t.shuffle
                        onToggled: Player.changeShuffle(checked)
                    }
                }
            }
        }
    }

    FlatButton {
        id: history
        x: list.x
        y: 28
        visible: queue.historyCount > 0
        filled: true
        iconName: queue.showHistory ? "chevron-down" : "chevron-right"
        text: Tr.t.history_played.arg(Format.number(queue.historyCount))
        onClicked: queue.showHistory = !queue.showHistory
    }

    ListView {
        id: list
        x: 28 + page.nowWidth + 32
        y: history.visible ? history.y + history.height + 16 : 0
        width: page.width - x - 28
        height: parent.height - y
        topMargin: history.visible ? 0 : 28
        bottomMargin: 24
        clip: true
        spacing: 6
        model: queue
        reuseItems: true
        boundsBehavior: Flickable.StopAtBounds
        Accessible.name: Tr.t.queue

        section.property: "section"
        section.delegate: SectionHeader {}

        delegate: QueueRow {
            width: ListView.view.width
            onActivated: queue.play(index)
            onRemove: queue.remove(index)
        }

        ScrollBar.vertical: ThinScrollBar {}
    }

    EmptyState {
        anchors.centerIn: list
        visible: list.count === 0 && !Player.loaded
        iconName: "queue"
        title: Tr.t.queue_empty
        message: Tr.t.queue_empty_hint
        action: Tr.t.open_tracks
        onTriggered: page.navigate("tracks")
    }

    // ListView shows section headers itself, so history's empty one hides its content.
    component SectionHeader: Item {
        id: header

        required property string section
        /// Something is listed above: shown history, or up next above the context.
        readonly property bool below: section === "next" ? queue.showHistory && queue.historyCount > 0 : (queue.showHistory && queue.historyCount > 0) || queue.upNextCount > 0

        width: list.width
        height: content.visible ? content.implicitHeight : 0

        Column {
            id: content
            width: parent.width
            visible: header.section !== "history"
            topPadding: header.below ? 16 : 0
            bottomPadding: 6
            spacing: 6

            Item {
                width: parent.width
                height: 32

                Label {
                    anchors.verticalCenter: parent.verticalCenter
                    text: header.section === "next" ? Tr.t.up_next : Tr.t.playing_from
                    color: Theme.text
                    font.pixelSize: 18
                    font.weight: Font.Bold
                }
                FlatButton {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    visible: header.section === "next"
                    text: Tr.t.clear
                    onClicked: queue.clearUpNext()
                }
                AbstractButton {
                    id: link
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    visible: header.section === "context"
                    hoverEnabled: true
                    text: Tr.t.open_in_library
                    onClicked: page.navigate(queue.contextPage)

                    background: null
                    contentItem: Label {
                        text: link.text
                        color: link.hovered ? Theme.text : Theme.text2
                        font.pixelSize: 13
                        font.underline: true
                    }
                }
            }
            Label {
                width: parent.width
                text: header.section === "next" ? Tr.t.up_next_hint : page.contextDescription
                elide: Text.ElideRight
                color: header.section === "next" ? Theme.text3 : Theme.text2
                font.pixelSize: 13
            }
        }
    }
}
