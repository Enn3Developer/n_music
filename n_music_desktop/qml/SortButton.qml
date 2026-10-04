pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// Shows the order of a track list, and picks another from a menu.
AbstractButton {
    id: button

    required property TrackList list

    readonly property var options: [
        {
            sort: "artist,album",
            label: Tr.t.sort_artist_album
        },
        {
            sort: "title",
            label: Tr.t.sort_title
        },
        {
            sort: "album",
            label: Tr.t.sort_album
        },
        {
            sort: "-year,album",
            label: Tr.t.sort_newest
        },
        {
            sort: "year,album",
            label: Tr.t.sort_oldest
        },
        {
            sort: "-plays,title",
            label: Tr.t.sort_most_played
        },
        {
            sort: "-lastPlayed",
            label: Tr.t.sort_recently_played
        },
        {
            sort: "length",
            label: Tr.t.sort_length
        },
        {
            sort: "location",
            label: Tr.t.sort_location
        }
    ]

    /// The order in words: an offered one's name, or the column a header click sorted by.
    readonly property string label: {
        const offered = options.find(option => option.sort === list.sort);
        if (offered)
            return offered.label;
        const first = list.sort.split(",")[0];
        const names = {
            title: Tr.t.column_title,
            album: Tr.t.column_album,
            year: Tr.t.column_year,
            plays: Tr.t.column_plays,
            length: Tr.t.column_time
        };
        const field = first.replace("-", "");
        return (names[field] || field) + (first.startsWith("-") ? " ↓" : " ↑");
    }

    implicitHeight: 32
    leftPadding: 12
    rightPadding: 12
    hoverEnabled: true
    font.pixelSize: 13
    font.weight: Font.Medium
    text: label
    Accessible.name: Tr.t.sort_by + ": " + label
    onClicked: menu.open()

    background: Rectangle {
        radius: 8
        color: button.down || menu.visible ? Theme.selected : button.hovered ? Theme.hover : "transparent"
        border.width: button.visualFocus ? 2 : 0
        border.color: Theme.text
    }

    contentItem: Row {
        spacing: 6

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            name: "sort"
            size: 15
            stroke: 1.9
            color: Theme.text2
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            text: button.text
            font: button.font
            color: Theme.text2
        }
        Icon {
            anchors.verticalCenter: parent.verticalCenter
            name: "chevron-down"
            size: 14
            stroke: 2
            color: Theme.text2
        }
    }

    PopupMenu {
        id: menu
        x: button.width - width
        y: button.height + 4

        Instantiator {
            model: button.options
            delegate: MenuEntry {
                required property var modelData
                text: modelData.label
                checked: button.list.sort === modelData.sort
                onTriggered: button.list.sort = modelData.sort
            }
            onObjectAdded: (index, entry) => menu.insertItem(index, entry)
            onObjectRemoved: (index, entry) => menu.removeItem(entry)
        }
    }
}
