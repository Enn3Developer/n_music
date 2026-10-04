import QtQuick
import NMusic

// A cover thumbnail with rounded corners, or an icon on a tile while there is none. Four
// `paths` make a mosaic instead.
Rectangle {
    id: cover

    /// The thumbnail's file; empty without a cover.
    property string path
    property list<string> paths
    property real size: 36
    property string iconName: "note"

    implicitWidth: size
    implicitHeight: size
    radius: 4
    color: image.status === Image.Ready ? "transparent" : Theme.raised

    Icon {
        anchors.centerIn: parent
        visible: image.status !== Image.Ready
        name: cover.iconName
        size: Math.round(Math.min(cover.size * 0.45, 72))
        color: Theme.text3
    }

    Image {
        id: image
        anchors.fill: parent
        asynchronous: true
        sourceSize: Qt.size(cover.width, cover.height)
        source: {
            const paths = cover.paths.length === 4 ? cover.paths : cover.path === "" ? [] : [cover.path];
            if (paths.length === 0)
                return "";
            return "image://cover/" + (cover.radius / cover.width).toFixed(4) + "/" + paths.map(path => encodeURIComponent(path)).join(",");
        }
    }
}
