import QtQuick
import QtQuick.Shapes
import NMusic

// The play and pause icons in one, pause while `playing`: each bar of pause turns into one half
// of the play triangle over 150 ms. The triangle sits a little right of the box's middle, where
// it looks centred. Drawn in `color` at `size` pixels, like an `Icon`.
Item {
    id: icon

    property bool playing: false
    property color color: Theme.text
    property real size: 20

    /// 0 shows pause, 1 play.
    property real morph: playing ? 0 : 1

    /// A point of pause, at `pauseX`, `pauseY`, on its way to its place in play, at `playX`,
    /// `playY`; on the icon's grid of 24.
    function at(pauseX: real, pauseY: real, playX: real, playY: real): point {
        return Qt.point(pauseX + (playX - pauseX) * morph, pauseY + (playY - pauseY) * morph);
    }

    implicitWidth: size
    implicitHeight: size

    Behavior on morph {
        NumberAnimation {
            duration: Motion.fade
            easing.bezierCurve: Motion.standard
        }
    }

    Shape {
        width: 24
        height: 24
        scale: icon.size / 24
        transformOrigin: Item.TopLeft
        preferredRendererType: Shape.CurveRenderer

        // The left bar, or the triangle's wide half; the halves overlap so no seam shows.
        ShapePath {
            fillColor: icon.color
            strokeColor: "transparent"

            PathPolyline {
                path: [icon.at(6, 5, 8.2, 4.5), icon.at(10, 5, 14.6, 8.5), icon.at(10, 19, 14.6, 15.5), icon.at(6, 19, 8.2, 19.5), icon.at(6, 5, 8.2, 4.5)]
            }
        }
        // The right bar, or the triangle's tip.
        ShapePath {
            fillColor: icon.color
            strokeColor: "transparent"

            PathPolyline {
                path: [icon.at(14, 5, 14.2, 8.25), icon.at(18, 5, 20.2, 12), icon.at(18, 19, 20.2, 12), icon.at(14, 19, 14.2, 15.75), icon.at(14, 5, 14.2, 8.25)]
            }
        }
    }
}
