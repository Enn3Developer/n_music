import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An on/off switch, the accent when on.
AbstractButton {
    id: toggle

    implicitWidth: 40
    implicitHeight: 22
    checkable: true
    hoverEnabled: true
    Accessible.role: Accessible.CheckBox

    background: Rectangle {
        radius: 11
        color: toggle.checked ? Theme.accent : toggle.hovered ? Theme.border : Theme.track
        border.width: toggle.visualFocus ? 2 : 0
        border.color: Theme.text

        Rectangle {
            x: toggle.checked ? parent.width - width - 3 : 3
            y: 3
            width: 16
            height: 16
            radius: 8
            color: toggle.checked ? Theme.accentInk : Theme.text

            Behavior on x {
                NumberAnimation {
                    duration: 120
                    easing.type: Easing.OutCubic
                }
            }
        }
    }
    contentItem: null
}
