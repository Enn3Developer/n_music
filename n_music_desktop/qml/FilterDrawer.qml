pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A panel sliding in from the right to write the rules and the order of a track list. It works
// on a copy, counting the matches as it goes; Apply hands it to the list.
FocusScope {
    id: drawer

    required property TrackList list
    /// Slid in.
    property bool shown: false
    /// The filter being written, see `Filters`; edits change it in place.
    property var spec: Filters.empty()
    /// The sort being written, as keys `{ field, descending }`.
    property var keys: []

    readonly property TrackList preview: counter.object as TrackList

    function open() {
        const applied = Filters.parse(list.filter);
        if (applied.rules.length === 0)
            applied.rules.push(Filters.newRule("artist"));
        spec = applied;
        keys = Filters.parseSort(list.sort);
        shown = true;
        update();
        forceActiveFocus();
    }

    function close() {
        shown = false;
    }

    function apply() {
        list.filter = Filters.json(Filters.clean(spec));
        if (keys.length > 0)
            list.sort = Filters.sortString(keys);
        close();
    }

    /// Counts the matches of the rules as they are.
    function update() {
        if (preview)
            preview.filter = Filters.json(Filters.clean(spec));
    }

    /// Shows the rules again after a change of their shape.
    function reshape() {
        spec = JSON.parse(JSON.stringify(spec));
        keys = JSON.parse(JSON.stringify(keys));
        update();
    }

    function addRule() {
        spec.rules.push(Filters.newRule("artist"));
        reshape();
    }

    function addGroup() {
        spec.rules.push(Filters.newGroup());
        reshape();
    }

    function removeRule(index: int) {
        spec.rules.splice(index, 1);
        reshape();
    }

    function addKey() {
        const field = Object.keys(Filters.sortFields).find(name => !keys.some(key => key.field === name));
        if (field === undefined)
            return;
        keys.push({
            field: field,
            descending: false
        });
        reshape();
    }

    x: shown ? parent.width - width : parent.width
    width: Math.min(520, parent.width)
    height: parent.height
    visible: x < parent.width
    Keys.onEscapePressed: close()

    Behavior on x {
        NumberAnimation {
            duration: 180
            easing.type: Easing.OutCubic
        }
    }

    Instantiator {
        id: counter
        active: drawer.shown
        onObjectChanged: drawer.update()

        delegate: TrackList {
            search: drawer.list.search
        }
    }

    // The shadow falling on the page.
    Rectangle {
        x: -48
        width: 48
        height: parent.height
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop {
                position: 0
                color: "transparent"
            }
            GradientStop {
                position: 1
                color: Qt.rgba(0, 0, 0, Theme.dark ? 0.35 : 0.12)
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.surface

        Rectangle {
            width: 1
            height: parent.height
            color: Theme.line2
        }
    }

    // Takes the clicks missing the controls, so they don't reach the page below.
    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        onWheel: wheel => wheel.accepted = true
    }

    Label {
        x: 20
        anchors.verticalCenter: closeButton.verticalCenter
        text: Tr.t.filter_tracks
        color: Theme.text
        font.pixelSize: 19
        font.weight: Font.Bold
    }
    IconButton {
        id: closeButton
        x: parent.width - 20 - width
        y: 18
        size: 36
        iconSize: 16
        stroke: 2.2
        iconName: "close"
        text: Tr.t.close
        onClicked: drawer.close()
    }

    Flickable {
        id: body
        y: 66
        width: parent.width
        height: footer.y - y
        contentHeight: content.implicitHeight + 4 + 20
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ThinScrollBar {}

        Column {
            id: content
            x: 20
            y: 4
            width: body.width - 40
            spacing: 10

            Row {
                height: 32
                spacing: 8

                Label {
                    anchors.verticalCenter: parent.verticalCenter
                    text: Tr.t.matching_before
                    color: Theme.text2
                    font.pixelSize: 14
                }
                SelectBox {
                    anchors.verticalCenter: parent.verticalCenter
                    implicitHeight: 32
                    leftPadding: 8
                    rightPadding: 8
                    font.weight: Font.DemiBold
                    options: Filters.matchOptions
                    value: drawer.spec.match
                    Accessible.name: Tr.t.combine_rules
                    onActivated: value => {
                        drawer.spec.match = value;
                        drawer.reshape();
                    }
                }
                Label {
                    anchors.verticalCenter: parent.verticalCenter
                    text: Tr.t.matching_after
                    color: Theme.text2
                    font.pixelSize: 14
                }
            }

            Repeater {
                model: drawer.spec.rules.length

                delegate: Loader {
                    id: entry

                    required property int index
                    readonly property var rule: drawer.spec.rules[index]

                    width: content.width
                    sourceComponent: rule.group === undefined ? ruleRow : ruleGroup

                    Component {
                        id: ruleRow

                        RuleRow {
                            rule: entry.rule
                            onEdited: drawer.update()
                            onReshaped: drawer.reshape()
                            onRemove: drawer.removeRule(entry.index)
                        }
                    }
                    Component {
                        id: ruleGroup

                        RuleGroup {
                            group: entry.rule
                            onEdited: drawer.update()
                            onReshaped: drawer.reshape()
                            onRemove: drawer.removeRule(entry.index)
                        }
                    }
                }
            }

            Row {
                topPadding: 2
                spacing: 6

                DashedButton {
                    iconName: "plus"
                    text: Tr.t.add_rule
                    onClicked: drawer.addRule()
                }
                DashedButton {
                    iconName: "plus"
                    text: Tr.t.add_group
                    onClicked: drawer.addGroup()
                }
            }

            Column {
                width: content.width
                spacing: 8

                // A line 10 pixels down, and 16 more to the heading.
                Item {
                    width: content.width
                    height: 19

                    Rectangle {
                        y: 10
                        width: parent.width
                        height: 1
                        color: Theme.line2
                    }
                }

                Label {
                    text: Tr.t.sort_by
                    color: Theme.text
                    font.pixelSize: 14
                    font.weight: Font.Bold
                }

                Repeater {
                    model: drawer.keys.length

                    delegate: Item {
                        id: sortKey

                        required property int index
                        readonly property var key: drawer.keys[index]

                        width: content.width
                        height: 36

                        SelectBox {
                            width: parent.width - 132 - 32 - 16
                            options: Filters.sortOptions
                            value: sortKey.key.field
                            Accessible.name: sortKey.index === 0 ? Tr.t.sort_field : Tr.t.then_by
                            onActivated: value => {
                                sortKey.key.field = value;
                                drawer.reshape();
                            }
                        }
                        SelectBox {
                            x: parent.width - 132 - 32 - 8
                            width: 132
                            muted: true
                            options: Filters.directionOptions(sortKey.key.field)
                            value: sortKey.key.descending ? "down" : "up"
                            Accessible.name: Tr.t.sort_direction
                            onActivated: value => {
                                sortKey.key.descending = value === "down";
                                drawer.reshape();
                            }
                        }
                        IconButton {
                            x: parent.width - 32
                            anchors.verticalCenter: parent.verticalCenter
                            size: 32
                            iconSize: 14
                            stroke: 2.2
                            iconName: "close"
                            color: Theme.text3
                            text: Tr.t.remove_sort_key
                            // The list keeps an order; the last key stays.
                            enabled: drawer.keys.length > 1
                            onClicked: {
                                drawer.keys.splice(sortKey.index, 1);
                                drawer.reshape();
                            }
                        }
                    }
                }

                FlatButton {
                    implicitHeight: 30
                    leftPadding: 10
                    rightPadding: 10
                    iconName: "plus"
                    text: Tr.t.then_by
                    enabled: drawer.keys.length < Object.keys(Filters.sortFields).length
                    onClicked: drawer.addKey()
                }
            }
        }
    }

    Item {
        id: footer
        y: parent.height - height
        width: parent.width
        height: 67

        Rectangle {
            width: parent.width
            height: 1
            color: Theme.line2
        }

        Row {
            x: 20
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: 0.5
            spacing: 4
            visible: drawer.preview !== null && drawer.preview.ready

            Label {
                text: drawer.preview ? Format.number(drawer.preview.count) : ""
                color: Theme.text
                font.pixelSize: 14
                font.weight: Font.Bold
                font.features: {
                    "tnum": 1
                }
            }
            Label {
                text: drawer.preview && drawer.preview.count === 1 ? Tr.t.match_one : Tr.t.match_many
                color: Theme.text2
                font.pixelSize: 14
            }
        }

        PillButton {
            x: parent.width - 20 - width
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: 0.5
            implicitHeight: 38
            primary: true
            text: Tr.t.apply
            onClicked: drawer.apply()
        }
    }
}
