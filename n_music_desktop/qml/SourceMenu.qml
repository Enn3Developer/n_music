import QtQuick
import NMusic

// What can be done with a source: rename it, update it, read its tags again, open its folder,
// or take it out of the library.
PopupMenu {
    id: menu

    /// The source, as `Sources.items` lists it.
    required property var source

    /// Renaming it was asked for.
    signal renameRequested
    /// Taking it out was asked for, to be confirmed first.
    signal removeRequested

    MenuEntry {
        iconName: "pencil"
        text: Tr.t.rename
        onTriggered: menu.renameRequested()
    }
    MenuEntry {
        iconName: "refresh"
        enabled: menu.source.updating === false
        text: Tr.t.update_now
        onTriggered: Sources.refresh(menu.source.location)
    }
    MenuEntry {
        iconName: "tag"
        enabled: menu.source.updating === false
        text: Tr.t.reload_metadata
        onTriggered: Sources.reload(menu.source.location)
    }
    MenuEntry {
        iconName: "folder"
        shown: menu.source.kind === "folder"
        enabled: menu.source.available === true
        text: Tr.t.open_folder
        onTriggered: Qt.openUrlExternally(Sources.folderUrl(menu.source.location))
    }
    MenuLine {}
    MenuEntry {
        iconName: "trash"
        danger: true
        text: Tr.t.remove
        onTriggered: menu.removeRequested()
    }
}
