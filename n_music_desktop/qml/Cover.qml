import QtQuick
import NMusic

// A cover thumbnail with rounded corners, or a placeholder on a tile when there is none: the app's
// monochrome icon in place of the cover art, or `iconName`. A bare tile while it loads. Four
// `paths` make a mosaic instead.
Rectangle {
    id: cover

    /// The thumbnail's file; empty without a cover.
    property string path
    property list<string> paths
    property real size: 36
    /// The placeholder for a tile standing for something other than cover art, like an artist;
    /// empty for the app's icon.
    property string iconName

    /// There is no cover to show, rather than one still loading.
    readonly property bool missing: image.status === Image.Null || image.status === Image.Error

    implicitWidth: size
    implicitHeight: size
    radius: 4
    color: image.status === Image.Ready ? "transparent" : Theme.raised

    MonoLogo {
        anchors.centerIn: parent
        visible: cover.missing && cover.iconName === ""
        size: cover.size
    }
    Icon {
        anchors.centerIn: parent
        visible: cover.missing && cover.iconName !== ""
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
