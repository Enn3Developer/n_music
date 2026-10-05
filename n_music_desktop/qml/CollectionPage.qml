pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// The tracks of an album, an artist or a genre.
Item {
    id: page

    /// `album`, `artist`, `genre` or `source`.
    required property string kind
    /// What follows the `:` of the page, see `Filters.collectionPage`.
    property string argument

    signal navigate(string page)

    readonly property var key: Filters.collectionKey(kind, argument)
    /// The page lists the tracks without the tag, like Unknown album.
    readonly property bool unknown: kind !== "source" && key.name === ""
    readonly property string name: !unknown ? key.name : ({
            album: Tr.t.unknown_album,
            artist: Tr.t.unknown_artist,
            genre: Tr.t.unknown_genre
        })[kind]

    readonly property string summary: {
        if (!tracks.ready)
            return "";
        const parts = [];
        // The tracks without an album are by any artist.
        if (kind === "album" && !unknown)
            parts.push(key.artist === "" ? Tr.t.unknown_artist : key.artist);
        if (kind === "source")
            parts.push(SourceKinds.name(SourceKinds.of(key.location)));
        parts.push(Format.count(tracks.count, Tr.t.track_one, Tr.t.tracks_many));
        if (tracks.count > 0)
            parts.push(Format.duration(tracks.duration));
        return parts.join(" · ");
    }

    TrackList {
        id: tracks
        filter: Filters.collectionFilter(page.kind, page.key)
        sort: page.kind === "genre" || page.kind === "source" ? "artist,album" : "album"
        label: page.name
        detail: page.kind === "album" ? page.key.artist : ""
        origin: page.kind + ":" + page.argument
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        NarrowBar {
            Layout.fillWidth: true
            visible: Shell.narrow
            title: ({
                    album: Tr.t.albums,
                    artist: Tr.t.artists,
                    genre: Tr.t.genres,
                    source: Tr.t.sources
                })[page.kind]
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Shell.narrow ? 16 : 28
            Layout.rightMargin: Shell.narrow ? 16 : 28
            Layout.topMargin: Shell.narrow ? 16 : 28
            Layout.bottomMargin: Shell.narrow ? 12 : 18
            spacing: Shell.narrow ? 16 : 24

            Cover {
                id: art
                Layout.alignment: Qt.AlignBottom
                size: Shell.narrow ? 96 : 168
                radius: page.kind === "artist" ? art.size / 2 : Shell.narrow ? 8 : 10
                iconName: ({
                        album: "",
                        artist: "artist",
                        genre: "tag",
                        source: SourceKinds.icon(SourceKinds.of(page.key.location ?? ""))
                    })[page.kind]
                paths: (page.kind === "genre" || page.kind === "source") && tracks.covers.length >= 4 ? tracks.covers : []
                path: tracks.covers.length > 0 ? tracks.covers[0] : ""
            }

            ColumnLayout {
                id: about
                Layout.alignment: Qt.AlignBottom
                Layout.fillWidth: true
                spacing: 8

                Label {
                    visible: !Shell.narrow
                    text: ({
                            album: Tr.t.field_album,
                            artist: Tr.t.field_artist,
                            genre: Tr.t.field_genre,
                            source: Tr.t.source
                        })[page.kind]
                    color: Theme.text2
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.96
                    font.capitalization: Font.AllUppercase
                }
                Label {
                    Layout.maximumWidth: about.width
                    text: page.name
                    elide: Text.ElideRight
                    color: Theme.text
                    font.pixelSize: Shell.narrow ? 22 : 34
                    font.weight: Font.Bold
                    font.letterSpacing: Shell.narrow ? -0.44 : -0.68
                }
                Label {
                    Layout.maximumWidth: about.width
                    text: page.summary
                    elide: Text.ElideRight
                    color: Theme.text2
                    font.pixelSize: 13
                    font.features: {
                        "tnum": 1
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 6
                    spacing: 8

                    PillButton {
                        primary: true
                        iconName: "play"
                        text: Tr.t.play
                        enabled: tracks.count > 0
                        onClicked: tracks.playAll(false)
                    }
                    PillButton {
                        iconName: "shuffle"
                        text: Shell.narrow ? "" : Tr.t.shuffle
                        enabled: tracks.count > 0
                        Accessible.name: Tr.t.shuffle
                        onClicked: tracks.playAll(true)
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                    SortButton {
                        iconOnly: Shell.narrow
                        list: tracks
                    }
                }
            }
        }

        TrackTable {
            id: table
            Layout.fillWidth: true
            Layout.fillHeight: true
            list: tracks
            visible: tracks.count > 0
            onNavigate: to => page.navigate(to)
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !table.visible

            EmptyState {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -20
                visible: tracks.ready
                iconName: "search"
                title: Tr.t.no_results
            }
        }
    }
}
