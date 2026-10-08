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

    /// The files drawn, percent-encoded and joined by commas: four for a mosaic, one, or none.
    /// The encoding keeps a comma in a path from splitting it.
    readonly property string files: {
        const paths = cover.paths.length === 4 ? cover.paths : cover.path === "" ? [] : [cover.path];
        return paths.map(path => encodeURIComponent(path)).join(",");
    }
    /// There is no cover to show, rather than one still loading: no file, or files that failed to
    /// load, also while they load again at a new size.
    readonly property bool missing: files === "" || image.failed === files
    /// The cover is on screen, maybe at its last size while it loads at a new one.
    readonly property bool shown: files !== "" && image.held === files

    implicitWidth: size
    implicitHeight: size
    radius: 4
    color: shown ? "transparent" : Theme.raised

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

        /// The files of the image it holds; empty without one.
        property string held
        /// The files it last failed to load.
        property string failed
        /// The files `source` asks for.
        property string asked

        anchors.fill: parent
        // Hidden while it holds another cover. A delegate reused for another item keeps the old
        // one until the new one loads.
        visible: cover.shown
        asynchronous: true
        // A resizing window asks for every cover again at each new size. The last one stays on
        // screen until the new one loads, rather than a bare tile.
        retainWhileLoading: true
        sourceSize: Qt.size(cover.width, cover.height)
        source: cover.files === "" ? "" : "image://cover/" + (cover.radius / cover.width).toFixed(4) + "/" + cover.files
        // Runs before the new source starts loading, which a cached image finishes at once.
        onSourceChanged: asked = cover.files
        onStatusChanged: {
            if (status === Image.Loading)
                return;
            held = status === Image.Ready ? asked : "";
            failed = status === Image.Error ? asked : "";
        }
    }
}
