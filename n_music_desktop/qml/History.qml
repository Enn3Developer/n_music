pragma Singleton
import QtQuick
import NMusic

// Where the window has been: the page it shows, and the pages to go back and forward to.
QtObject {
    id: history

    /// The page shown, see `Main.page`.
    property string page: "tracks"
    /// The pages before it, the latest last.
    property list<string> earlier: []
    /// The pages gone back from, the next one last.
    property list<string> later: []
    readonly property bool canGoBack: earlier.length > 0
    readonly property bool canGoForward: later.length > 0

    /// How many pages back it remembers.
    readonly property int depth: 100

    /// Shows `to`; the pages gone back from are forgotten.
    function go(to: string) {
        if (to === page)
            return;
        earlier = earlier.slice(1 - depth).concat([page]);
        later = [];
        page = to;
    }

    function back() {
        if (!canGoBack)
            return;
        const to = earlier[earlier.length - 1];
        earlier = earlier.slice(0, -1);
        later = later.concat([page]);
        page = to;
    }

    function forward() {
        if (!canGoForward)
            return;
        const to = later[later.length - 1];
        later = later.slice(0, -1);
        earlier = earlier.concat([page]);
        page = to;
    }

    /// `pages` without those `keep` refuses, nor the same page twice in a row or right
    /// before the one shown.
    // Not `list<string>`: returned empty, Qt hands it over as `[""]`.
    function tidy(pages: list<string>, keep: var): var {
        const kept = [];
        for (const entry of pages) {
            if (keep(entry) && entry !== kept[kept.length - 1])
                kept.push(entry);
        }
        if (kept[kept.length - 1] === page)
            kept.pop();
        return kept;
    }

    // A deleted playlist's page has nothing to show any more.
    property Connections playlists: Connections {
        target: Playlists

        function onItemsChanged() {
            const pages = Playlists.items.map(playlist => "playlist:" + playlist.id);
            const keep = entry => !entry.startsWith("playlist:") || pages.includes(entry);
            history.earlier = history.tidy(history.earlier, keep);
            history.later = history.tidy(history.later, keep);
        }
    }
}
