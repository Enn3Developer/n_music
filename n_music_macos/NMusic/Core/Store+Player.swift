import Foundation

/// What the play session plays from, as the queue page's "Playing from" header tells it
/// (Qt's `hub::Context`, set by every track list's Play): the list's name, how it was narrowed
/// down, and the page that shows it ("Open in library"). App state, not saved: after a launch
/// that resumes the session it is "Library", as in the Qt app.
///
/// The track lists tell it through `PlayingFrom` (`TrackList.play`, `playAll`); this reads
/// that, so there is one context whichever way a list played.
@MainActor
final class PlayContext {
    static let shared = PlayContext()

    struct Origin: Equatable, Sendable {
        /// The list's name, like `Tracks` or an album's name; empty: "Library".
        var label: String
        /// How the list was narrowed down (a search, a filter summary, an album's artist);
        /// empty parts are dropped.
        var detail: [String] = []
        /// The page id that shows the list, like `PageID.tracks` or `album.page`.
        var page: String = PageID.tracks
    }

    private init() {}

    func set(_ origin: Origin) {
        let detail = origin.detail.map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        let context = PlayingFrom.Context(label: origin.label, detail: detail.joined(separator: " · "), page: origin.page)
        if PlayingFrom.shared.context != context { PlayingFrom.shared.set(context) }
    }

    /// The label part of "Playing from %1": the list's name, else "Library".
    var label: String {
        guard let label = PlayingFrom.shared.context?.label, !label.isEmpty else { return tr("library") }
        return label
    }

    /// The detail parts, joined by " · " as Qt does.
    var detail: String { PlayingFrom.shared.context?.detail ?? "" }

    /// Where "Open in library" goes.
    var page: String {
        guard let page = PlayingFrom.shared.context?.page, !page.isEmpty else { return PageID.tracks }
        return page
    }
}

extension Store {
    /// Plays what `query` selects from `start`, telling the queue page where the session comes
    /// from (every track list's Play, row double-click and Shuffle should use this).
    func play(_ query: Query, from start: Locator? = nil, context: PlayContext.Origin) {
        PlayContext.shared.set(context)
        play(query, from: start)
    }

    /// `shufflePlay` telling the queue page where the session comes from.
    func shufflePlay(_ query: Query, context: PlayContext.Origin) {
        PlayContext.shared.set(context)
        shufflePlay(query)
    }
}
