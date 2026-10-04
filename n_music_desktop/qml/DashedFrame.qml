import QtQuick
import QtQuick.Shapes
import NMusic

// A dashed one pixel outline with rounded corners, for things to add.
Shape {
    id: frame

    property real radius: 8
    property color color: Theme.border

    preferredRendererType: Shape.CurveRenderer

    ShapePath {
        strokeColor: frame.color
        strokeWidth: 1
        strokeStyle: ShapePath.DashLine
        dashPattern: [3, 3]
        fillColor: "transparent"

        PathRectangle {
            x: 0.5
            y: 0.5
            width: frame.width - 1
            height: frame.height - 1
            radius: frame.radius - 0.5
        }
    }
}
