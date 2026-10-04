pragma Singleton
import QtQuick
import NMusic

// The kinds of sources a library can have. The core only reads local folders so far: the others
// are `later`.
QtObject {
    readonly property var all: [
        {
            value: "folder",
            name: Tr.t.source_folder,
            detail: Tr.t.source_folder_detail,
            icon: "folder",
            later: false
        },
        {
            value: "web",
            name: Tr.t.source_web,
            detail: Tr.t.source_web_detail,
            icon: "link",
            later: true
        },
        {
            value: "spotify",
            name: "Spotify",
            detail: Tr.t.source_spotify_detail,
            icon: "cloud",
            later: true
        },
        {
            value: "youtube",
            name: "YouTube",
            detail: Tr.t.source_youtube_detail,
            icon: "cloud",
            later: true
        },
        {
            value: "deezer",
            name: "Deezer",
            detail: Tr.t.source_deezer_detail,
            icon: "cloud",
            later: true
        }
    ]

    /// The name of the kind `value`, like `Local folder`.
    function name(value: string): string {
        return all.find(kind => kind.value === value)?.name ?? value;
    }
}
