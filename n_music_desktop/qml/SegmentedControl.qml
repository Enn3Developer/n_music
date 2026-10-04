pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// Picks one of a few `options`, shown side by side.
Rectangle {
    id: control

    /// The labels of the options.
    property list<string> options
    property int current: 0

    signal activated(int index)

    implicitHeight: 42
    radius: 10
    color: Theme.field
    Accessible.role: Accessible.PageTabList

    Row {
        anchors.fill: parent
        anchors.margins: 4
        spacing: 4

        Repeater {
            model: control.options

            AbstractButton {
                id: option

                required property int index
                required property string modelData
                readonly property bool selected: index === control.current

                width: (parent.width - (control.options.length - 1) * 4) / control.options.length
                height: parent.height
                text: modelData
                hoverEnabled: true
                checkable: true
                checked: selected
                Accessible.role: Accessible.RadioButton
                onClicked: control.activated(index)

                background: Rectangle {
                    radius: 7
                    color: option.selected ? Theme.line2 : option.hovered ? Theme.hover : "transparent"
                    border.width: option.visualFocus ? 2 : 0
                    border.color: Theme.text
                }
                contentItem: Label {
                    text: option.text
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                    color: option.selected ? Theme.text : Theme.text2
                    font.pixelSize: 13
                    font.weight: option.selected ? Font.DemiBold : Font.Medium
                }
            }
        }
    }
}
