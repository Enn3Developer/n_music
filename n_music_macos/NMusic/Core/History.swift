import Foundation

/// Where the window has been: the page it shows, and the pages to go back and forward to.
/// Exactly the Qt app's `History.qml` (spec §2.4).
struct History: Equatable, Sendable {
    /// How many pages back it remembers.
    static let depth = 100

    /// The page shown.
    private(set) var page = PageID.tracks
    /// The pages before it, the latest last.
    private(set) var earlier: [String] = []
    /// The pages gone back from, the next one last.
    private(set) var later: [String] = []

    var canGoBack: Bool { !earlier.isEmpty }
    var canGoForward: Bool { !later.isEmpty }

    /// Shows `to`; the pages gone back from are forgotten. Nothing happens when it shows already.
    mutating func go(_ to: String) {
        guard to != page else { return }
        earlier = Array((earlier + [page]).suffix(Self.depth))
        later = []
        page = to
    }

    mutating func back() {
        guard let to = earlier.last else { return }
        earlier.removeLast()
        later.append(page)
        page = to
    }

    mutating func forward() {
        guard let to = later.last else { return }
        later.removeLast()
        earlier.append(page)
        page = to
    }

    /// Forgets the pages of playlists that no longer exist, then the same page twice in a row
    /// and a page right before the one shown. The page shown stays.
    mutating func prune(playlists: Set<PlaylistId>) {
        prune { entry in
            guard entry.hasPrefix("playlist:") else { return true }
            return PageID.playlistID(PageID.argument(of: entry)).map(playlists.contains) ?? false
        }
    }

    /// Forgets the pages of sources whose root left the library (`roots`), the same way.
    mutating func prune(roots: [Locator]) {
        prune { entry in
            guard entry.hasPrefix("source:") else { return true }
            return PageID.sourceRoot(PageID.argument(of: entry), among: roots) != nil
        }
    }

    private mutating func prune(keeping keep: (String) -> Bool) {
        earlier = tidy(earlier, keep)
        later = tidy(later, keep)
    }

    private func tidy(_ pages: [String], _ keep: (String) -> Bool) -> [String] {
        var kept: [String] = []
        for entry in pages where keep(entry) && entry != kept.last {
            kept.append(entry)
        }
        if kept.last == page {
            kept.removeLast()
        }
        return kept
    }
}
