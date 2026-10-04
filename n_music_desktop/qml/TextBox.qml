pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A one-line input. Naming a library field in `suggest` offers the library's values containing
// what is typed; Up, Down and Enter pick one too.
TextField {
    id: box

    /// A field `Catalog.values` knows, like `artist`; empty offers nothing.
    property string suggest
    /// Takes only whole numbers.
    property bool numeric: false

    /// The library's values of `suggest`, read as the input takes focus.
    property var values: []
    /// The values containing the text, those starting with it first.
    readonly property var matches: {
        const typed = text.trim().toLowerCase();
        if (typed === "")
            return [];
        const starting = [];
        const containing = [];
        for (const value of values) {
            const lower = value.toLowerCase();
            if (lower === typed)
                continue;
            if (lower.startsWith(typed))
                starting.push(value);
            else if (lower.includes(typed))
                containing.push(value);
            if (starting.length >= 8)
                break;
        }
        return starting.concat(containing).slice(0, 8);
    }

    /// The user changed the text.
    signal edited

    function pick(value: string) {
        // Edits like typing does: assigning `text` would drop the binding showing the value.
        clear();
        insert(0, value);
        suggestions.close();
        edited();
    }

    function pickCurrent(event: KeyEvent) {
        event.accepted = suggestions.visible && choices.currentIndex >= 0;
        if (event.accepted)
            pick(matches[choices.currentIndex]);
    }

    implicitHeight: 36
    leftPadding: 10
    rightPadding: 10
    color: Theme.text
    placeholderTextColor: Theme.text3
    selectionColor: Theme.accent
    selectedTextColor: Theme.accentInk
    font.pixelSize: 14
    font.features: numeric ? {
        "tnum": 1
    } : {}
    validator: numeric ? digits : null
    inputMethodHints: numeric ? Qt.ImhDigitsOnly : Qt.ImhNone

    onActiveFocusChanged: {
        if (activeFocus && suggest !== "")
            values = Catalog.values(suggest);
        else if (!activeFocus)
            suggestions.close();
    }
    onTextEdited: {
        edited();
        if (matches.length > 0) {
            choices.currentIndex = -1;
            suggestions.open();
        } else {
            suggestions.close();
        }
    }
    Keys.onDownPressed: event => {
        event.accepted = suggestions.visible;
        if (suggestions.visible)
            choices.incrementCurrentIndex();
    }
    Keys.onUpPressed: event => {
        event.accepted = suggestions.visible;
        if (suggestions.visible)
            choices.decrementCurrentIndex();
    }
    Keys.onReturnPressed: event => pickCurrent(event)
    Keys.onEnterPressed: event => pickCurrent(event)
    Keys.onEscapePressed: event => {
        event.accepted = suggestions.visible;
        suggestions.close();
    }

    background: Rectangle {
        radius: 8
        color: Theme.input
        border.width: 1
        border.color: box.activeFocus ? Theme.accent : box.hovered ? Theme.text3 : Theme.border
    }

    RegularExpressionValidator {
        id: digits
        regularExpression: /[0-9]{0,9}/
    }

    Popup {
        id: suggestions
        y: box.height + 4
        width: Math.max(box.width, 220)
        padding: 6
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

        background: Rectangle {
            radius: 10
            color: Theme.menu
            border.width: 1
            border.color: Theme.line2
        }

        contentItem: ListView {
            id: choices
            implicitHeight: contentHeight
            interactive: false
            currentIndex: -1
            model: box.matches

            delegate: AbstractButton {
                id: choice

                required property string modelData
                required property int index

                width: ListView.view.width
                height: 30
                leftPadding: 10
                rightPadding: 10
                hoverEnabled: true
                focusPolicy: Qt.NoFocus
                text: modelData
                onClicked: box.pick(modelData)

                background: Rectangle {
                    radius: 6
                    color: choice.hovered || choice.ListView.isCurrentItem ? Theme.menuHover : "transparent"
                }
                contentItem: Label {
                    text: choice.text
                    elide: Text.ElideRight
                    verticalAlignment: Text.AlignVCenter
                    color: Theme.text
                    font.pixelSize: 13
                }
            }
        }
    }
}
