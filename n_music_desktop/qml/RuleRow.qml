pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Dialogs
import QtQuick.Layouts
import NMusic

// A filter rule being written: the field it tests, the condition, and the value.
Item {
    id: row

    /// The rule; edits change it in place.
    required property var rule

    /// A value was typed.
    signal edited
    /// A choice changed, so the editors show a different rule.
    signal reshaped
    signal remove

    readonly property var field: Filters.fields[rule.field]
    readonly property string kind: field.kind
    readonly property bool low: kind === "range" && rule.op !== "until" && rule.op !== "at_most"
    readonly property bool high: kind === "range" && rule.op !== "from" && rule.op !== "at_least"

    implicitHeight: 36

    SelectBox {
        id: fieldBox
        width: 150
        options: Filters.fieldOptions
        value: row.rule.field
        popupWidth: 230
        Accessible.name: Tr.t.filter_field
        onActivated: value => {
            Filters.retarget(row.rule, value);
            row.reshaped();
        }
    }

    SelectBox {
        id: opBox
        x: fieldBox.width + 8
        width: 110
        visible: row.kind !== "period"
        muted: true
        options: row.field.ops.map(op => ({
                    value: op,
                    label: Tr.t["op_" + op]
                }))
        value: row.rule.op
        Accessible.name: Tr.t.filter_condition
        onActivated: value => {
            row.rule.op = value;
            row.reshaped();
        }
    }
    Label {
        x: opBox.x + 4
        height: 36
        visible: row.kind === "period"
        verticalAlignment: Text.AlignVCenter
        text: Tr.t.op_the_last
        color: Theme.text3
        font.pixelSize: 13
    }

    RowLayout {
        x: opBox.x + opBox.width + 8
        width: remove.x - 8 - x
        height: 36
        spacing: 6

        TextBox {
            Layout.fillWidth: true
            visible: row.kind === "text" || row.kind === "folder"
            text: row.rule.value
            suggest: row.field.values ?? ""
            Accessible.name: row.field.label
            onEdited: {
                row.rule.value = text;
                row.edited();
            }
        }
        IconButton {
            visible: row.kind === "folder"
            size: 36
            iconSize: 16
            iconName: "folder"
            text: Tr.t.choose_folder
            onClicked: picker.open()
        }

        TextBox {
            Layout.fillWidth: true
            Layout.maximumWidth: 72
            visible: row.low
            numeric: true
            text: row.rule.from
            Accessible.name: Tr.t.range_from
            onEdited: {
                row.rule.from = text;
                row.edited();
            }
        }
        Label {
            visible: row.low && row.high
            text: Tr.t.range_to
            color: Theme.text3
            font.pixelSize: 13
        }
        TextBox {
            Layout.fillWidth: true
            Layout.maximumWidth: 72
            visible: row.high
            numeric: true
            text: row.rule.to
            Accessible.name: Tr.t.range_until
            onEdited: {
                row.rule.to = text;
                row.edited();
            }
        }

        TextBox {
            Layout.preferredWidth: 56
            visible: row.kind === "period"
            numeric: true
            text: row.rule.amount
            Accessible.name: Tr.t.period_amount
            onEdited: {
                row.rule.amount = text;
                row.edited();
            }
        }
        SelectBox {
            Layout.fillWidth: true
            visible: row.kind === "period"
            muted: true
            options: Filters.unitOptions
            value: row.rule.unit
            Accessible.name: Tr.t.period_unit
            onActivated: value => {
                row.rule.unit = value;
                row.reshaped();
            }
        }

        Item {
            Layout.fillWidth: true
            visible: row.kind === "range"
        }
    }

    IconButton {
        id: remove
        x: row.width - width
        anchors.verticalCenter: parent.verticalCenter
        size: 32
        iconSize: 14
        stroke: 2.2
        iconName: "close"
        color: Theme.text3
        text: Tr.t.remove_rule
        onClicked: row.remove()
    }

    FolderDialog {
        id: picker
        title: Tr.t.choose_folder
        onAccepted: {
            row.rule.value = Catalog.folder(selectedFolder);
            row.reshaped();
        }
    }
}
