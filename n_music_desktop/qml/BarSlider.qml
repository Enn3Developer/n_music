import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A slim slider: a 4 px track filled up to the value, with a round knob.
Slider {
    id: slider

    property color fill: Theme.text
    /// Shows the knob only under the pointer or while dragging.
    property bool knobOnHover: false

    implicitHeight: 16
    padding: 0
    hoverEnabled: true

    background: Rectangle {
        x: slider.leftPadding
        y: slider.topPadding + (slider.availableHeight - height) / 2
        width: slider.availableWidth
        height: 4
        radius: 2
        color: Theme.track

        Rectangle {
            width: slider.visualPosition * parent.width
            height: parent.height
            radius: 2
            color: slider.fill
        }
    }

    handle: Rectangle {
        x: slider.leftPadding + slider.visualPosition * slider.availableWidth - width / 2
        y: slider.topPadding + (slider.availableHeight - height) / 2
        width: 12
        height: 12
        radius: 6
        color: slider.fill
        visible: slider.enabled && (!slider.knobOnHover || slider.hovered || slider.pressed || slider.visualFocus)
        border.width: slider.visualFocus ? 2 : 0
        border.color: Theme.accent
    }
}
