import QtQuick
import QtQuick.Shapes
import NMusic

// A line icon from `Icons`, drawn in `color` at `size` pixels.
Item {
    id: icon

    property string name
    property color color: Theme.text
    property real size: 18
    property real stroke: 1.8

    readonly property var parts: Icons.paths[name] || []

    implicitWidth: size
    implicitHeight: size

    Shape {
        width: 24
        height: 24
        scale: icon.size / 24
        transformOrigin: Item.TopLeft
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            id: p0
            readonly property var part: icon.parts[0]
            strokeColor: part && !part.f ? icon.color : "transparent"
            fillColor: part && part.f ? icon.color : "transparent"
            strokeWidth: icon.stroke
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: p0.part ? p0.part.d : "" }
        }
        ShapePath {
            id: p1
            readonly property var part: icon.parts[1]
            strokeColor: part && !part.f ? icon.color : "transparent"
            fillColor: part && part.f ? icon.color : "transparent"
            strokeWidth: icon.stroke
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: p1.part ? p1.part.d : "" }
        }
        ShapePath {
            id: p2
            readonly property var part: icon.parts[2]
            strokeColor: part && !part.f ? icon.color : "transparent"
            fillColor: part && part.f ? icon.color : "transparent"
            strokeWidth: icon.stroke
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: p2.part ? p2.part.d : "" }
        }
        ShapePath {
            id: p3
            readonly property var part: icon.parts[3]
            strokeColor: part && !part.f ? icon.color : "transparent"
            fillColor: part && part.f ? icon.color : "transparent"
            strokeWidth: icon.stroke
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: p3.part ? p3.part.d : "" }
        }
        ShapePath {
            id: p4
            readonly property var part: icon.parts[4]
            strokeColor: part && !part.f ? icon.color : "transparent"
            fillColor: part && part.f ? icon.color : "transparent"
            strokeWidth: icon.stroke
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: p4.part ? p4.part.d : "" }
        }
    }
}
