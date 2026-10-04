import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A filter rule, like `Genre is J-Pop`, with a button removing it while `removable`.
AbstractButton {
    id: chip

    /// What the rule tests, like `Genre`.
    property string name
    /// The rest of the rule, like `is J-Pop`.
    property string detail
    property bool removable: true

    signal remove

    implicitHeight: removable ? 32 : 28
    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    leftPadding: removable ? 12 : 10
    rightPadding: removable ? 4 : 10
    hoverEnabled: true
    font.pixelSize: 13
    text: name + " " + detail
    Accessible.name: text

    background: Rectangle {
        radius: height / 2
        color: chip.hovered ? Theme.menuHover : Theme.raised
        border.width: chip.visualFocus ? 2 : 0
        border.color: Theme.text
    }

    contentItem: Row {
        spacing: chip.removable ? 6 : 5

        Label {
            anchors.verticalCenter: parent.verticalCenter
            text: chip.name
            font: chip.font
            color: Theme.text2
        }
        Label {
            anchors.verticalCenter: parent.verticalCenter
            text: chip.detail
            font: chip.font
            color: Theme.text
        }
        IconButton {
            anchors.verticalCenter: parent.verticalCenter
            visible: chip.removable
            size: 24
            radius: 12
            iconSize: 12
            stroke: 2.4
            iconName: "close"
            text: Tr.t.remove_filter.arg(chip.text)
            focusPolicy: Qt.TabFocus
            onClicked: chip.remove()
        }
    }
}
