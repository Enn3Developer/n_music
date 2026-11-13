import SwiftUI

/// The Albums, Artists or Genres grid (spec §3.3, `qml/GroupsPage.qml`): the title over the
/// count, a search field and the group sort on the right, then a grid of cards (square covers,
/// round for artists) that open the group's collection page. The sort is not persisted; the
/// search, sort and scroll last for the session (the page stays alive).
struct GroupsPage: View {
    let kind: GroupKind

    private let store = Store.shared
    @State private var search = ""
    @State private var sort: GroupSort
    @State private var cards: [GroupsCard] = []
    /// The first list arrived: nothing shows before (spec §9.2).
    @State private var ready = false
    /// The first cards fade in and rise, one after another; later ones show at once.
    @State private var entering = true
    /// The search the grid lists: the field's, 150 ms after the last keystroke.
    @State private var searched = ""
    @FocusState private var searchFocused: Bool

    init(kind: GroupKind) {
        self.kind = kind
        _sort = State(initialValue: kind == .album ? .artist : .name)
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            if !cards.isEmpty {
                GroupsGrid(kind: kind, cards: cards, entering: entering)
            } else {
                emptyState
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .task(id: search) {
            // Typing waits for a pause; clearing the field lists everything at once.
            if !search.isEmpty && search != searched {
                try? await Task.sleep(for: .milliseconds(150))
                guard !Task.isCancelled else { return }
            }
            searched = search
        }
        .task(id: QueryKey(search: searched, sort: sort, version: store.libraryVersion)) {
            await load()
        }
        #if DEBUG
        .onAppear { GroupsDebug.shared.install() }
        .onChange(of: GroupsDebug.shared.command?.serial) {
            guard let command = GroupsDebug.shared.command, Navigation.shared.section == kind.rawValue + "s" else { return }
            switch command.verb {
            case "search": search = command.argument
            case "sort":
                let sorts: [String: GroupSort] = ["artist": .artist, "name": .name, "newest": .newest, "oldest": .oldest, "mostTracks": .mostTracks]
                if let value = sorts[command.argument] { sort = value }
            default: break
            }
        }
        #endif
    }

    // MARK: Header

    private var header: some View {
        HStack(alignment: .bottom, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text(words.title)
                    .font(.system(size: 26, weight: .bold))
                    .tracking(-0.52)
                    .foregroundStyle(Theme.text)
                    .accessibilityAddTraits(.isHeader)
                Text(ready ? Format.count(cards.count, words.one, words.many) : " ")
                    .font(.system(size: 13).monospacedDigit())
                    .foregroundStyle(Theme.text2)
            }
            .layoutPriority(1)
            Spacer(minLength: 0)
            // Qt's SearchField (38 tall, 14-point text), the one the Tracks page uses.
            TracksSearchField(text: $search, placeholder: words.search, focused: $searchFocused)
                .frame(minWidth: 160, idealWidth: 260, maxWidth: 260)
            GroupsMenuButton(options: orders, value: sort) { sort = $0 }
        }
        .padding(.leading, 28)
        .padding(.trailing, 28)
        .padding(.top, 22)
        .padding(.bottom, 16)
    }

    // MARK: Empty states (spec §9.1)

    @ViewBuilder private var emptyState: some View {
        if ready {
            // The field's text, as Qt reads it (`qml/GroupsPage.qml:350`).
            if search.trimmingCharacters(in: .whitespaces).isEmpty {
                if store.scan != nil {
                    EmptyStateView(symbol: "arrow.clockwise", title: tr("scanning_library"))
                } else {
                    EmptyStateView(
                        symbol: "folder",
                        title: tr("empty_library"),
                        message: tr("empty_library_hint"),
                        buttonTitle: tr("open_sources")
                    ) { Navigation.shared.go(PageID.sources) }
                }
            } else {
                EmptyStateView(symbol: "magnifyingglass", title: tr("no_results"))
            }
        } else {
            Color.clear.frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    // MARK: Data

    private func load() async {
        let search = searched
        let sort = sort
        let unknown = words.unknown
        let loaded: [GroupsCard]
        switch kind {
        case .album:
            let rows = await store.albums(search: search, sort: sort)
            let unknownArtist = tr("unknown_artist")
            loaded = rows.map { row in
                GroupsCard(
                    id: row.id,
                    name: row.name ?? unknown,
                    known: row.name != nil,
                    detail: [row.artist ?? unknownArtist, row.year.map(String.init) ?? "—"].joined(separator: " · "),
                    cover: row.cover,
                    page: row.page
                )
            }
        case .artist:
            let rows = await store.artists(search: search, sort: sort)
            loaded = rows.map { row in
                GroupsCard(
                    id: row.id, name: row.name ?? unknown, known: row.name != nil,
                    detail: Format.count(row.tracks, "track_one", "tracks_many"),
                    cover: row.cover, page: row.page
                )
            }
        case .genre:
            let rows = await store.genres(search: search, sort: sort)
            loaded = rows.map { row in
                GroupsCard(
                    id: row.id, name: row.name ?? unknown, known: row.name != nil,
                    detail: Format.count(row.tracks, "track_one", "tracks_many"),
                    cover: row.covers.first, page: row.page
                )
            }
        }
        guard !Task.isCancelled else { return }
        if loaded != cards { cards = loaded }
        if !ready {
            ready = true
            // The cards laid out for the first list fade in; later ones show at once.
            Task {
                try? await Task.sleep(for: .milliseconds(300))
                entering = false
            }
        }
    }

    private struct QueryKey: Equatable {
        var search: String
        var sort: GroupSort
        var version: Int
    }

    // MARK: Wording (`qml/GroupsPage.qml:16-69`)

    private var words: (title: String, one: String, many: String, search: String, unknown: String) {
        switch kind {
        case .album: (tr("albums"), "albums_one", "albums_many", tr("search_albums"), tr("unknown_album"))
        case .artist: (tr("artists"), "artists_one", "artists_many", tr("search_artists"), tr("unknown_artist"))
        case .genre: (tr("genres"), "genres_one", "genres_many", tr("search_genres"), tr("unknown_genre"))
        }
    }

    private var orders: [(value: GroupSort, label: String)] {
        switch kind {
        case .album:
            [(.artist, tr("sort_album_artist")), (.name, tr("sort_title")),
             (.newest, tr("sort_newest")), (.oldest, tr("sort_oldest"))]
        case .artist, .genre:
            [(.name, tr("sort_name")), (.mostTracks, tr("sort_most_tracks"))]
        }
    }
}

/// One card of a grid: what it shows and the page it opens.
struct GroupsCard: Identifiable, Equatable, Sendable {
    let id: String
    let name: String
    /// False for the Unknown group, whose name is in `text2`.
    let known: Bool
    let detail: String
    let cover: String?
    let page: String
}

/// The grid (`qml/GroupsPage.qml:192-339`): max(1, ⌊width / 188⌋) columns filling the width,
/// 20 apart, 24 between rows; margins 28 left, 8 + the gap right. Lazy: only the cards on screen
/// exist, and their covers decode at the size they show at.
private struct GroupsGrid: View {
    let kind: GroupKind
    let cards: [GroupsCard]
    let entering: Bool

    var body: some View {
        GeometryReader { geometry in
            let gap: CGFloat = 20
            let width = max(1, geometry.size.width - 36)
            let columns = max(1, Int(width / 188))
            let side = max(1, (width / CGFloat(columns) - gap).rounded(.down))
            ScrollView {
                LazyVGrid(
                    columns: Array(repeating: GridItem(.fixed(side), spacing: gap, alignment: .top), count: columns),
                    alignment: .leading,
                    spacing: 24
                ) {
                    ForEach(Array(cards.enumerated()), id: \.element.id) { index, card in
                        GroupsCardView(kind: kind, card: card, side: side, delay: entering ? min(Double(index) * 0.016, 0.08) : nil)
                    }
                }
                .padding(.leading, 28)
                .padding(.trailing, 8 + gap)
                .padding(.bottom, 24)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .scrollIndicators(.automatic)
            .accessibilityLabel(title)
        }
    }

    private var title: String {
        switch kind {
        case .album: tr("albums")
        case .artist: tr("artists")
        case .genre: tr("genres")
        }
    }
}

/// A card: the cover (radius 8, round for artists) with a 6 % white veil on hover, scaling to
/// 97 % and fading to 86 % while pressed; the name (14 SemiBold) over the detail (13, `text2`).
private struct GroupsCardView: View {
    let kind: GroupKind
    let card: GroupsCard
    let side: CGFloat
    /// Fades in after this delay when the first cards come; `nil` shows at once.
    let delay: Double?

    @State private var hovering = false
    @State private var shown = true

    var body: some View {
        Button {
            Navigation.shared.go(card.page)
        } label: {
            VStack(alignment: .leading, spacing: 10) {
                // Hover, press and the art's veil live in the style: only the cover scales.
                Color.clear.frame(width: side, height: side)
                VStack(alignment: .leading, spacing: 2) {
                    Text(card.name)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(card.known ? Theme.text : Theme.text2)
                        .lineLimit(1)
                        .truncationMode(.tail)
                    Text(card.detail)
                        .font(.system(size: 13))
                        .foregroundStyle(Theme.text2)
                        .lineLimit(1)
                        .truncationMode(.tail)
                }
                .frame(width: side, alignment: .leading)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(GroupsCardStyle(art: art, round: kind == .artist, side: side, hovering: hovering))
        .onHover { hovering = $0 }
        .animation(Motion.standard(Motion.fade), value: hovering)
        .opacity(shown ? 1 : 0)
        .offset(y: shown ? 0 : 8 * Motion.shared.travel)
        .accessibilityLabel(card.name)
        .accessibilityHint(card.detail)
        .onAppear {
            guard let delay, !Motion.shared.reduced else { return }
            var instant = Transaction()
            instant.disablesAnimations = true
            withTransaction(instant) { shown = false }
            withAnimation(Motion.standard(Motion.move).delay(delay)) { shown = true }
        }
    }

    private var art: GroupsArt {
        GroupsArt(path: card.cover, size: side, radius: kind == .artist ? nil : 8, placeholder: placeholder)
    }

    private var placeholder: GroupsPlaceholder {
        switch kind {
        case .album: .logo
        case .artist: .symbol("person")
        case .genre: .symbol("tag")
        }
    }
}

/// The card's cover, drawn over the label's top square: a 6 % white veil on hover; pressed, it
/// scales to 97 % around its centre and fades to 86 % (`qml/GroupsPage.qml:287-296`); the
/// name and detail stay put.
private struct GroupsCardStyle: ButtonStyle {
    let art: GroupsArt
    let round: Bool
    let side: CGFloat
    let hovering: Bool

    func makeBody(configuration: Configuration) -> some View {
        let pressed = configuration.isPressed
        configuration.label
            .overlay(alignment: .topLeading) {
                art
                    .overlay {
                        if hovering {
                            Color.white.opacity(0.06)
                                .clipShape(round ? AnyShape(Circle()) : AnyShape(RoundedRectangle(cornerRadius: 8, style: .continuous)))
                        }
                    }
                    .scaleEffect(pressed ? 0.97 : 1)
                    .opacity(pressed ? 0.86 : 1)
                    .animation(Motion.standard(pressed ? Motion.exit : Motion.fade), value: pressed)
                    .allowsHitTesting(false)
            }
    }
}
