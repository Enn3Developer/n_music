import AppKit
import SwiftUI

/// The page of one album, artist, genre or source (spec §3.4, `qml/CollectionPage.qml`): the
/// cover beside the eyebrow, the name (a source's with a rename pencil), the summary, Play,
/// Shuffle, a source's "more" menu and the sort; then the shared track table, or "No tracks
/// match". The page stays alive: `argument` changes when another album opens; the sort stays
/// (it is not persisted).
struct CollectionPage: View {
    let kind: CollectionKind
    let argument: String

    private let store = Store.shared
    @State private var list: TrackList
    /// Qt's narrow header (see `groupsNarrowHeader(_:)`).
    @State private var narrow = false
    /// A source page's source as the Sources page lists it (for its "more" menu's states).
    @State private var info: SourceInfo?
    @State private var renaming: SourceInfo?
    @State private var removing: SourceInfo?

    init(kind: CollectionKind, argument: String) {
        self.kind = kind
        self.argument = argument
        // Not persisted (`qml/CollectionPage.qml:59`).
        _list = State(initialValue: TrackList(sort: kind == .genre || kind == .source ? "artist,album" : "album"))
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            if missing {
                // A source page without its source lists nothing, and reads nothing.
                EmptyStateView(symbol: "magnifyingglass", title: tr("no_results"))
                    .offset(y: 20)
            } else if list.ready && list.count > 0 {
                TrackTable(list: list, layout: .library)
            } else if list.ready {
                EmptyStateView(symbol: "magnifyingglass", title: tr("no_results"))
                    .offset(y: 20)
            } else {
                Color.clear.frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .groupsNarrowHeader($narrow)
        .task(id: SourceKey(
            root: root, version: store.libraryVersion, roots: store.roots,
            scanning: store.scan?.libraries, names: store.sources
        )) {
            guard let root else {
                info = nil
                return
            }
            let list = await store.sourceList()
            guard !Task.isCancelled else { return }
            let found = list.first { $0.root == root }
            if info != found { info = found }
        }
        .onAppear {
            configure()
            list.activate()
        }
        .onChange(of: argument) { configure() }
        .onChange(of: store.roots ?? []) { configure() }
        .onChange(of: name) { list.label = name }
        #if DEBUG
        .onAppear { GroupsDebug.shared.install() }
        .onChange(of: GroupsDebug.shared.command?.serial) {
            guard let command = GroupsDebug.shared.command, Navigation.shared.section == kind.section else { return }
            switch command.verb {
            case "track-sort": list.sort = command.argument
            case "rename": renaming = info
            case "remove": removing = info
            case "close": renaming = nil; removing = nil
            default: break
            }
        }
        #endif
        .sourceDialogs(renaming: $renaming, removing: $removing) {
            Navigation.shared.go(PageID.sources)
        }
    }

    /// Points the list at this page's tracks (`Filters.collectionFilter`): the album by its
    /// album artist (or one of its artists), the artist, the genre, the source; Unknown pages
    /// list the untagged.
    private func configure() {
        let filter: String
        switch kind {
        case .album:
            filter = collectionFilter(kind: .album, name: albumKey.name, artist: albumKey.artist)
        case .artist, .genre:
            filter = collectionFilter(kind: kind, name: PageID.name(argument), artist: "")
        case .source:
            // Without its root (not announced yet, or removed) the page lists nothing and runs
            // no query: a source filter without a location would match every track.
            guard let root else {
                list.suspended = true
                list.label = name
                list.origin = kind.section + ":" + argument
                list.detail = ""
                return
            }
            filter = collectionFilter(kind: .source, name: sourceLocation(root: root), artist: "")
        }
        if list.filter != filter { list.filter = filter }
        list.suspended = false
        list.label = name
        list.origin = kind.section + ":" + argument
        list.detail = kind == .album ? albumKey.artist : ""
    }

    // MARK: Header (`qml/CollectionPage.qml:80-210`)

    private var header: some View {
        @Bindable var list = list
        let narrow = self.narrow
        return HStack(alignment: .bottom, spacing: narrow ? 16 : 24) {
            GroupsArt(
                path: missing ? nil : list.covers.first,
                mosaic: !missing && (kind == .genre || kind == .source) ? list.covers : [],
                size: narrow ? 96 : 168,
                radius: kind == .artist ? nil : narrow ? 8 : 10,
                placeholder: placeholder
            )
            VStack(alignment: .leading, spacing: 8) {
                if !narrow {
                    Text(eyebrow.uppercased())
                        .font(.system(size: 12, weight: .semibold))
                        .tracking(0.96)
                        .foregroundStyle(Theme.text2)
                }
                HStack(spacing: 8) {
                    Text(name)
                        .font(.system(size: narrow ? 22 : 34, weight: .bold))
                        .tracking(narrow ? -0.44 : -0.68)
                        .foregroundStyle(Theme.text)
                        .lineLimit(1)
                        .truncationMode(.tail)
                        .accessibilityAddTraits(.isHeader)
                    if source != nil {
                        IconButton(symbol: "pencil", label: tr("rename_source"), size: narrow ? 28 : 34, iconSize: 16, tint: Theme.text3) {
                            renaming = info
                        }
                    }
                }
                Text(list.ready || missing ? summary : " ")
                    .font(.system(size: 13).monospacedDigit())
                    .foregroundStyle(Theme.text2)
                    .lineLimit(1)
                HStack(spacing: 8) {
                    Button {
                        list.playAll(shuffle: false)
                    } label: {
                        Label(tr("play"), systemImage: "play.fill")
                    }
                    .buttonStyle(PillButtonStyle(kind: .primary))
                    .fixedSize()
                    .disabled(count == 0)
                    Button {
                        list.playAll(shuffle: true)
                    } label: {
                        if narrow {
                            Image(systemName: "shuffle")
                        } else {
                            Label(tr("shuffle"), systemImage: "shuffle")
                        }
                    }
                    .buttonStyle(PillButtonStyle(kind: .plain))
                    .fixedSize()
                    .help(tr("shuffle"))
                    .accessibilityLabel(tr("shuffle"))
                    .disabled(count == 0)
                    if let source {
                        // The Sources page's SourceMenu (spec §5.4), with its states.
                        GroupsMoreButton(label: tr("source_actions", source.displayName), iconSize: 18, tint: Theme.text2) {
                            guard let info else { return [] }
                            return SourceCard.menuEntries(info, rename: { renaming = info }, remove: { removing = info })
                        }
                    }
                    Spacer(minLength: 0)
                    GroupsSortButton(sort: $list.sort, narrow: narrow)
                }
                .padding(.top, 6)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, narrow ? 16 : 28)
        .padding(.top, narrow ? 16 : 28)
        .padding(.bottom, narrow ? 12 : 18)
    }

    // MARK: Identity

    private var albumKey: (name: String, artist: String) { PageID.albumKey(argument) }

    private var root: Locator? {
        kind == .source ? PageID.sourceRoot(argument, among: store.roots ?? []) : nil
    }

    /// A source page whose source is not (or no longer) in the library.
    private var missing: Bool { kind == .source && root == nil }

    /// The tracks listed; none on a page without its source (the list keeps its last answer).
    private var count: Int { missing ? 0 : list.count }

    /// The source listed, as the store lists it; nil for other pages or a removed source.
    private var source: SourceRow? {
        guard let root else { return nil }
        return store.sources.first { $0.root == root }
    }

    /// The page lists the tracks without the tag (Unknown album…).
    private var unknown: Bool {
        switch kind {
        case .album: albumKey.name.isEmpty
        case .artist, .genre: PageID.name(argument).isEmpty
        case .source: false
        }
    }

    /// What the page is called; a source's changes as it is renamed.
    private var name: String {
        switch kind {
        case .album: unknown ? tr("unknown_album") : albumKey.name
        case .artist: unknown ? tr("unknown_artist") : PageID.name(argument)
        case .genre: unknown ? tr("unknown_genre") : PageID.name(argument)
        case .source: source?.displayName ?? root?.defaultName ?? ""
        }
    }

    private var eyebrow: String {
        switch kind {
        case .album: tr("field_album")
        case .artist: tr("field_artist")
        case .genre: tr("field_genre")
        case .source: tr("source")
        }
    }

    private var placeholder: GroupsPlaceholder {
        switch kind {
        case .album: .logo
        case .artist: .symbol("person")
        case .genre: .symbol("tag")
        case .source: .symbol(root.map { SourceKind($0).symbol } ?? SourceKind.folder.symbol)
        }
    }

    private var summary: String {
        var parts: [String] = []
        // The tracks without an album are by any artist.
        if kind == .album && !unknown {
            parts.append(albumKey.artist.isEmpty ? tr("unknown_artist") : albumKey.artist)
        }
        if kind == .source {
            parts.append((root.map(SourceKind.init) ?? .folder).name)
        }
        parts.append(Format.count(count, "track_one", "tracks_many"))
        if count > 0 { parts.append(Format.duration(list.length)) }
        return parts.joined(separator: " · ")
    }
}

extension CollectionKind {
    /// The page id's section.
    var section: String {
        switch self {
        case .album: "album"
        case .artist: "artist"
        case .genre: "genre"
        case .source: "source"
        }
    }
}

/// What a source page's source query depends on.
private struct SourceKey: Equatable {
    let root: Locator?
    let version: Int
    let roots: [Locator]?
    let scanning: [Locator]?
    let names: [SourceRow]
}
