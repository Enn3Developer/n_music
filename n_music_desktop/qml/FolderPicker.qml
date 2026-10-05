pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// Picks a folder of this computer: the places to start from on the side, and the folders of
// the one shown under the way to it, which can be typed instead.
Popup {
    id: picker

    property string title: Tr.t.choose_folder
    property string confirmText: Tr.t.add_this_folder
    /// The paths of the folders picked before: they show as added, and cannot be picked again.
    property var taken: []

    /// The folder at `path` was picked.
    signal chosen(string path)

    readonly property bool narrow: width < 600
    /// The folder shown was picked before.
    readonly property bool takenHere: taken.includes(browser.path)
    readonly property bool choosable: browser.ready && browser.problem === "" && !takenHere
    /// The way to the folder shows as a field to type a path in.
    property bool typing: false
    /// Where it opens next: the folder holding the one picked last.
    property string from
    /// The folder left for the one holding it, which the list then marks.
    property string left

    function choose() {
        if (!choosable)
            return;
        from = browser.parentPath !== "" ? browser.parentPath : browser.path;
        close();
        chosen(browser.path);
    }

    function go(path: string) {
        left = "";
        browser.open(path);
        list.forceActiveFocus();
    }

    /// Goes to the folder `steps` up the way, marking the one it comes from.
    function up(steps: int) {
        const crumbs = browser.crumbs;
        const index = crumbs.length - 1 - steps;
        if (steps < 1 || (index < 0 && browser.parentPath === ""))
            return;
        left = index >= 0 ? crumbs[index + 1].path : browser.path;
        browser.open(index >= 0 ? crumbs[index].path : browser.parentPath);
        list.forceActiveFocus();
    }

    function startTyping() {
        typing = true;
        pathField.text = browser.path;
        pathField.forceActiveFocus();
        pathField.selectAll();
    }

    function stopTyping() {
        typing = false;
        list.forceActiveFocus();
    }

    function goTyped() {
        left = "";
        if (browser.open(pathField.text) || pathField.text.trim() === "")
            stopTyping();
    }

    /// What a place, or a step of the way, is called.
    function placeName(place: var): string {
        return place.kind === "" || place.kind === "drive" ? place.name : Tr.t["place_" + place.kind];
    }

    function placeIcon(kind: string): string {
        return ({
                home: "home",
                music: "note",
                root: "drive",
                drive: "drive"
            })[kind] ?? "folder";
    }

    /// The room a step of the way takes, the folder shown being in bold.
    function stepWidth(crumb: var, shown: bool): real {
        return 16 + (crumb.kind !== "" ? 22 : 0) + Math.ceil((shown ? boldMetrics : metrics).advanceWidth(placeName(crumb)));
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(760, parent.width - 32)
    height: Math.min(560, parent.height - 32)
    modal: true
    focus: true
    padding: 0
    closePolicy: Popup.CloseOnEscape
    onAboutToShow: {
        typing = false;
        left = "";
        browser.findPlaces();
        browser.open(from !== "" ? from : browser.start());
    }
    onOpened: list.forceActiveFocus()

    Overlay.modal: Rectangle {
        color: Theme.dark ? "#99000000" : "#55000000"
    }

    background: Rectangle {
        radius: 12
        color: Theme.surface
        border.width: 1
        border.color: Theme.line2
    }

    FolderBrowser {
        id: browser
    }

    Shortcut {
        sequence: "Ctrl+L"
        enabled: picker.opened
        onActivated: picker.startTyping()
    }

    FontMetrics {
        id: metrics
        font.family: Theme.font
        font.pixelSize: 14
    }
    FontMetrics {
        id: boldMetrics
        font.family: Theme.font
        font.pixelSize: 14
        font.weight: Font.DemiBold
    }

    // Shows that a folder is being listed only when that takes a while.
    Timer {
        id: listing
        interval: 300
        running: !browser.ready
    }

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 20
            Layout.rightMargin: 12
            Layout.topMargin: 12
            Layout.bottomMargin: picker.narrow ? 4 : 10
            spacing: 8

            Label {
                Layout.fillWidth: true
                text: picker.title
                elide: Text.ElideRight
                color: Theme.text
                font.pixelSize: 17
                font.weight: Font.Bold
            }
            IconButton {
                size: 32
                iconSize: 16
                iconName: "close"
                text: Tr.t.close
                onClicked: picker.close()
            }
        }

        // Narrow, the places line up above the folders.
        ListView {
            Layout.fillWidth: true
            Layout.bottomMargin: 12
            implicitHeight: 32
            visible: picker.narrow
            orientation: ListView.Horizontal
            spacing: 6
            leftMargin: 16
            rightMargin: 16
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: browser.places
            Accessible.name: Tr.t.places

            delegate: AbstractButton {
                id: chip

                required property var modelData
                readonly property bool active: modelData.path === browser.place

                height: 32
                leftPadding: 12
                rightPadding: 12
                hoverEnabled: true
                text: picker.placeName(modelData)
                onClicked: picker.go(modelData.path)

                background: Rectangle {
                    radius: height / 2
                    color: chip.active ? Theme.raised : chip.down ? Theme.selected : chip.hovered ? Theme.hover : "transparent"
                    border.width: chip.visualFocus ? 2 : 1
                    border.color: chip.visualFocus ? Theme.text : chip.active ? Theme.border : Theme.line2
                }
                contentItem: Row {
                    spacing: 6

                    Icon {
                        anchors.verticalCenter: parent.verticalCenter
                        name: picker.placeIcon(chip.modelData.kind)
                        size: 14
                        color: chip.active ? Theme.accentText : Theme.text2
                    }
                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: chip.text
                        color: chip.active ? Theme.text : Theme.text2
                        font.pixelSize: 13
                        font.weight: Font.Medium
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 1
            color: Theme.line
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            ListView {
                Layout.preferredWidth: 200
                Layout.fillHeight: true
                visible: !picker.narrow
                topMargin: 10
                bottomMargin: 10
                spacing: 2
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                model: browser.places
                Accessible.name: Tr.t.places

                delegate: NavItem {
                    required property var modelData

                    x: 10
                    width: ListView.view.width - 20
                    text: picker.placeName(modelData)
                    iconName: picker.placeIcon(modelData.kind)
                    active: modelData.path === browser.place
                    onClicked: picker.go(modelData.path)
                }
            }
            Rectangle {
                Layout.fillHeight: true
                implicitWidth: 1
                visible: !picker.narrow
                color: Theme.line
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 10
                    spacing: 6

                    IconButton {
                        size: 34
                        iconSize: 16
                        iconName: "arrow-up"
                        enabled: browser.parentPath !== ""
                        text: Tr.t.parent_folder
                        onClicked: picker.up(1)
                    }
                    // The way to the folder shown; the steps that do not fit fold into a menu
                    // before the others.
                    Item {
                        id: way

                        readonly property var crumbs: browser.crumbs
                        /// The first step shown.
                        readonly property int first: {
                            const last = crumbs.length - 1;
                            for (let first = 0; first < last; first++) {
                                let room = first > 0 ? more.implicitWidth + 2 : 0;
                                for (let index = first; index <= last; index++)
                                    room += (index > 0 ? 16 : 0) + picker.stepWidth(crumbs[index], index === last) + (index > first ? 2 : 0);
                                if (room <= width)
                                    return first;
                            }
                            return Math.max(0, last);
                        }

                        Layout.fillWidth: true
                        implicitHeight: 34
                        visible: !picker.typing

                        Row {
                            height: parent.height
                            spacing: 2

                            IconButton {
                                id: more
                                anchors.verticalCenter: parent.verticalCenter
                                visible: way.first > 0
                                size: 30
                                iconSize: 16
                                iconName: "more"
                                text: Tr.t.folders_above
                                onClicked: folded.open()

                                PopupMenu {
                                    id: folded
                                    y: more.height + 4

                                    Instantiator {
                                        model: way.crumbs.slice(0, way.first)
                                        delegate: MenuEntry {
                                            required property var modelData
                                            required property int index

                                            text: picker.placeName(modelData)
                                            iconName: picker.placeIcon(modelData.kind)
                                            // Once the menu closed: going there empties it.
                                            onTriggered: Qt.callLater(picker.up, way.crumbs.length - 1 - index)
                                        }
                                        onObjectAdded: (index, entry) => folded.insertItem(index, entry)
                                        onObjectRemoved: (index, entry) => folded.removeItem(entry)
                                    }
                                }
                            }
                            Repeater {
                                model: way.crumbs.slice(way.first)

                                Row {
                                    id: crumb

                                    required property var modelData
                                    required property int index
                                    /// Its place on the whole way.
                                    readonly property int depth: way.first + index
                                    readonly property bool last: depth === way.crumbs.length - 1

                                    height: way.height
                                    spacing: 2

                                    Icon {
                                        anchors.verticalCenter: parent.verticalCenter
                                        visible: crumb.depth > 0
                                        name: "chevron-right"
                                        size: 14
                                        stroke: 2
                                        color: Theme.text3
                                    }
                                    AbstractButton {
                                        id: step
                                        anchors.verticalCenter: parent.verticalCenter
                                        // The folder shown may not fit even alone.
                                        width: crumb.last ? Math.min(implicitWidth, way.width - crumb.x - x) : implicitWidth
                                        height: 30
                                        leftPadding: 8
                                        rightPadding: 8
                                        hoverEnabled: true
                                        focusPolicy: Qt.NoFocus
                                        text: picker.placeName(crumb.modelData)
                                        onClicked: picker.up(way.crumbs.length - 1 - crumb.depth)

                                        background: Rectangle {
                                            radius: 6
                                            color: step.down ? Theme.selected : step.hovered && !crumb.last ? Theme.hover : "transparent"
                                        }
                                        contentItem: RowLayout {
                                            spacing: 6

                                            Icon {
                                                visible: crumb.modelData.kind !== ""
                                                name: picker.placeIcon(crumb.modelData.kind)
                                                size: 16
                                                color: Theme.text2
                                            }
                                            Label {
                                                Layout.fillWidth: true
                                                text: step.text
                                                elide: Text.ElideMiddle
                                                color: crumb.last ? Theme.text : Theme.text2
                                                font.pixelSize: 14
                                                font.weight: crumb.last ? Font.DemiBold : Font.Normal
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    TextField {
                        id: pathField
                        Layout.fillWidth: true
                        implicitHeight: 34
                        visible: picker.typing
                        leftPadding: 10
                        rightPadding: 10
                        color: Theme.text
                        placeholderText: Tr.t.type_path
                        placeholderTextColor: Theme.text3
                        selectionColor: Theme.accent
                        selectedTextColor: Theme.accentInk
                        font.pixelSize: 14
                        Accessible.name: Tr.t.type_path
                        Keys.onReturnPressed: picker.goTyped()
                        Keys.onEnterPressed: picker.goTyped()
                        Keys.onEscapePressed: picker.stopTyping()
                        onActiveFocusChanged: {
                            if (!activeFocus)
                                picker.typing = false;
                        }

                        background: Rectangle {
                            radius: 8
                            color: Theme.input
                            border.width: 1
                            border.color: pathField.activeFocus ? Theme.accent : Theme.border
                        }
                    }
                    IconButton {
                        size: 34
                        iconSize: 16
                        iconName: "pencil"
                        visible: !picker.typing
                        text: Tr.t.type_path
                        onClicked: picker.startTyping()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 1
                    color: Theme.line
                }

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ListView {
                        id: list
                        anchors.fill: parent
                        topMargin: 6
                        bottomMargin: 6
                        clip: true
                        focus: true
                        currentIndex: -1
                        boundsBehavior: Flickable.StopAtBounds
                        model: browser.folders
                        Accessible.name: Tr.t.folders
                        // Back in the folder holding the one left, that one is marked.
                        onModelChanged: {
                            currentIndex = browser.folders.findIndex(folder => folder.path === picker.left);
                            if (currentIndex >= 0)
                                positionViewAtIndex(currentIndex, ListView.Contain);
                        }
                        // Return opens the folder marked, or else picks the one shown.
                        Keys.onPressed: event => {
                            const marked = currentIndex >= 0 ? browser.folders[currentIndex] : null;
                            const control = event.modifiers & Qt.ControlModifier;
                            switch (event.key) {
                            case Qt.Key_Return:
                            case Qt.Key_Enter:
                                if (marked !== null && !control)
                                    picker.go(marked.path);
                                else
                                    picker.choose();
                                break;
                            case Qt.Key_Right:
                                if (marked !== null)
                                    picker.go(marked.path);
                                break;
                            case Qt.Key_Left:
                            case Qt.Key_Backspace:
                                picker.up(1);
                                break;
                            default:
                                return;
                            }
                            event.accepted = true;
                        }

                        delegate: AbstractButton {
                            id: folder

                            required property var modelData
                            required property int index
                            readonly property bool marked: ListView.isCurrentItem && list.activeFocus

                            x: 8
                            width: ListView.view.width - 16
                            height: 40
                            leftPadding: 10
                            rightPadding: 10
                            hoverEnabled: true
                            focusPolicy: Qt.NoFocus
                            text: modelData.name
                            onClicked: picker.go(modelData.path)

                            background: Rectangle {
                                radius: 8
                                color: folder.down || folder.marked ? Theme.selected : folder.hovered ? Theme.hover : "transparent"
                            }
                            contentItem: RowLayout {
                                spacing: 12

                                Icon {
                                    name: "folder"
                                    size: 18
                                    color: folder.hovered || folder.marked ? Theme.accentText : Theme.text2
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: folder.text
                                    elide: Text.ElideRight
                                    color: Theme.text
                                    font.pixelSize: 14
                                }
                                Badge {
                                    visible: picker.taken.includes(folder.modelData.path)
                                    text: Tr.t.added
                                }
                                Icon {
                                    name: "chevron-right"
                                    size: 14
                                    stroke: 2
                                    color: Theme.text3
                                    opacity: folder.hovered || folder.marked ? 1 : 0
                                }
                            }
                        }

                        ScrollBar.vertical: ThinScrollBar {}
                    }

                    Label {
                        anchors.centerIn: parent
                        width: parent.width - 48
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.Wrap
                        color: Theme.text3
                        font.pixelSize: 14
                        text: {
                            if (!browser.ready)
                                return listing.running ? "" : Tr.t.folder_loading;
                            if (browser.problem !== "")
                                return Tr.t["folder_" + browser.problem];
                            return list.count === 0 ? Tr.t.no_folders : "";
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 1
            color: Theme.line
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 20
            Layout.rightMargin: 14
            Layout.topMargin: 12
            Layout.bottomMargin: 12
            spacing: 10

            Label {
                id: where
                Layout.fillWidth: true
                // Beside the buttons where they leave room.
                visible: picker.narrow
                text: picker.takenHere ? Tr.t.folder_taken : browser.path
                elide: Text.ElideMiddle
                color: Theme.text3
                font.pixelSize: 13
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Label {
                    Layout.fillWidth: true
                    visible: !picker.narrow
                    text: where.text
                    elide: Text.ElideMiddle
                    color: Theme.text3
                    font.pixelSize: 13
                }
                Item {
                    Layout.fillWidth: true
                    visible: picker.narrow
                }
                PillButton {
                    implicitHeight: 38
                    text: Tr.t.cancel
                    onClicked: picker.close()
                }
                PillButton {
                    implicitHeight: 38
                    primary: true
                    enabled: picker.choosable
                    text: picker.confirmText
                    onClicked: picker.choose()
                }
            }
        }
    }
}
