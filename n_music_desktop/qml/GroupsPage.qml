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
                icon: "disc"
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

    GroupList {
        id: groups
        kind: page.kind
        search: search.text
        sort: page.kind === "album" ? "artist" : "name"
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
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
            SearchField {
                id: search
                Layout.alignment: Qt.AlignBottom
                Layout.preferredWidth: 260
                Layout.minimumWidth: 160
                placeholder: page.words.search
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

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 28
            Layout.rightMargin: 8
            clip: true
            visible: groups.count > 0
            cellWidth: width / columns
            cellHeight: cellWidth - 20 + 10 + 40 + 24
            bottomMargin: 24
            model: groups
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            Accessible.name: page.words.title

            delegate: AbstractButton {
                id: card

                required property int index
                required property string name
                required property string artist
                required property int year
                required property int tracks
                required property string cover
                required property bool known

                width: grid.cellWidth - 20
                height: grid.cellHeight - 24
                hoverEnabled: true
                text: known ? name : page.words.unknown
                Accessible.name: text
                onClicked: {
                    const key = groups.key(index);
                    page.navigate(Filters.collectionPage(page.kind, key.name, key.artist));
                }

                background: null
                contentItem: Column {
                    spacing: 10

                    Cover {
                        size: card.width
                        radius: page.kind === "artist" ? card.width / 2 : 8
                        path: card.cover
                        iconName: page.words.icon
                        opacity: card.down ? 0.8 : 1

                        Rectangle {
                            anchors.fill: parent
                            radius: parent.radius
                            color: "#FFFFFF"
                            opacity: card.hovered ? 0.06 : 0
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
                            text: page.kind === "album" ? [card.artist === "" ? Tr.t.unknown_artist : card.artist, card.year > 0 ? card.year : "—"].join(" · ") : Format.count(card.tracks, Tr.t.track_one, Tr.t.tracks_many)
                            elide: Text.ElideRight
                            color: Theme.text2
                            font.pixelSize: 13
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
