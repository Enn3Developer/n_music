import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A small rounded tag beside a name, like a source's kind.
Control {
    id: badge

    property string text
    /// Its ground, a step off what it sits on.
    property color fill: Theme.raised

    implicitHeight: 20
    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    leftPadding: 7
    rightPadding: 7
    Accessible.role: Accessible.StaticText
    Accessible.name: text

    contentItem: Label {
        text: badge.text
        verticalAlignment: Text.AlignVCenter
        color: Theme.text2
        font.pixelSize: 11
        font.weight: Font.DemiBold
    }
    background: Rectangle {
        radius: height / 2
        color: badge.fill
    }
}
