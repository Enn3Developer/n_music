pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// A playlist: its tracks, or for a smart playlist those its rules match, with what changes it.
Item {
    id: page

    /// The playlist shown.
    property real playlistId

    signal navigate(string page)

    /// The order the playlist was left in, or its kind's usual one.
    readonly property string savedSort: playlist.sort !== "" ? playlist.sort : playlist.smart ? "artist,album" : "-added"
    readonly property var rules: Filters.parse(playlist.rule)

    /// How many tracks, how long, and how it changes.
    readonly property string summary: {
        if (!tracks.ready)
            return "";
        const parts = [Format.count(tracks.count, Tr.t.track_one, Tr.t.tracks_many)];
        if (tracks.count > 0)
            parts.push(Format.duration(tracks.duration));
        if (playlist.smart) {
            parts.push(Tr.t.updates_as_you_listen);
        } else if (playlist.modified > 0) {
            const ago = Format.ago(playlist.modified);
            parts.push(Tr.t.changed_ago.arg(Format.daysSince(playlist.modified) < 35 ? ago.toLocaleLowerCase() : ago));
        }
        return parts.join(" · ");
    }

    onSavedSortChanged: tracks.sort = savedSort

    Playlist {
        id: playlist
        playlistId: page.playlistId
    }

    TrackList {
        id: tracks
        playlist: page.playlistId
        label: playlist.name
        origin: "playlist:" + page.playlistId
        Component.onCompleted: sort = page.savedSort
        onSortChanged: {
            if (playlist.exists && sort !== page.savedSort)
                Playlists.setSort(page.playlistId, sort);
        }
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
                visible: !playlist.smart
                size: 168
                radius: 10
                iconName: "playlist"
                paths: tracks.covers.length >= 4 ? tracks.covers : []
                path: tracks.covers.length > 0 ? tracks.covers[0] : ""
            }
            Rectangle {
                Layout.alignment: Qt.AlignBottom
                visible: playlist.smart
                implicitWidth: 168
                implicitHeight: 168
                radius: 10
                color: Theme.field
                border.width: 1
                border.color: Theme.track

                Icon {
                    anchors.centerIn: parent
                    name: "filter"
                    size: 64
                    stroke: 1.4
                    color: Theme.accentText
                }
            }

            ColumnLayout {
                id: about
                Layout.alignment: Qt.AlignBottom
                Layout.fillWidth: true
                spacing: 8

                Label {
                    text: playlist.smart ? Tr.t.smart_playlist : Tr.t.playlist
                    color: Theme.text2
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.96
                    font.capitalization: Font.AllUppercase
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Label {
                        Layout.maximumWidth: about.width - (rename.visible ? rename.width + 8 : 0)
                        text: playlist.name
                        elide: Text.ElideRight
                        color: Theme.text
                        font.pixelSize: 34
                        font.weight: Font.Bold
                        font.letterSpacing: -0.68
                    }
                    IconButton {
                        id: rename
                        visible: !playlist.smart
                        size: 34
                        iconSize: 16
                        stroke: 1.9
                        iconName: "pencil"
                        color: Theme.text3
                        text: Tr.t.rename_playlist
                        onClicked: renaming.ask(playlist.name)
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                }

                Flow {
                    Layout.fillWidth: true
                    visible: playlist.smart
                    spacing: 6

                    Repeater {
                        model: page.rules.rules.length

                        delegate: Row {
                            id: entry

                            required property int index
                            readonly property var words: Filters.describe(page.rules.rules[index])

                            spacing: 6
                            visible: words !== null

                            FilterChip {
                                removable: false
                                name: entry.words ? entry.words.name : ""
                                detail: entry.words ? entry.words.text : ""
                                onClicked: drawer.open()
                            }
                            Label {
                                anchors.verticalCenter: parent.verticalCenter
                                visible: entry.index < page.rules.rules.length - 1
                                text: page.rules.match === "any" ? Tr.t.rules_or : Tr.t.rules_and
                                color: Theme.text3
                                font.pixelSize: 12
                            }
                        }
                    }
                    Label {
                        height: 28
                        visible: playlist.rule === "" || page.rules.rules.length === 0
                        verticalAlignment: Text.AlignVCenter
                        text: playlist.rule === "" ? Tr.t.rules_not_editable : Tr.t.rules_every_track
                        color: Theme.text3
                        font.pixelSize: 13
                    }
                    AbstractButton {
                        id: editRules
                        height: 28
                        visible: playlist.rule !== ""
                        leftPadding: 10
                        rightPadding: 10
                        hoverEnabled: true
                        text: Tr.t.edit_rules
                        onClicked: drawer.open()

                        background: Rectangle {
                            radius: 14
                            color: editRules.hovered ? Theme.hover : "transparent"
                        }
                        contentItem: Label {
                            verticalAlignment: Text.AlignVCenter
                            text: editRules.text
                            color: editRules.hovered ? Theme.text : Theme.text2
                            font.pixelSize: 13
                            font.underline: true
                        }
                    }
                }

                Label {
                    text: page.summary
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
                        text: Tr.t.shuffle
                        enabled: tracks.count > 0
                        onClicked: tracks.playAll(true)
                    }
                    IconButton {
                        id: more
                        size: 40
                        radius: 20
                        outlined: true
                        iconSize: 16
                        iconName: "more"
                        color: Theme.text
                        text: Tr.t.playlist_actions
                        onClicked: actions.open()

                        PopupMenu {
                            id: actions
                            y: more.height + 4

                            MenuEntry {
                                iconName: "pencil"
                                text: Tr.t.rename_playlist
                                onTriggered: renaming.ask(playlist.name)
                            }
                            MenuEntry {
                                iconName: "filter"
                                shown: playlist.smart
                                enabled: playlist.rule !== ""
                                text: Tr.t.edit_rules
                                onTriggered: drawer.open()
                            }
                            MenuEntry {
                                iconName: "trash"
                                danger: true
                                text: Tr.t.delete_playlist
                                onTriggered: deleting.open()
                            }
                        }
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                    SortButton {
                        list: tracks
                        extraOptions: playlist.smart ? [] : [
                            {
                                sort: "-added",
                                label: Tr.t.sort_added_newest
                            },
                            {
                                sort: "added",
                                label: Tr.t.sort_added_oldest
                            }
                        ]
                    }
                }
            }
        }

        TrackTable {
            id: table
            Layout.fillWidth: true
            Layout.fillHeight: true
            list: tracks
            onNavigate: to => page.navigate(to)
            layout: playlist.smart ? "smart" : "playlist"
            playlistName: playlist.smart ? "" : playlist.name
            visible: tracks.count > 0
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !table.visible

            EmptyState {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -20
                visible: tracks.ready
                iconName: playlist.smart ? "filter" : "playlist"
                title: playlist.smart ? Tr.t.smart_playlist_empty : Tr.t.playlist_empty
                message: playlist.smart ? Tr.t.smart_playlist_empty_hint : Tr.t.playlist_empty_hint
                action: playlist.smart ? (playlist.rule !== "" ? Tr.t.edit_rules : "") : Tr.t.open_tracks
                onTriggered: {
                    if (playlist.smart)
                        drawer.open();
                    else
                        page.navigate("tracks");
                }
            }
        }
    }

    // Dims the page under the rules drawer; a click on it closes the drawer.
    Rectangle {
        anchors.fill: parent
        color: Theme.bg
        opacity: drawer.shown ? 0.7 : 0
        visible: opacity > 0

        Behavior on opacity {
            NumberAnimation {
                duration: 180
            }
        }

        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            onClicked: drawer.close()
            onWheel: wheel => wheel.accepted = true
        }
    }

    FilterDrawer {
        id: drawer
        title: Tr.t.edit_rules
        filter: playlist.rule
        sort: tracks.sort
        onApplied: (filter, sort) => {
            Playlists.setRule(page.playlistId, filter);
            tracks.sort = sort;
        }
    }

    PromptDialog {
        id: renaming
        title: Tr.t.rename_playlist
        asksText: true
        placeholder: Tr.t.playlist_name
        confirmText: Tr.t.rename
        onConfirmed: name => Playlists.rename(page.playlistId, name)
    }

    PromptDialog {
        id: deleting
        title: Tr.t.delete_playlist_title.arg(playlist.name)
        message: Tr.t.delete_playlist_message
        danger: true
        confirmText: Tr.t.delete_playlist
        onConfirmed: {
            Playlists.remove(page.playlistId);
            page.navigate("tracks");
        }
    }
}
