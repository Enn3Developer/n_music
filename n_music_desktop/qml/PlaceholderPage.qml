import QtQuick
import QtQuick.Controls.Basic
import NMusic

// A page whose content comes in a later step: only its title.
Item {
    id: page

    property string title

    Label {
        x: 28
        y: 22
        text: page.title
        color: Theme.text
        font.pixelSize: 26
        font.weight: Font.Bold
        font.letterSpacing: -0.52
    }
}
