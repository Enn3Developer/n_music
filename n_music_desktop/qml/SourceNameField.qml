import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// A name for a source being added, which can be left empty: the name it gets then shows in
// its place.
RowLayout {
    id: row

    property alias text: field.text
    /// What the source is called without a name of its own; empty while that is unknown.
    property string defaultName

    spacing: 12

    Label {
        text: Tr.t.display_name
        color: Theme.text2
        font.pixelSize: 13
        font.weight: Font.Medium
    }
    TextBox {
        id: field
        Layout.fillWidth: true
        placeholderText: row.defaultName !== "" ? row.defaultName : Tr.t.optional
        Accessible.name: Tr.t.display_name
    }
}
