import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// How far the running scan got: the tracks read of those found, as a bar.
Rectangle {
    implicitHeight: progress.implicitHeight + 32
    radius: 12
    color: Theme.surface
    border.width: 1
    border.color: Theme.line2
    Accessible.role: Accessible.ProgressBar
    Accessible.name: Tr.t.updating_library

    ColumnLayout {
        id: progress
        anchors.fill: parent
        anchors.leftMargin: 18
        anchors.rightMargin: 18
        anchors.topMargin: 16
        anchors.bottomMargin: 16
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Icon {
                name: "refresh"
                size: 18
                stroke: 2
                color: Theme.accentText
            }
            Label {
                Layout.fillWidth: true
                text: Tr.t.updating_library
                elide: Text.ElideRight
                color: Theme.text
                font.pixelSize: 15
                font.weight: Font.DemiBold
            }
            Label {
                text: Scan.found > 0 ? Tr.t.tracks_read.arg(Format.number(Scan.read)).arg(Format.number(Scan.found)) : Tr.t.scanning_library
                color: Theme.text2
                font.pixelSize: 14
                font.features: {
                    "tnum": 1
                }
            }
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 6
            radius: 3
            color: Theme.line2

            Rectangle {
                width: Scan.found > 0 ? parent.width * Math.min(1, Scan.read / Scan.found) : 0
                height: parent.height
                radius: 3
                color: Theme.accent
            }
        }
        Label {
            Layout.fillWidth: true
            text: Tr.t.updating_library_hint
            wrapMode: Text.Wrap
            color: Theme.text3
            font.pixelSize: 13
        }
    }
}
