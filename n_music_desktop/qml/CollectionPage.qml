pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// The tracks of an album, an artist or a genre.
Item {
    id: page

    /// `album`, `artist` or `genre`.
    required property string kind
    /// What follows the `:` of the page, see `Filters.collectionPage`.
    property string argument

    signal navigate(string page)

    readonly property var key: Filters.collectionKey(kind, argument)

    readonly property string summary: {
        if (!tracks.ready)
            return "";
        const parts = [];
        if (kind === "album")
            parts.push(key.artist === "" ? Tr.t.unknown_artist : key.artist);
        parts.push(Format.count(tracks.count, Tr.t.track_one, Tr.t.tracks_many));
        if (tracks.count > 0)
            parts.push(Format.duration(tracks.duration));
        return parts.join(" · ");
    }

    TrackList {
        id: tracks
        filter: Filters.collectionFilter(page.kind, page.key)
        sort: page.kind === "genre" ? "artist,album" : "album"
        label: page.key.name
        detail: page.kind === "album" ? page.key.artist : ""
        origin: page.kind + ":" + page.argument
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 28
            Layout.rightMargin: 28
            Layout.topMargin: 28
            Layout.bottomMargin: 18
            spacing: 24

            Cover {
                Layout.alignment: Qt.AlignBottom
                size: 168
                radius: page.kind === "artist" ? 84 : 10
                iconName: ({
                        album: "disc",
                        artist: "artist",
                        genre: "tag"
                    })[page.kind]
                paths: page.kind === "genre" && tracks.covers.length >= 4 ? tracks.covers : []
                path: tracks.covers.length > 0 ? tracks.covers[0] : ""
            }

            ColumnLayout {
                id: about
                Layout.alignment: Qt.AlignBottom
                Layout.fillWidth: true
                spacing: 8

                Label {
                    text: ({
                            album: Tr.t.field_album,
                            artist: Tr.t.field_artist,
                            genre: Tr.t.field_genre
                        })[page.kind]
                    color: Theme.text2
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.96
                    font.capitalization: Font.AllUppercase
                }
                Label {
                    Layout.maximumWidth: about.width
                    text: page.key.name
                    elide: Text.ElideRight
                    color: Theme.text
                    font.pixelSize: 34
                    font.weight: Font.Bold
                    font.letterSpacing: -0.68
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
                        text: Tr.t.shuffle
                        enabled: tracks.count > 0
                        onClicked: tracks.playAll(true)
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                    SortButton {
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
