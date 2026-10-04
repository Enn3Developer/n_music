import QtQuick
import QtQuick.Layouts
import NMusic

// A rounded card of settings, one row under the other.
Rectangle {
    id: group

    default property alias rows: column.data

    implicitHeight: column.implicitHeight + 2
    radius: 12
    color: Theme.surface
    border.width: 1
    border.color: Theme.line2

    ColumnLayout {
        id: column
        anchors.fill: parent
        anchors.margins: 1
        spacing: 0
    }
}
