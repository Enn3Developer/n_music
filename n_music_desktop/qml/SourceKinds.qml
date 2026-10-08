pragma Singleton
import QtQuick
import NMusic

// The kinds of sources a library can have. The core reads local folders, web playlists and
// Telegram chats, the last in builds that sign in to Telegram: the others are `later`.
QtObject {
    readonly property var all: [
        {
            value: "folder",
            name: Tr.t.source_folder,
            group: Tr.t.local_folders,
            detail: Tr.t.source_folder_detail,
            icon: "folder",
            later: false
        },
        {
            value: "web",
            name: Tr.t.source_web,
            group: Tr.t.web_playlists,
            detail: Tr.t.source_web_detail,
            icon: "link",
            later: false
        },
        {
            value: "telegram",
            name: Tr.t.source_telegram,
            group: Tr.t.telegram_chats,
            detail: Tr.t.source_telegram_detail,
            icon: "send",
            later: !Telegram.available
        },
        {
            value: "spotify",
            name: "Spotify",
            group: "Spotify",
            detail: Tr.t.source_spotify_detail,
            icon: "cloud",
            later: true
        },
        {
            value: "youtube",
            name: "YouTube",
            group: "YouTube",
            detail: Tr.t.source_youtube_detail,
            icon: "cloud",
            later: true
        },
        {
            value: "deezer",
            name: "Deezer",
            group: "Deezer",
            detail: Tr.t.source_deezer_detail,
            icon: "cloud",
            later: true
        }
    ]

    /// The name of the kind `value`, like `Local folder`.
    function name(value: string): string {
        return all.find(kind => kind.value === value)?.name ?? value;
    }

    function icon(value: string): string {
        return all.find(kind => kind.value === value)?.icon ?? "folder";
    }

    /// The kind of the source at `location`: `web` for an http or https address, `telegram` for
    /// a Telegram chat, else `folder`.
    function of(location: string): string {
        if (/^https?:\/\//i.test(location))
            return "web";
        return /^telegram:-?\d+$/.test(location) ? "telegram" : "folder";
    }
}
