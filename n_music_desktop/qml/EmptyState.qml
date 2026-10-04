import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// A message in place of an empty list, with an optional action.
ColumnLayout {
    id: empty

    property string iconName
    property string title
    property string message
    property string action

    signal triggered

    width: 360
    spacing: 0

    Rectangle {
        Layout.alignment: Qt.AlignHCenter
        implicitWidth: 64
        implicitHeight: 64
        radius: 32
        color: Theme.raised

        Icon {
            anchors.centerIn: parent
            name: empty.iconName
            size: 28
            color: Theme.text2
        }
    }
    Label {
        Layout.topMargin: 18
        Layout.fillWidth: true
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        text: empty.title
        color: Theme.text
        font.pixelSize: 17
        font.weight: Font.DemiBold
    }
    Label {
        Layout.topMargin: 6
        Layout.fillWidth: true
        visible: empty.message !== ""
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        text: empty.message
        color: Theme.text2
        font.pixelSize: 14
    }
    PillButton {
        Layout.topMargin: 20
        Layout.alignment: Qt.AlignHCenter
        visible: empty.action !== ""
        primary: true
        text: empty.action
        onClicked: empty.triggered()
    }
}
