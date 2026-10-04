pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// Rules that count as one: all, any or none of them must match.
Rectangle {
    id: box

    /// The group; edits change it in place.
    required property var group

    /// A value was typed.
    signal edited
    /// A choice, a rule added or a rule removed changed what the group shows.
    signal reshaped
    signal remove

    implicitHeight: content.implicitHeight + 20
    radius: 10
    color: Theme.field
    border.width: 1
    border.color: Theme.line2

    Column {
        id: content
        x: 10
        y: 10
        width: box.width - 20
        spacing: 8

        Item {
            width: content.width
            height: 32

            Row {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 8

                SelectBox {
                    anchors.verticalCenter: parent.verticalCenter
                    implicitHeight: 28
                    leftPadding: 6
                    rightPadding: 6
                    radius: 6
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                    options: Filters.groupOptions
                    value: box.group.group
                    Accessible.name: Tr.t.group_combines
                    onActivated: value => {
                        box.group.group = value;
                        box.reshaped();
                    }
                }
                Label {
                    anchors.verticalCenter: parent.verticalCenter
                    text: Tr.t.group_of
                    color: Theme.text2
                    font.pixelSize: 13
                }
            }
            IconButton {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                size: 32
                iconSize: 14
                stroke: 2.2
                iconName: "close"
                color: Theme.text3
                text: Tr.t.remove_group
                onClicked: box.remove()
            }
        }

        Repeater {
            model: box.group.rules.length

            delegate: RuleRow {
                required property int index

                width: content.width
                rule: box.group.rules[index]
                onEdited: box.edited()
                onReshaped: box.reshaped()
                onRemove: {
                    box.group.rules.splice(index, 1);
                    box.reshaped();
                }
            }
        }

        FlatButton {
            implicitHeight: 30
            leftPadding: 10
            rightPadding: 10
            iconName: "plus"
            text: Tr.t.add_rule_to_group
            onClicked: {
                box.group.rules.push(Filters.newRule("artist"));
                box.reshaped();
            }
        }
    }
}
