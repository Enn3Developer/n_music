import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// The top of a page in narrow windows: a button opening the navigation, the page's title and
// its actions. With a `field`, a search button unfolds it in place of the title.
Rectangle {
    id: bar

    property string title
    /// The page's search field, which the page moves into `slot` while `fieldShown`; null for
    /// none.
    property SearchField field: null
    /// The search button was pressed: the field shows until closed, as it does while it holds
    /// text.
    property bool searching: false
    readonly property bool fieldShown: field !== null && (searching || field.text !== "")
    readonly property alias slot: slot
    /// The page's actions, after the title.
    default property alias actions: trailing.data

    function openSearch() {
        searching = true;
        Qt.callLater(field.focusInput);
    }

    function closeSearch() {
        field.text = "";
        searching = false;
    }

    implicitHeight: 56
    color: Theme.bg
    // Wider windows show the field where it was.
    onVisibleChanged: {
        if (!visible)
            searching = false;
    }

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Theme.line
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 8
        anchors.rightMargin: 8
        anchors.bottomMargin: 1
        spacing: 4
        // Escape in an empty field leaves the search.
        Keys.onEscapePressed: event => {
            event.accepted = bar.fieldShown;
            if (bar.fieldShown)
                bar.closeSearch();
        }

        IconButton {
            size: 44
            radius: 10
            iconSize: 20
            stroke: 2
            color: Theme.text
            iconName: bar.fieldShown ? "arrow-left" : "menu"
            text: bar.fieldShown ? Tr.t.close_search : Tr.t.open_navigation
            onClicked: {
                if (bar.fieldShown)
                    bar.closeSearch();
                else
                    Shell.navigationRequested();
            }
        }
        Label {
            Layout.fillWidth: true
            Layout.leftMargin: 4
            visible: !bar.fieldShown
            text: bar.title
            elide: Text.ElideRight
            color: Theme.text
            font.pixelSize: 18
            font.weight: Font.Bold
        }
        RowLayout {
            id: slot
            Layout.fillWidth: true
            visible: bar.fieldShown
        }
        IconButton {
            visible: bar.field !== null && !bar.fieldShown
            size: 44
            radius: 10
            iconSize: 20
            stroke: 2
            color: Theme.text
            iconName: "search"
            text: Tr.t.search
            onClicked: bar.openSearch()
        }
        RowLayout {
            id: trailing
            spacing: 4
        }
    }
}
