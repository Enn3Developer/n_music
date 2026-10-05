import QtQuick

// The app's icon, its corners rounded like the tiles around it.
Item {
    id: logo

    property real size: 28
    property real radius: 8

    implicitWidth: size
    implicitHeight: size

    Image {
        anchors.fill: parent
        sourceSize: Qt.size(logo.size, logo.size)
        source: "image://cover/" + (logo.radius / logo.size).toFixed(4) + "/" + encodeURIComponent(":/qt/qml/NMusic/assets/icons/icon.png")
    }
}
