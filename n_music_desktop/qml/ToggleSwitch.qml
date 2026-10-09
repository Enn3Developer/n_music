import QtQuick
import QtQuick.Controls.Basic
import NMusic

// An on/off switch, the accent when on. The knob slides across over 150 ms and stretches while
// pressed; under reduced motion it changes sides at once.
AbstractButton {
    id: toggle

    implicitWidth: 40
    implicitHeight: 22
    checkable: true
    hoverEnabled: true
    Accessible.role: Accessible.CheckBox

    background: Rectangle {
        radius: 11
        color: toggle.hovered ? Theme.border : Theme.track

        ColorFade on color {}

        // The accent fades in over the track.
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: Theme.accent
            opacity: toggle.checked ? 1 : 0

            Behavior on opacity {
                OpacityAnimator {
                    duration: Motion.fade
                }
            }
        }
        // Above the accent, which would hide it.
        Rectangle {
            anchors.fill: parent
            visible: toggle.visualFocus
            radius: parent.radius
            color: "transparent"
            border.width: 2
            border.color: Theme.text
        }

        Rectangle {
            x: toggle.checked ? parent.width - width - 3 : 3
            y: 3
            width: 16
            height: 16
            radius: 8
            color: toggle.checked ? Theme.accentInk : Theme.text
            transform: Scale {
                origin.x: 8
                origin.y: 8
                xScale: toggle.down ? 1 + 0.12 * Motion.travel : 1
                yScale: toggle.down ? 1 - 0.05 * Motion.travel : 1

                // Pressing stretches it in 80 ms; letting go eases it back over 150 ms.
                Behavior on xScale {
                    id: stretch

                    NumberAnimation {
                        duration: stretch.targetValue > 1 ? Motion.exit : Motion.fade
                        easing.bezierCurve: Motion.standard
                    }
                }
                Behavior on yScale {
                    id: squeeze

                    NumberAnimation {
                        duration: squeeze.targetValue < 1 ? Motion.exit : Motion.fade
                        easing.bezierCurve: Motion.standard
                    }
                }
            }

            ColorFade on color {}
            Behavior on x {
                enabled: !Motion.reduced

                XAnimator {
                    duration: Motion.fade
                    easing.bezierCurve: Motion.standard
                }
            }
        }
    }
    contentItem: null
}
