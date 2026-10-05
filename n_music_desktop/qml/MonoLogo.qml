import QtQuick
import QtQuick.Shapes
import NMusic

// The app's monochrome icon, the Android launcher's themed layer: equaliser bars around an N on
// a 50 unit grid. The bars stay quieter than the N, which one colour would swallow.
Item {
    id: logo

    property real size: 36
    /// The N.
    property color color: Theme.text3
    /// The bars around it.
    property color barColor: Theme.border

    implicitWidth: size
    implicitHeight: size

    Shape {
        width: 50
        height: 50
        scale: logo.size / 50
        transformOrigin: Item.TopLeft
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "transparent"
            fillColor: logo.barColor
            PathSvg {
                path: "M21 8h7v1h-7z"
                    + "M17 9h4v1h-4zM28 9h4v1h-4z"
                    + "M16 10h1v1h-1zM32 10h1v1h-1z"
                    + "M14 11h2v1h-2zM33 11h2v1h-2z"
                    + "M13 12h1v1h-1zM35 12h1v1h-1z"
                    + "M18 13h6v2h-6zM25 13h6v2h-6z"
                    + "M11 14h2v1h-2zM36 14h2v1h-2z"
                    + "M10 16h1v1h-1zM13 16h4v2h-4zM18 16h6v2h-6zM25 16h6v2h-6zM32 16h4v2h-4zM38 16h1v1h-1z"
                    + "M9 17h1v2h-1zM39 17h1v2h-1z"
                    + "M13 19h4v2h-4zM18 19h6v2h-6zM25 19h6v2h-6zM32 19h4v2h-4z"
                    + "M8 20h1v1h-1zM40 20h1v1h-1z"
                    + "M7 22h5v2h-5zM13 22h4v2h-4zM18 22h6v2h-6zM25 22h6v2h-6zM32 22h4v2h-4zM37 22h5v2h-5z"
                    + "M7 25h5v2h-5zM13 25h4v2h-4zM18 25h6v2h-6zM25 25h6v2h-6zM32 25h4v2h-4zM37 25h5v2h-5z"
                    + "M7 28h5v2h-5zM13 28h4v2h-4zM18 28h6v2h-6zM25 28h6v2h-6zM32 28h4v2h-4zM37 28h5v2h-5z"
                    + "M8 31h4v1h-4zM13 31h4v1h-4zM18 31h6v1h-6zM25 31h6v1h-6zM32 31h4v1h-4zM37 31h4v1h-4z"
                    + "M9 33h3v1h-3zM13 33h4v1h-4zM18 33h6v1h-6zM25 33h6v1h-6zM32 33h4v1h-4zM37 33h3v1h-3z"
                    + "M10 35h2v1h-2zM13 35h4v1h-4zM18 35h6v1h-6zM25 35h6v1h-6zM32 35h4v1h-4zM37 35h2v1h-2z"
                    + "M13 37h4v1h-4zM18 37h6v1h-6zM25 37h6v1h-6zM32 37h4v1h-4z"
                    + "M14 39h3v1h-3zM18 39h6v1h-6zM25 39h6v1h-6zM32 39h3v1h-3z"
                    + "M18 41h6v1h-6zM25 41h6v1h-6z"
            }
        }
        ShapePath {
            strokeColor: "transparent"
            fillColor: logo.color
            PathSvg {
                path: "M21 20h2v8h-2zM23 21h1v3.6h-1zM24 22.2h1v3.6h-1zM25 23.4h1v3.6h-1zM26 20h2v8h-2z"
            }
        }
    }
}
