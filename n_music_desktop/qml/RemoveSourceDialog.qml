import QtQuick
import NMusic

// Asks before taking a source and its tracks out of the library.
PromptDialog {
    id: dialog

    /// The source asked about, as `Sources.items` writes its location.
    property string location

    /// The source was taken out.
    signal removed

    /// Asks about the source at `location`, called `name`.
    function askFor(location: string, name: string) {
        dialog.location = location;
        title = Tr.t.remove_source_title.arg(name);
        open();
    }

    message: Tr.t.remove_source_message
    confirmText: Tr.t.remove
    danger: true
    onConfirmed: {
        Sources.remove(location);
        removed();
    }
}
