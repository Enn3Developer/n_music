pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// The library's sources as cards, by kind: a card opens its tracks or a menu updating it or
// taking it out, and the top adds sources, updates them all and shows how far a scan got.
Item {
    id: page

    signal navigate(string page)

    /// `name` or `-tracks`.
    property string sort: "name"

    readonly property var orders: [
        {
            value: "name",
            label: Tr.t.sort_name
        },
        {
            value: "-tracks",
            label: Tr.t.sort_most_tracks
        }
    ]

    /// The sources whose name or location contains the search, in the order of `sort`, by
    /// kind: `{ title, sources }` for each kind with some, in the order of `SourceKinds.all`.
    readonly property var sections: {
        const search = searchField.text.trim().toLowerCase();
        const found = Sources.items.filter(source => source.name.toLowerCase().includes(search) || source.location.toLowerCase().includes(search));
        const byName = (a, b) => a.name.localeCompare(b.name);
        found.sort(sort === "-tracks" ? (a, b) => b.tracks - a.tracks || byName(a, b) : byName);
        return SourceKinds.all.map(kind => ({
                    title: kind.group,
                    sources: found.filter(source => source.kind === kind.value)
                })).filter(section => section.sources.length > 0);
    }

    /// When the last complete update finished, like `today, 17:38`; empty before one.
    readonly property string updated: {
        if (Scan.updated <= 0)
            return "";
        const date = new Date(Scan.updated * 1000);
        const time = date.toLocaleTimeString(Qt.locale(), Locale.ShortFormat);
        const days = Format.daysSince(Scan.updated);
        return days === 0 ? Tr.t.today_at.arg(time) : days === 1 ? Tr.t.yesterday_at.arg(time) : date.toLocaleString(Qt.locale(), Locale.ShortFormat);
    }

    /// Room between the cards.
    readonly property real gap: Shell.narrow ? 12 : 20
    readonly property int columns: Math.max(1, Math.floor((cards.width + gap) / 188))
    readonly property real cardWidth: (cards.width - (columns - 1) * gap) / columns

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        NarrowBar {
            id: bar
            Layout.fillWidth: true
            visible: Shell.narrow
            title: Tr.t.sources
            field: searchField

            MenuButton {
                iconOnly: true
                iconName: "sort"
                options: page.orders
                value: page.sort
                Accessible.name: Tr.t.sort_by + ": " + text
                onActivated: value => page.sort = value
            }
            IconButton {
                id: narrowActions
                size: 44
                radius: 10
                iconSize: 20
                stroke: 2
                color: Theme.text
                iconName: "more"
                text: Tr.t.library_actions
                enabled: Sources.items.length > 0
                onClicked: actions.popup(narrowActions, narrowActions.width - actions.width, narrowActions.height + 4)
            }
            AddSourceButton {
                id: narrowAdder
                iconOnly: true
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
                    text: Tr.t.sources
                    color: Theme.text
                    font.pixelSize: 26
                    font.weight: Font.Bold
                    font.letterSpacing: -0.52
                }
                Label {
                    text: Sources.loaded ? Format.count(Sources.items.length, Tr.t.sources_one, Tr.t.sources_many) : ""
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
                    id: searchField
                    parent: Shell.narrow && bar.fieldShown ? bar.slot : searchHolder
                    Layout.alignment: Qt.AlignVCenter
                    Layout.fillWidth: Shell.narrow
                    Layout.preferredWidth: Shell.narrow ? -1 : 260
                    Layout.minimumWidth: Shell.narrow ? 0 : 160
                    placeholder: Tr.t.search_sources
                }
            }
            MenuButton {
                Layout.alignment: Qt.AlignBottom
                options: page.orders
                value: page.sort
                Accessible.name: Tr.t.sort_by + ": " + text
                onActivated: value => page.sort = value
            }
            IconButton {
                id: wideActions
                Layout.alignment: Qt.AlignBottom
                size: 38
                radius: 19
                iconSize: 17
                outlined: true
                iconName: "more"
                text: Tr.t.library_actions
                enabled: Sources.items.length > 0
                onClicked: actions.popup(wideActions, wideActions.width - actions.width, wideActions.height + 4)

                Tip {
                    visible: wideActions.hovered && !actions.visible
                    text: wideActions.text
                }
            }
            AddSourceButton {
                id: adder
                Layout.alignment: Qt.AlignBottom
                Layout.preferredHeight: 38
            }
        }

        ScanCard {
            Layout.fillWidth: true
            Layout.leftMargin: Shell.narrow ? 12 : 28
            Layout.rightMargin: Shell.narrow ? 12 : 28
            Layout.topMargin: Shell.narrow ? 12 : 0
            Layout.bottomMargin: Shell.narrow ? 0 : 20
            visible: Scan.running
        }

        Flickable {
            id: scroller
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: page.sections.length > 0
            clip: true
            contentWidth: width
            contentHeight: cards.implicitHeight + cards.y + 24
            boundsBehavior: Flickable.StopAtBounds
            Accessible.name: Tr.t.sources

            Column {
                id: cards
                x: Shell.narrow ? 12 : 28
                y: Shell.narrow ? 12 : 0
                width: scroller.width - 2 * x
                spacing: 28

                Repeater {
                    model: page.sections

                    Column {
                        id: section

                        required property var modelData

                        width: cards.width
                        spacing: 14

                        Row {
                            spacing: 8

                            Label {
                                text: section.modelData.title
                                color: Theme.text2
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                                font.letterSpacing: 0.96
                                font.capitalization: Font.AllUppercase
                            }
                            Label {
                                text: Format.number(section.modelData.sources.length)
                                color: Theme.text3
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                                font.features: {
                                    "tnum": 1
                                }
                            }
                        }
                        Grid {
                            columns: page.columns
                            columnSpacing: page.gap
                            rowSpacing: 24

                            Repeater {
                                model: section.modelData.sources

                                SourceCard {
                                    required property var modelData

                                    width: page.cardWidth
                                    source: modelData
                                    onClicked: page.navigate(Filters.collectionPage("source", modelData.prefix, ""))
                                    onRemoveRequested: removal.askFor(modelData.location, modelData.name)
                                }
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
            visible: !scroller.visible

            EmptyState {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -40
                visible: Sources.loaded
                readonly property bool empty: Sources.items.length === 0
                iconName: empty ? "folder" : "search"
                title: empty ? Tr.t.no_sources_title : Tr.t.no_results
                message: empty ? Tr.t.no_sources_hint : ""
                action: empty ? Tr.t.add_source : ""
                onTriggered: (Shell.narrow ? narrowAdder : adder).open()
            }
        }
    }

    // What updates every source.
    PopupMenu {
        id: actions

        MenuEntry {
            iconName: "refresh"
            enabled: !Scan.running
            text: Tr.t.update_library
            onTriggered: Scan.refresh()
        }
        MenuEntry {
            iconName: "tag"
            enabled: !Scan.running
            text: Tr.t.reload_metadata_title
            onTriggered: Scan.reload()
        }
        MenuLine {
            shown: page.updated !== ""
        }
        MenuEntry {
            note: true
            shown: page.updated !== ""
            text: Tr.t.last_update.arg(page.updated)
        }
    }

    RemoveSourceDialog {
        id: removal
    }
}
