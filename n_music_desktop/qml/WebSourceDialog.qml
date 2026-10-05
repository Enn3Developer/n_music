import QtQuick
import NMusic

// Asks for the address of a playlist on the web, refusing what is not one, and for a name for
// it, which can be left empty.
PromptDialog {
    id: dialog

    /// The address typed, as `Sources.webAddress` writes it, named `name` unless that is empty.
    signal chosen(string address, string name)

    readonly property string address: Sources.webAddress(text)

    title: Tr.t.add_web_source
    message: Tr.t.web_source_message
    asksText: true
    placeholder: "https://example.com/music.m3u"
    confirmText: Tr.t.add
    check: text => Sources.webAddress(text) === "" ? Tr.t.web_address_invalid : ""
    onAboutToShow: naming.text = ""
    onConfirmed: text => chosen(Sources.webAddress(text), naming.text.trim())

    SourceNameField {
        id: naming
        width: parent.width
        defaultName: dialog.address !== "" ? Sources.defaultName(dialog.address) : ""
    }
}
