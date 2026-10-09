import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A square button showing only an icon; `text` names it for assistive technology. The pointer
// tints it, and pressing shrinks the icon.
AbstractButton {
    id: button

    property string iconName
    property real size: 40
    property real iconSize: 20
    property real stroke: 1.8
    property color color: Theme.text2
    property real radius: 8
    /// Draws a border around it.
    property bool outlined: false

    implicitWidth: size
    implicitHeight: size
    padding: 0
    hoverEnabled: true
    opacity: enabled ? 1 : 0.45
    Accessible.name: text

    background: Rectangle {
        radius: button.radius
        color: button.down ? Theme.selected : button.hovered ? Theme.raised : Qt.alpha(Theme.raised, 0)
        border.width: button.visualFocus ? 2 : button.outlined ? 1 : 0
        border.color: button.visualFocus ? Theme.text : Theme.border

        TintFade on color {}
    }

    contentItem: Item {
        Icon {
            anchors.centerIn: parent
            name: button.iconName
            size: button.iconSize
            stroke: button.stroke
            color: button.color
            scale: button.down ? 0.86 : 1

            ColorFade on color {}
            PressScale on scale {}
        }
    }
}
