pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// Every track of the library: search it, sort it, play it.
Item {
    id: page

    signal navigate(string page)

    /// How many tracks are listed, of how many, and how long they last.
    readonly property string summary: {
        if (!tracks.ready)
            return "";
        const count = tracks.count === tracks.total ? Format.count(tracks.total, Tr.t.track_one, Tr.t.tracks_many) : Tr.t.tracks_filtered.arg(Format.number(tracks.count)).arg(Format.number(tracks.total));
        return tracks.count > 0 ? count + " · " + Format.duration(tracks.duration) : count;
    }

    /// The applied rules, a chip each.
    readonly property var rules: Filters.parse(tracks.filter).rules

    function removeRule(index: int) {
        const spec = Filters.parse(tracks.filter);
        spec.rules.splice(index, 1);
        tracks.filter = Filters.json(spec);
    }

    TrackList {
        id: tracks
        search: search.text
        sort: "artist,album"
        label: Tr.t.tracks
        origin: "tracks"
        detail: [search.text.trim() === "" ? "" : Tr.t.search_detail.arg(search.text.trim()), Filters.summary(tracks.filter)].filter(part => part !== "").join(" · ")
    }

    Shortcut {
        sequences: [StandardKey.Find]
        enabled: !drawer.shown
        onActivated: {
            if (Shell.narrow)
                bar.openSearch();
            else
                search.focusInput();
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        NarrowBar {
            id: bar
            Layout.fillWidth: true
            visible: Shell.narrow
            title: Tr.t.tracks
            field: search

            IconButton {
                size: 44
                radius: 10
                iconSize: 20
                stroke: 2
                color: Theme.text
                iconName: "filter"
                text: page.rules.length > 0 ? Tr.t.filters_active.arg(page.rules.length) : Tr.t.filter
                onClicked: drawer.open()

                // Rules apply.
                Rectangle {
                    x: parent.width - 6 - width
                    y: 6
                    visible: page.rules.length > 0
                    width: 8
                    height: 8
                    radius: 4
                    color: Theme.accent
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: !Shell.narrow
            Layout.leftMargin: Shell.compact ? 20 : 28
            Layout.rightMargin: Shell.compact ? 20 : 28
            Layout.topMargin: Shell.compact ? 16 : 22
            Layout.bottomMargin: Shell.compact ? 10 : 14
            spacing: 14

            RowLayout {
                Layout.fillWidth: true
                spacing: Shell.compact ? 10 : 16

                // The title over the summary; beside the count alone in compact windows.
                GridLayout {
                    Layout.alignment: Shell.regular ? Qt.AlignBottom : Qt.AlignVCenter
                    Layout.rightMargin: Shell.wide ? 8 : 0
                    columns: Shell.compact ? 2 : 1
                    rowSpacing: 4
                    columnSpacing: 10

                    Label {
                        text: Tr.t.tracks
                        color: Theme.text
                        font.pixelSize: Shell.compact ? 22 : 26
                        font.weight: Font.Bold
                        font.letterSpacing: Shell.compact ? -0.44 : -0.52
                    }
                    Label {
                        Layout.alignment: Qt.AlignVCenter
                        text: !Shell.compact ? page.summary : tracks.ready ? Format.number(tracks.count) : ""
                        color: Theme.text2
                        font.pixelSize: 13
                        font.features: {
                            "tnum": 1
                        }
                    }
                }
                Item {
                    Layout.fillWidth: true
                    visible: !Shell.wide
                }
                // Holds the tools in wide and compact windows.
                RowLayout {
                    id: inlineSlot
                    Layout.fillWidth: Shell.wide
                    Layout.alignment: Qt.AlignVCenter
                    visible: !Shell.regular
                }
                RowLayout {
                    Layout.alignment: Shell.wide ? Qt.AlignVCenter : Qt.AlignBottom
                    visible: !Shell.compact
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
                }
                AbstractButton {
                    id: playAll
                    Layout.alignment: Qt.AlignVCenter
                    visible: Shell.compact
                    implicitWidth: 36
                    implicitHeight: 36
                    padding: 0
                    hoverEnabled: true
                    opacity: enabled ? 1 : 0.45
                    enabled: tracks.count > 0
                    text: Tr.t.play_all
                    Accessible.name: text
                    onClicked: tracks.playAll(false)

                    background: Rectangle {
                        radius: 18
                        color: playAll.down ? Qt.darker(Theme.accent, 1.08) : playAll.hovered ? Qt.lighter(Theme.accent, 1.06) : Theme.accent
                        border.width: playAll.visualFocus ? 2 : 0
                        border.color: Theme.text
                    }
                    contentItem: Item {
                        Icon {
                            anchors.centerIn: parent
                            // The triangle's weight sits left of its box.
                            anchors.horizontalCenterOffset: 1
                            name: "play"
                            size: 16
                            color: Theme.accentInk
                        }
                    }
                }
            }

            // Holds the tools below the title in regular windows.
            RowLayout {
                id: regularSlot
                Layout.fillWidth: true
                visible: Shell.regular
            }
        }

        // The rules and playing them all, below the bar of narrow windows.
        Flickable {
            Layout.fillWidth: true
            implicitHeight: chips.height
            visible: Shell.narrow
            contentWidth: chips.width
            clip: true
            flickableDirection: Flickable.HorizontalFlick
            boundsBehavior: Flickable.StopAtBounds

            Row {
                id: chips
                padding: 10
                leftPadding: 12
                rightPadding: 12
                spacing: 6

                Repeater {
                    model: page.rules.length

                    delegate: FilterChip {
                        required property int index
                        readonly property var words: Filters.describe(page.rules[index])

                        visible: words !== null
                        name: words ? words.name : ""
                        detail: words ? words.text : ""
                        onClicked: drawer.open()
                        onRemove: page.removeRule(index)
                    }
                }
                PillButton {
                    primary: true
                    small: true
                    implicitHeight: 32
                    iconName: "play"
                    text: Tr.t.play_count.arg(Format.number(tracks.count))
                    enabled: tracks.count > 0
                    onClicked: tracks.playAll(false)
                }
            }
        }

        TrackTable {
            id: table
            Layout.fillWidth: true
            Layout.fillHeight: true
            list: tracks
            onNavigate: to => page.navigate(to)
            visible: tracks.count > 0
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !table.visible

            EmptyState {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -40
                visible: tracks.ready
                readonly property bool empty: tracks.total === 0
                iconName: empty ? (Scan.running ? "refresh" : "folder") : "search"
                title: empty ? (Scan.running ? Tr.t.scanning_library : Tr.t.empty_library) : Tr.t.no_results
                message: empty && !Scan.running ? Tr.t.empty_library_hint : ""
                action: empty && !Scan.running ? Tr.t.open_sources : ""
                onTriggered: page.navigate("settings:sources")
            }
        }
    }

    // Searching, filtering and sorting: below the title in regular windows, beside it otherwise.
    // One of each, so the search keeps its text across the move.
    RowLayout {
        parent: Shell.regular ? regularSlot : inlineSlot
        Layout.fillWidth: !Shell.compact
        spacing: Shell.compact ? 10 : 8

        RowLayout {
            id: searchHolder
            Layout.alignment: Shell.compact ? Qt.AlignVCenter : Qt.AlignTop
            Layout.fillWidth: true

            // In the bar while it searches, in narrow windows.
            SearchField {
                id: search
                parent: Shell.narrow && bar.fieldShown ? bar.slot : searchHolder
                Layout.alignment: Qt.AlignVCenter
                Layout.fillWidth: true
                Layout.minimumWidth: Shell.narrow ? 0 : Shell.compact ? 160 : 200
                Layout.preferredWidth: Shell.narrow ? -1 : Shell.compact ? 260 : Shell.wide ? 380 : 360
                Layout.maximumWidth: Shell.narrow ? Number.POSITIVE_INFINITY : Layout.preferredWidth
                implicitHeight: Shell.compact ? 36 : 38
                radius: Shell.compact ? 9 : 10
                placeholder: Shell.compact ? Tr.t.search : Tr.t.search_tracks
            }
        }
        Flow {
            Layout.alignment: Qt.AlignTop
            Layout.fillWidth: true
            visible: !Shell.compact
            topPadding: 3
            spacing: 8

            Repeater {
                model: page.rules.length

                delegate: FilterChip {
                    required property int index
                    readonly property var words: Filters.describe(page.rules[index])

                    visible: words !== null
                    name: words ? words.name : ""
                    detail: words ? words.text : ""
                    onClicked: drawer.open()
                    onRemove: page.removeRule(index)
                }
            }
            DashedButton {
                implicitHeight: 32
                radius: 16
                iconName: "filter"
                color: Theme.text2
                font.weight: Font.Normal
                text: Tr.t.filter
                onClicked: drawer.open()
            }
        }
        // The chips folded into a count.
        IconButton {
            Layout.alignment: Qt.AlignVCenter
            visible: Shell.compact
            size: 36
            radius: 9
            iconSize: 16
            stroke: 2
            outlined: true
            iconName: "filter"
            color: Theme.text2
            text: page.rules.length > 0 ? Tr.t.filters_active.arg(page.rules.length) : Tr.t.filter
            onClicked: drawer.open()

            Rectangle {
                x: parent.width - width + 5
                y: -5
                visible: page.rules.length > 0
                implicitWidth: Math.max(16, count.implicitWidth + 8)
                implicitHeight: 16
                radius: 8
                color: Theme.accent

                Label {
                    id: count
                    anchors.centerIn: parent
                    text: page.rules.length
                    color: Theme.accentInk
                    font.pixelSize: 10
                    font.weight: Font.Bold
                }
            }
        }
        SortButton {
            Layout.alignment: Shell.compact ? Qt.AlignVCenter : Qt.AlignTop
            Layout.topMargin: Shell.compact ? 0 : 3
            iconOnly: Shell.compact
            list: tracks
        }
    }

    // Dims the page under the filter drawer; a click on it closes the drawer.
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
        filter: tracks.filter
        sort: tracks.sort
        search: tracks.search
        saveable: true
        onApplied: (filter, sort) => {
            tracks.filter = filter;
            tracks.sort = sort;
        }
        onSaveRequested: (filter, sort) => {
            naming.filter = filter;
            naming.sort = sort;
            naming.ask("");
        }
    }

    PromptDialog {
        id: naming

        property string filter
        property string sort

        title: Tr.t.new_smart_playlist
        asksText: true
        placeholder: Tr.t.playlist_name
        confirmText: Tr.t.create
        onConfirmed: name => Playlists.createSmart(name, filter, sort)
    }
}
