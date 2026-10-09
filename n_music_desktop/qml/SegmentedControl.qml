pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// Picks one of a few `options`, shown side by side. The picked one's ground slides to a new
// pick over 200 ms; under reduced motion it moves at once and fades in.
Rectangle {
    id: control

    /// The labels of the options.
    property list<string> options
    property int current: 0

    signal activated(int index)

    /// How far apart the options start.
    readonly property real step: (width - 8 + 4) / Math.max(1, options.length)
    /// The option the ground last went to.
    property int shown: -1

    implicitHeight: 42
    radius: 10
    color: Theme.field
    Accessible.role: Accessible.PageTabList
    Component.onCompleted: shown = current
    onCurrentChanged: {
        if (shown >= 0 && current >= 0) {
            if (Motion.reduced) {
                appear.restart();
            } else {
                // From wherever it shows now, a slide under way included.
                slide.from = shift.x + (shown - current) * step;
                slide.restart();
            }
        }
        shown = current;
    }

    Rectangle {
        id: ground
        x: 4 + control.current * control.step
        y: 4
        width: control.step - 4
        height: parent.height - 8
        visible: control.current >= 0 && control.current < control.options.length
        radius: 7
        color: Theme.line2
        transform: Translate {
            id: shift
        }

        NumberAnimation {
            id: slide
            target: shift
            property: "x"
            to: 0
            duration: Motion.move
            easing.bezierCurve: Motion.standard
        }
        OpacityAnimator {
            id: appear
            target: ground
            from: 0
            to: 1
            duration: Motion.fade
        }
    }

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

                width: control.step - 4
                height: parent.height
                text: modelData
                hoverEnabled: true
                checkable: true
                checked: selected
                Accessible.role: Accessible.RadioButton
                onClicked: control.activated(index)

                background: Rectangle {
                    radius: 7
                    color: !option.selected && option.hovered ? Theme.hover : Qt.alpha(Theme.hover, 0)
                    border.width: option.visualFocus ? 2 : 0
                    border.color: Theme.text

                    TintFade on color {}
                }
                contentItem: Label {
                    text: option.text
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                    color: option.selected ? Theme.text : Theme.text2
                    font.pixelSize: 13
                    font.weight: option.selected ? Font.DemiBold : Font.Medium

                    ColorFade on color {}
                }
            }
        }
    }
}
