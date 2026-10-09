pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// The library's albums, artists or genres as a grid of cards; a card opens its tracks.
Item {
    id: page

    /// `album`, `artist` or `genre`.
    required property string kind

    signal navigate(string page)

    readonly property var words: ({
            album: {
                title: Tr.t.albums,
                one: Tr.t.albums_one,
                many: Tr.t.albums_many,
                search: Tr.t.search_albums,
                unknown: Tr.t.unknown_album,
                icon: ""
            },
            artist: {
                title: Tr.t.artists,
                one: Tr.t.artists_one,
                many: Tr.t.artists_many,
                search: Tr.t.search_artists,
                unknown: Tr.t.unknown_artist,
                icon: "artist"
            },
            genre: {
                title: Tr.t.genres,
                one: Tr.t.genres_one,
                many: Tr.t.genres_many,
                search: Tr.t.search_genres,
                unknown: Tr.t.unknown_genre,
                icon: "tag"
            }
        })[kind]

    readonly property var orders: kind === "album" ? [
        {
            value: "artist",
            label: Tr.t.sort_album_artist
        },
        {
            value: "name",
            label: Tr.t.sort_title
        },
        {
            value: "-year",
            label: Tr.t.sort_newest
        },
        {
            value: "year",
            label: Tr.t.sort_oldest
        }
    ] : [
        {
            value: "name",
            label: Tr.t.sort_name
        },
        {
            value: "-tracks",
            label: Tr.t.sort_most_tracks
        }
    ]

    /// The first cards are coming: they fade in and rise 8 px, one after another.
    property bool entering: true

    /// The cover of the card opening `target`, a collection page, when it shows whole: where a
    /// cover flies from or back to. Null when it is out of sight, cut off or still coming in.
    function coverFor(target: string): Cover {
        const first = grid.indexAt(grid.contentX + 1, grid.contentY + 1);
        if (first < 0)
            return null;
        const end = grid.indexAt(grid.contentX + grid.width - 2, grid.contentY + grid.height - 2);
        const last = end < 0 ? grid.count - 1 : end;
        for (let index = first; index <= last; ++index) {
            const key = groups.key(index);
            if (Filters.collectionPage(page.kind, key.name, key.artist) !== target)
                continue;
            const cover = coverOf(grid.itemAtIndex(index));
            if (cover === null || cover.parent.opacity < 1)
                return null;
            const box = cover.mapToItem(grid, 0, 0, cover.width, cover.height);
            const whole = box.x >= -0.5 && box.y >= -0.5 && box.x + box.width <= grid.width + 0.5 && box.y + box.height <= grid.height + 0.5;
            return whole ? cover : null;
        }
        return null;
    }

    function coverOf(card: var): Cover {
        return card ? card.artwork : null;
    }

    GroupList {
        id: groups
        kind: page.kind
        search: search.text
        sort: page.kind === "album" ? "artist" : "name"
        // The cards laid out for the first list fade in; later ones show at once.
        onReadyChanged: entered.start()
    }

    Timer {
        id: entered
        interval: 300
        onTriggered: page.entering = false
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        NarrowBar {
            id: bar
            Layout.fillWidth: true
            visible: Shell.narrow
            title: page.words.title
            field: search

            MenuButton {
                iconOnly: true
                iconName: "sort"
                options: page.orders
                value: groups.sort
                Accessible.name: Tr.t.sort_by + ": " + text
                onActivated: value => groups.sort = value
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: !Shell.narrow
            Layout.leftMargin: 28
            Layout.rightMargin: 28
            Layout.topMargin: 22
            Layout.bottomMargin: 16
            spacing: 16

            ColumnLayout {
                Layout.alignment: Qt.AlignBottom
                spacing: 4

                Label {
                    text: page.words.title
                    color: Theme.text
                    font.pixelSize: 26
                    font.weight: Font.Bold
                    font.letterSpacing: -0.52
                }
                Label {
                    text: groups.ready ? Format.count(groups.count, page.words.one, page.words.many) : ""
                    color: Theme.text2
                    font.pixelSize: 13
                    font.features: {
                        "tnum": 1
                    }
                }
            }
            Item {
                Layout.fillWidth: true
            }
            RowLayout {
                id: searchHolder
                Layout.alignment: Qt.AlignBottom

                // In the bar while it searches, in narrow windows.
                SearchField {
                    id: search
                    parent: Shell.narrow && bar.fieldShown ? bar.slot : searchHolder
                    Layout.alignment: Qt.AlignVCenter
                    Layout.fillWidth: Shell.narrow
                    Layout.preferredWidth: Shell.narrow ? -1 : 260
                    Layout.minimumWidth: Shell.narrow ? 0 : 160
                    placeholder: page.words.search
                }
            }
            MenuButton {
                Layout.alignment: Qt.AlignBottom
                options: page.orders
                value: groups.sort
                Accessible.name: Tr.t.sort_by + ": " + text
                onActivated: value => groups.sort = value
            }
        }

        GridView {
            id: grid

            readonly property int columns: Math.max(1, Math.floor(width / 188))
            /// Room between the cards.
            readonly property real gap: Shell.narrow ? 12 : 20

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: Shell.narrow ? 12 : 28
            Layout.rightMargin: Shell.narrow ? 0 : 8
            Layout.topMargin: Shell.narrow ? 12 : 0
            clip: true
            visible: groups.count > 0
            cellWidth: width / columns
            cellHeight: cellWidth - gap + 10 + 40 + 24
            bottomMargin: 24
            model: groups
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            Accessible.name: page.words.title

            delegate: AbstractButton {
                id: card

                /// Its cover, which can fly to the page it opens.
                readonly property Cover artwork: art

                required property int index
                required property string name
                required property string artist
                required property int year
                required property int tracks
                required property string cover
                required property bool known

                readonly property string detail: {
                    if (page.kind === "album")
                        return [card.artist === "" ? Tr.t.unknown_artist : card.artist, card.year > 0 ? card.year : "—"].join(" · ");
                    return Format.count(card.tracks, Tr.t.track_one, Tr.t.tracks_many);
                }

                width: grid.cellWidth - grid.gap
                height: grid.cellHeight - 24
                hoverEnabled: true
                text: known ? name : page.words.unknown
                Accessible.name: text
                onClicked: {
                    const key = groups.key(index);
                    page.navigate(Filters.collectionPage(page.kind, key.name, key.artist));
                }

                background: null
                contentItem: Item {
                    implicitWidth: body.implicitWidth
                    implicitHeight: body.implicitHeight

                    // The first cards come in 16 ms apart, the last at most 80 ms after the
                    // first: a fade over 150 ms and an 8 px rise over 200 ms.
                    Component.onCompleted: {
                        if (!page.entering)
                            return;
                        body.opacity = 0;
                        cardIn.start();
                    }

                    SequentialAnimation {
                        id: cardIn

                        PauseAnimation {
                            duration: Math.min(card.index * 16, 80)
                        }
                        ParallelAnimation {
                            OpacityAnimator {
                                target: body
                                from: 0
                                to: 1
                                duration: Motion.fade
                                easing.bezierCurve: Motion.standard
                            }
                            YAnimator {
                                target: body
                                from: 8 * Motion.travel
                                to: 0
                                duration: Motion.move
                                easing.bezierCurve: Motion.standard
                            }
                        }
                    }

                    Column {
                        id: body
                        width: parent.width
                        spacing: 10

                        Cover {
                            id: art
                            size: card.width
                            radius: page.kind === "artist" ? card.width / 2 : 8
                            path: card.cover
                            iconName: page.words.icon
                            opacity: card.down ? 0.86 : 1
                            scale: card.down ? 0.97 : 1

                            PressScale on scale {}
                            Behavior on opacity {
                                id: dim

                                OpacityAnimator {
                                    duration: dim.targetValue < 1 ? Motion.exit : Motion.fade
                                }
                            }

                            Rectangle {
                                anchors.fill: parent
                                radius: parent.radius
                                color: "#FFFFFF"
                                opacity: card.hovered ? 0.06 : 0

                                HoverFade on opacity {}
                            }
                        }
                        Column {
                            width: card.width
                            spacing: 2

                            Label {
                                width: parent.width
                                text: card.text
                                elide: Text.ElideRight
                                color: card.known ? Theme.text : Theme.text2
                                font.pixelSize: 14
                                font.weight: Font.DemiBold
                            }
                            Label {
                                width: parent.width
                                text: card.detail
                                elide: Text.ElideRight
                                color: Theme.text2
                                font.pixelSize: 13
                            }
                        }
                    }
                }
            }

            ScrollBar.vertical: ThinScrollBar {}
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !grid.visible

            EmptyState {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -40
                visible: groups.ready
                readonly property bool empty: search.text.trim() === ""
                iconName: empty ? (Scan.running ? "refresh" : "folder") : "search"
                title: empty ? (Scan.running ? Tr.t.scanning_library : Tr.t.empty_library) : Tr.t.no_results
                message: empty && !Scan.running ? Tr.t.empty_library_hint : ""
                action: empty && !Scan.running ? Tr.t.open_sources : ""
                onTriggered: page.navigate("sources")
            }
        }
    }
}
