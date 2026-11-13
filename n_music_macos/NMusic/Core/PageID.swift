import Foundation

/// Page ids, exactly as the Qt app writes them (spec §2.1): `<section>` or
/// `<section>:<argument>`. Sections: tracks, albums, album, artists, artist, genres, genre,
/// sources, source, playlist, queue, settings.
///
/// Build them with these helpers, never by hand, so every page agrees on the arguments:
///
///     Navigation.shared.go(PageID.album(name: row.name, artist: row.artist))
///     Navigation.shared.go(PageID.playlist(id))
enum PageID {
    static let tracks = "tracks"
    static let albums = "albums"
    static let artists = "artists"
    static let genres = "genres"
    static let sources = "sources"
    static let queue = "queue"
    /// Settings at the section it showed last (`settings:playback` the first time).
    static let settings = "settings"

    /// The sections in the Qt app's creation order.
    static let sections = [
        "tracks", "albums", "album", "artists", "artist", "genres", "genre",
        "sources", "source", "playlist", "queue", "settings",
    ]

    /// `encodeURIComponent`, as the Qt app's arguments are written.
    static func encode(_ text: String) -> String {
        var allowed = CharacterSet.alphanumerics
        allowed.insert(charactersIn: "-_.!~*'()")
        return text.addingPercentEncoding(withAllowedCharacters: allowed) ?? text
    }

    static func decode(_ argument: String) -> String {
        argument.removingPercentEncoding ?? argument
    }

    /// An album's page: `album:` + the encoded JSON `{"name":…,"artist":…}`, `artist` being
    /// its album artist. A `nil` name is the Unknown album page.
    static func album(name: String?, artist: String?) -> String {
        let key: [String: String] = ["name": name ?? "", "artist": artist ?? ""]
        let data = (try? JSONSerialization.data(withJSONObject: key, options: [.sortedKeys])) ?? Data()
        return "album:" + encode(String(decoding: data, as: UTF8.self))
    }

    /// An artist's page; `nil` (or empty) is Unknown artist.
    static func artist(_ name: String?) -> String { "artist:" + encode(name ?? "") }

    /// A genre's page; `nil` (or empty) is Unknown genre.
    static func genre(_ name: String?) -> String { "genre:" + encode(name ?? "") }

    /// A source's page, by its root.
    static func source(_ root: Locator) -> String { "source:" + encode(root.key) }

    static func playlist(_ id: PlaylistId) -> String { "playlist:\(id)" }

    /// One of `playback`, `telegram`, `appearance`, `updates`, `about`.
    static func settings(_ section: String) -> String { "settings:" + section }

    // MARK: Reading them back

    /// The section of `page`: what comes before its `:`.
    static func section(of page: String) -> String {
        page.firstIndex(of: ":").map { String(page[..<$0]) } ?? page
    }

    /// What follows the `:` of `page`; empty without one.
    static func argument(of page: String) -> String {
        page.firstIndex(of: ":").map { String(page[page.index(after: $0)...]) } ?? ""
    }

    /// The album an `album:` argument names: (name, album artist); empty strings for none.
    static func albumKey(_ argument: String) -> (name: String, artist: String) {
        guard let object = try? JSONSerialization.jsonObject(with: Data(decode(argument).utf8)) as? [String: Any]
        else { return ("", "") }
        return ((object["name"] as? String) ?? "", (object["artist"] as? String) ?? "")
    }

    /// The name an `artist:` or `genre:` argument names; empty for the Unknown page.
    static func name(_ argument: String) -> String { decode(argument) }

    /// The root a `source:` argument names, among `roots`.
    static func sourceRoot(_ argument: String, among roots: [Locator]) -> Locator? {
        let key = decode(argument)
        return roots.first { $0.key == key }
    }

    static func playlistID(_ argument: String) -> PlaylistId? { PlaylistId(argument) }
}

/// What a groups page (Albums, Artists, Genres) lists.
enum GroupKind: String, Sendable, CaseIterable {
    case album, artist, genre
}

// What a collection page lists is the core's `CollectionKind` (.album, .artist, .genre,
// .source), from the generated bindings.
