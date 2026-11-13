import Foundation

// What the views need of the core's generated types (Generated/NMusicCore.swift), beyond what
// the bindings give.

extension Locator {
    /// Tells tracks and sources apart in lists and page ids: what the locator points at.
    var key: String {
        switch self {
        case .local(let path): "local:\(path)"
        case .documentTree(let uri): "tree:\(uri)"
        case .document(let uri, _): "document:\(uri)"
        case .web(let address): "web:\(address)"
        case .telegramChat(let chat): "telegram:\(chat)"
        case .telegramAudio(let uri, _): "telegramAudio:\(uri)"
        }
    }

    /// Where it is, as the Qt app shows it: the path or the address.
    var location: String {
        switch self {
        case .local(let path): path
        case .documentTree(let uri), .web(let uri), .telegramChat(let uri): uri
        case .document(let uri, _), .telegramAudio(let uri, _): uri
        }
    }

    /// The file it is, for a local track or folder.
    var fileURL: URL? {
        if case .local(let path) = self {
            return URL(fileURLWithPath: path)
        }
        return nil
    }

    var isRemote: Bool {
        switch self {
        case .web, .telegramChat, .telegramAudio: true
        case .local, .documentTree, .document: false
        }
    }

    /// What it's called without a name of its own: its folder's or playlist's name.
    var defaultName: String { defaultSourceName(root: self) }
}

extension TrackRow: Identifiable {
    public var id: String { locator.key }

    /// "artist · album", empty parts dropped, as the player bar's second line.
    var artistAndAlbum: String {
        [artist, album ?? ""].filter { !$0.isEmpty }.joined(separator: " · ")
    }
}

extension QueueRow: Identifiable {
    public var id: ItemId { item }
}

extension AlbumRow: Identifiable {
    public var id: String { "\(name ?? "")\u{1}\(artist ?? "")" }

    /// The tracks of this album: by name and album artist; without a name, the untagged ones.
    var filter: Filter {
        guard let name else { return .untagged(.album) }
        var parts: [Filter] = [.album(name)]
        if let artist { parts.append(.albumArtist(artist)) }
        return .all(parts)
    }

    var page: String { PageID.album(name: name, artist: artist) }
}

extension ArtistRow: Identifiable {
    public var id: String { name ?? "" }
    var filter: Filter { name.map(Filter.artist) ?? .untagged(.artist) }
    var page: String { PageID.artist(name) }
}

extension GenreRow: Identifiable {
    public var id: String { name ?? "" }
    var filter: Filter { name.map(Filter.genre) ?? .untagged(.genre) }
    var page: String { PageID.genre(name) }
}

extension PlaylistRow: Identifiable {
    var isSmart: Bool { rule != nil }
    var page: String { PageID.playlist(id) }
}

extension PlaylistSummary: Identifiable {
    var page: String { PageID.playlist(id) }
}

extension SourceRow: Identifiable {
    public var id: String { root.key }
    var displayName: String { name ?? root.defaultName }
    var page: String { PageID.source(root) }
}

extension OutputDevice: Identifiable {}

extension Filter {
    /// Every track.
    static let everything = Filter.all([])

    /// This filter narrowed by a search, when there is one.
    func searching(_ text: String) -> Filter {
        let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return self }
        if case .all(let parts) = self {
            return .all(parts + [.search(text)])
        }
        return .all([self, .search(text)])
    }
}

extension Query {
    /// The whole library in scan order: what Play on an empty player plays.
    static var library: Query { libraryQuery() }
}

extension LoopStatus {
    /// Qt's repeat number: 0 off, 1 repeat all, 2 repeat one.
    var repeatIndex: Int {
        switch self {
        case .off: 0
        case .playlist: 1
        case .file: 2
        }
    }

    init(repeatIndex: Int) {
        self = switch repeatIndex {
        case 1: .playlist
        case 2: .file
        default: .off
        }
    }

    /// The next one the repeat button turns to: off → all → one → off.
    var cycled: LoopStatus { LoopStatus(repeatIndex: (repeatIndex + 1) % 3) }

    /// The SF Symbol of the repeat button.
    var symbol: String { self == .file ? "repeat.1" : "repeat" }

    /// The translation key of the repeat button's name: `repeat_off`, `repeat_all`, `repeat_one`.
    var labelKey: String {
        switch self {
        case .off: "repeat_off"
        case .playlist: "repeat_all"
        case .file: "repeat_one"
        }
    }
}
