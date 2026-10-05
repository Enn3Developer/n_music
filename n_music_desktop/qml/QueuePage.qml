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

    /// A row is being dragged to another place: `held`, its top showing at `heldTop` in the
    /// list's content.
    property bool dragging: false
    property QueueRow held: null
    property real heldTop: 0
    /// How far below its top it was picked up, and where the pointer is in the scene.
    property real grip: 0
    property real pointer: 0
    /// How far apart rows stand.
    property real stride: 0
    /// The held row eases into its place once let go.
    property bool settling: false
    /// Pixels a second the list scrolls by while the pointer carries a row near its top or bottom.
    readonly property real scrollSpeed: {
        if (!dragging)
            return 0;
        const y = list.mapFromItem(null, 0, pointer).y;
        const edge = Math.min(64, list.height / 4);
        if (y < edge)
            return -1200 * Math.min(1, (edge - y) / edge);
        if (y > list.height - edge)
            return 1200 * Math.min(1, (y - list.height + edge) / edge);
        return 0;
    }

    function pickUp(row: QueueRow, offset: real, sceneY: real) {
        dragging = true;
        held = row;
        grip = offset;
        pointer = sceneY;
        stride = row.height + list.spacing;
        queue.startDrag(row.index);
        follow();
    }

    // Keeps the held row under the pointer, anywhere for the current one, among the rows still to
    // play for the others, and moves it to the place it shows over. Places count from the held
    // row's own: the view shifts its rows as it scrolls far, so places measured before go stale.
    function follow() {
        if (!dragging || held === null)
            return;
        // Past the edges the list scrolls instead.
        const top = Math.max(0, Math.min(list.height, list.mapFromItem(null, 0, pointer).y)) + list.contentY - grip;
        const first = held.y + ((held.current ? 0 : list.count - queue.leftCount) - held.index) * stride;
        const last = held.y + (list.count - 1 - held.index) * stride;
        heldTop = Math.max(first, Math.min(last, top));
        // The view lets go of a row moved to a place out of sight.
        const shown = Math.max(list.contentY, Math.min(list.contentY + list.height - held.height, heldTop));
        const to = held.index + Math.round((shown - held.y) / stride);
        if (to !== held.index) {
            queue.dragTo(to);
            // Lays the move out now, for the held row's place to count from.
            list.forceLayout();
        }
    }

    function drop(keep: bool) {
        if (!dragging)
            return;
        dragging = false;
        queue.endDrag(keep);
        settling = true;
        held = null;
        settling = false;
    }

    QueueList {
        id: queue
        showCurrent: true
        moving: displaced.running
    }

    // Scrolls the list under a row carried near its edges.
    FrameAnimation {
        id: scroller
        running: page.scrollSpeed < 0 ? !list.atYBeginning : page.scrollSpeed > 0 && !list.atYEnd
        onTriggered: {
            const top = list.originY - list.topMargin;
            const bottom = Math.max(top, list.originY + list.contentHeight + list.bottomMargin - list.height);
            list.contentY = Math.max(top, Math.min(bottom, list.contentY + page.scrollSpeed * scroller.frameTime));
        }
    }

    // Escape puts a dragged row back.
    Shortcut {
        sequences: [StandardKey.Cancel]
        enabled: page.dragging
        onActivated: page.drop(false)
    }

    NarrowBar {
        id: bar
        width: page.width
        visible: Shell.narrow
        title: Tr.t.queue
    }

    Flickable {
        id: side
        x: 28
        width: page.nowWidth
        height: parent.height
        visible: !Shell.narrow
        contentHeight: now.height + 28 + 24
        clip: true
        interactive: contentHeight > height
        boundsBehavior: Flickable.StopAtBounds

        // Atop the list in narrow windows, scrolling with it.
        Column {
            id: now
            parent: Shell.narrow && list.headerItem ? list.headerItem : side.contentItem
            y: Shell.narrow ? 16 : 28
            width: parent.width
            spacing: 18

            Cover {
                // Shrinks in short windows to keep the controls below it in view.
                size: Shell.narrow ? Math.min(now.width, 240) : Math.max(160, Math.min(now.width, page.height - 28 - 24 - 3 * now.spacing - about.height - (chips.visible ? chips.height + now.spacing : 0) - ending.height))
                radius: 12
                path: Player.cover
            }

            Column {
                id: about
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
                id: chips
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
                id: ending
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
        parent: Shell.narrow && list.headerItem ? list.headerItem : page
        x: Shell.narrow ? 0 : list.x
        y: Shell.narrow ? now.y + now.height + 24 : 28
        visible: queue.historyCount > 0
        filled: true
        iconName: queue.showHistory ? "chevron-down" : "chevron-right"
        text: Tr.t.history_played.arg(Format.number(queue.historyCount))
        onClicked: queue.showHistory = !queue.showHistory
    }

    ListView {
        id: list
        x: Shell.narrow ? 16 : 28 + page.nowWidth + 32
        y: Shell.narrow ? bar.height : history.visible ? history.y + history.height + 16 : 0
        width: Shell.narrow ? page.width - 32 : page.width - x - 28
        height: parent.height - y
        topMargin: history.visible || Shell.narrow ? 0 : 28
        header: Shell.narrow ? nowHolder : null
        bottomMargin: 24
        clip: true
        spacing: 6
        model: queue
        reuseItems: true
        // The view lets go of a row moved past those it built, so a dragged one keeps some built
        // past either edge.
        displayMarginBeginning: page.dragging ? 2 * page.stride : 0
        displayMarginEnd: page.dragging ? 2 * page.stride : 0
        boundsBehavior: Flickable.StopAtBounds
        Accessible.name: Tr.t.queue

        /// How far the rows are scrolled from their top, margin included; kept while the margin
        /// changes.
        property real scrolled: 0
        /// The margin above the rows, as last seen.
        property real seenTopMargin: 0

        function trackScroll() {
            // Flickable moves the rows into new bounds before telling of a new margin.
            if (topMargin === seenTopMargin)
                scrolled = contentY - originY + topMargin;
        }

        Component.onCompleted: {
            seenTopMargin = topMargin;
            trackScroll();
        }
        onOriginYChanged: trackScroll()
        onContentYChanged: {
            trackScroll();
            // Scrolling happens in the view's layout too, where its model must not change.
            Qt.callLater(page.follow);
        }
        // The margin comes and goes with the history button: the rows keep their place under it.
        onTopMarginChanged: {
            seenTopMargin = topMargin;
            contentY = originY - topMargin + scrolled;
        }

        section.property: "section"
        section.delegate: SectionHeader {}

        delegate: QueueRow {
            id: entry
            width: ListView.view.width
            dragged: page.held === entry
            lift: dragged ? page.heldTop - y : 0
            onActivated: queue.play(index)
            onRemove: queue.remove(index)
            onPickedUp: (offset, sceneY) => page.pickUp(entry, offset, sceneY)
            onCarried: sceneY => {
                page.pointer = sceneY;
                page.follow();
            }
            onDropped: page.drop(true)
            // These can come while the view lays its rows out, which must not change its model.
            onDragCanceled: Qt.callLater(page.drop, false)
            ListView.onPooled: {
                if (entry.dragged)
                    Qt.callLater(page.drop, false);
            }
            Component.onDestruction: {
                if (entry.dragged)
                    Qt.callLater(page.drop, false);
            }

            Behavior on lift {
                enabled: page.settling
                NumberAnimation {
                    duration: 180
                    easing.type: Easing.OutCubic
                }
            }
        }
        // Rows make way for the one dragged over them.
        moveDisplaced: Transition {
            id: displaced

            NumberAnimation {
                property: "y"
                duration: 160
                easing.type: Easing.OutCubic
            }
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

    // Holds what plays now and the history button in narrow windows.
    Component {
        id: nowHolder

        Item {
            /// The height the view last saw.
            property real seen: height

            width: list.width
            height: now.y + now.height + 24 + (history.visible ? history.height + 16 : 0)
            // The view keeps its position as this grows above the tracks, which would scroll
            // this out of view while it shows.
            onHeightChanged: {
                if (list.contentY < 0)
                    list.contentY -= height - seen;
                seen = height;
            }
        }
    }

    // ListView shows section headers itself, so history's empty one hides its content.
    component SectionHeader: Item {
        id: header

        required property string section

        width: list.width
        height: content.visible ? content.implicitHeight : 0

        Column {
            id: content
            width: parent.width
            visible: header.section === "next"
            // Apart from the history shown above.
            topPadding: queue.showHistory && queue.historyCount > 0 ? 16 : 0
            bottomPadding: 6
            spacing: 6

            Item {
                width: parent.width
                height: 32

                Label {
                    anchors.left: parent.left
                    anchors.right: actions.left
                    anchors.rightMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    text: Tr.t.up_next
                    elide: Text.ElideRight
                    color: Theme.text
                    font.pixelSize: 18
                    font.weight: Font.Bold
                }
                Row {
                    id: actions
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 14

                    FlatButton {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: queue.upNextCount > 0
                        text: Tr.t.clear_queued
                        onClicked: queue.clearUpNext()
                    }
                    AbstractButton {
                        id: link
                        anchors.verticalCenter: parent.verticalCenter
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
            }
            Label {
                width: parent.width
                text: Tr.t.playing_from.arg(page.contextDescription)
                elide: Text.ElideRight
                color: Theme.text2
                font.pixelSize: 13
            }
        }
    }
}
