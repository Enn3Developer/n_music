import QtQuick
import NMusic

// A cover thumbnail with rounded corners, or a note on a tile while there is none.
Rectangle {
    id: cover

    /// The thumbnail's file; empty without a cover.
    property string path
    property real size: 36

    implicitWidth: size
    implicitHeight: size
    radius: 4
    color: image.status === Image.Ready ? "transparent" : Theme.raised

    Icon {
        anchors.centerIn: parent
        visible: image.status !== Image.Ready
        name: "note"
        size: Math.round(cover.size * 0.45)
        color: Theme.text3
    }

    Image {
        id: image
        anchors.fill: parent
        asynchronous: true
        sourceSize: Qt.size(cover.width, cover.height)
        source: cover.path === "" ? "" : "image://cover/" + (cover.radius / cover.width).toFixed(4) + "/" + encodeURIComponent(cover.path)
    }
}
