import QtQuick
import NMusic

// Gives a source a name of its own, or takes it back when left empty.
PromptDialog {
    id: dialog

    /// The source named, as `Sources.items` writes its location.
    property string location

    /// Asks for a name for `source`, as `Sources.items` lists it.
    function askFor(source: var) {
        location = source.location;
        placeholder = source.defaultName;
        message = Tr.t.rename_source_message.arg(source.defaultName);
        ask(source.displayName);
    }

    title: Tr.t.rename_source
    asksText: true
    optional: true
    confirmText: Tr.t.rename
    onConfirmed: name => Sources.rename(location, name)
}
