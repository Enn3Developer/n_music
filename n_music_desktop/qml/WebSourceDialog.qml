import QtQuick
import NMusic

// Asks for the address of a playlist on the web, refusing what is not one.
PromptDialog {
    id: dialog

    /// The address typed, as `Sources.webAddress` writes it.
    signal chosen(string address)

    title: Tr.t.add_web_source
    message: Tr.t.web_source_message
    asksText: true
    placeholder: "https://example.com/music.m3u"
    confirmText: Tr.t.add
    check: text => Sources.webAddress(text) === "" ? Tr.t.web_address_invalid : ""
    onConfirmed: text => chosen(Sources.webAddress(text))
}
