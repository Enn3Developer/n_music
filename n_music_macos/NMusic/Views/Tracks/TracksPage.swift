import SwiftUI

/// The Tracks page's list and requests, kept for the session: the filter drawer (in the shell's
/// inspector) reads and applies its rules and order.
@MainActor
@Observable
final class TracksModel {
    static let shared = TracksModel()

    /// Every track of the library: search it, filter it, sort it (spec §3.1). The order starts
    /// as artist, then album, and is not saved.
    let list = TrackList(sort: "artist,album", label: "", origin: PageID.tracks)

    /// The rules and order to save as a smart playlist (the drawer's Save as smart playlist).
    var smartPlaylist: SmartPlaylistRequest?

    private init() {}

    /// The applied filter read once per change of it.
    @ObservationIgnored private var parsed: (filter: String, spec: FilterSpec, chips: [Chip])?

    typealias Chip = (index: Int, name: String, text: String)

    private var applied: (spec: FilterSpec, chips: [Chip]) {
        let filter = list.filter
        if let parsed, parsed.filter == filter { return (parsed.spec, parsed.chips) }
        let spec = Filters.parse(filter)
        let chips: [Chip] = spec.entries.enumerated().compactMap { index, entry in
            Filters.describe(entry).map { (index, $0.name, $0.text) }
        }
        parsed = (filter, spec, chips)
        return (spec, chips)
    }

    /// The applied rules that can be put in words, a chip each: their place in the filter and
    /// their words.
    var chips: [Chip] { applied.chips }

    /// How many rules apply, described or not (the compact badge, `qml/TracksPage.qml:334`).
    var ruleCount: Int { applied.spec.entries.count }

    func removeRule(at index: Int) {
        var spec = Filters.parse(list.filter)
        guard spec.entries.indices.contains(index) else { return }
        spec.entries.remove(at: index)
        list.filter = Filters.json(spec)
    }
}

struct SmartPlaylistRequest: Identifiable {
    let id = UUID()
    var filter: String
    var sort: String
}

/// Every track of the library (spec §3.1): the header by size class (wide one row, regular two,
/// compact one tight row), the track table, the empty states, the filter drawer's scrim.
struct TracksPage: View {
    private let store = Store.shared
    private let navigation = Navigation.shared
    private let model = TracksModel.shared

    @State private var search = ""
    @FocusState private var searchFocused: Bool

    private var list: TrackList { model.list }
    private var drawerOpen: Bool { navigation.filterDrawer == .tracks }

    var body: some View {
        @Bindable var model = model
        // The size class is the window's, as Qt's (the drawer covers the page, never narrows it).
        let width = navigation.windowWidth
        let compact = width < 1100
        VStack(alignment: .leading, spacing: 0) {
            header(wide: width >= 1600, compact: compact)
                .padding(.leading, compact ? 20 : 28)
                .padding(.trailing, compact ? 20 : 28)
                .padding(.top, compact ? 16 : 22)
                .padding(.bottom, compact ? 10 : 14)
            content(compact: compact)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .overlay { scrim }
        .onAppear {
            search = list.search
            list.label = tr("tracks")
            list.activate()
            FilterCatalog.shared.refresh()
        }
        .onChange(of: search) { _, text in list.search = text }
        #if DEBUG
        .onAppear { TracksDebug.install() }
        .onChange(of: TracksDebugState.shared.search) { _, text in if let text { search = text } }
        #endif
        .onChange(of: Tr.shared.language) { list.label = tr("tracks") }
        // The drawer belongs to this page: it never shows over another one.
        .onChange(of: navigation.section) { _, section in
            if section != PageID.tracks && drawerOpen { navigation.filterDrawer = nil }
        }
        .onChange(of: store.sources) { FilterCatalog.shared.refresh() }
        .onChange(of: navigation.focusSearchRequest) {
            guard !drawerOpen, navigation.section == PageID.tracks else { return }
            searchFocused = true
        }
        .sheet(item: $model.smartPlaylist) { request in
            PromptDialog(
                title: tr("new_smart_playlist"),
                placeholder: tr("playlist_name"),
                confirmTitle: tr("create")
            ) { name in
                store.createPlaylist(
                    name: name,
                    rule: parseFilter(json: request.filter) ?? .everything,
                    sort: parseSort(sort: request.sort, playlist: nil)
                )
                return nil
            }
        }
    }

    // MARK: Header

    /// "1,036 tracks · 2 days 20 h", "12 of 1,036 tracks · 48 min"; empty until read.
    private var summary: String {
        guard list.ready else { return "" }
        let count = list.count == list.total
            ? Format.count(list.total, "track_one", "tracks_many")
            : tr("tracks_filtered", Format.number(list.count), Format.number(list.total))
        return list.count > 0 ? count + " · " + Format.duration(list.length) : count
    }

    @ViewBuilder
    private func header(wide: Bool, compact: Bool) -> some View {
        if compact {
            HStack(spacing: 10) {
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Text(tr("tracks"))
                        .font(.system(size: 22, weight: .bold))
                        .tracking(-0.44)
                        .foregroundStyle(Theme.text)
                    Text(list.ready ? Format.number(list.count) : "")
                        .font(.system(size: 13))
                        .monospacedDigit()
                        .foregroundStyle(Theme.text2)
                        .lineLimit(1)
                }
                .layoutPriority(1)
                Spacer(minLength: 8)
                TracksSearchField(text: $search, placeholder: tr("search"), height: 36, radius: 9, focused: $searchFocused)
                    .frame(minWidth: 160, idealWidth: 260, maxWidth: 260)
                TracksFilterIconButton(count: model.ruleCount) { navigation.openFilterDrawer(.tracks) }
                SortButton(sort: Binding(get: { list.sort }, set: { list.sort = $0 }), iconOnly: true)
                Button { list.playAll(shuffle: false) } label: {
                    Image(systemName: "play.fill")
                        .font(.system(size: 13, weight: .bold))
                        .foregroundStyle(Theme.accentInk)
                        .frame(width: 36, height: 36)
                        .background(Circle().fill(Theme.accent))
                        .contentShape(Circle())
                }
                .buttonStyle(PressScaleStyle())
                .disabled(list.count == 0)
                .opacity(list.count == 0 ? 0.45 : 1)
                .help(tr("play_all"))
                .accessibilityLabel(tr("play_all"))
            }
        } else if wide {
            HStack(spacing: 16) {
                titleBlock
                    .padding(.trailing, 8)
                tools(searchWidth: 380)
                pills
            }
        } else {
            VStack(alignment: .leading, spacing: 14) {
                HStack(alignment: .bottom, spacing: 16) {
                    titleBlock
                    Spacer(minLength: 0)
                    pills
                }
                tools(searchWidth: 360)
            }
        }
    }

    /// The search, the chips and Filter pill (filling the room left, wrapping), and the sort
    /// button, all top-aligned (`qml/TracksPage.qml:265-360`).
    private func tools(searchWidth: CGFloat) -> some View {
        HStack(alignment: .top, spacing: 8) {
            TracksSearchField(text: $search, placeholder: tr("search_tracks"), focused: $searchFocused)
                .frame(minWidth: 200, idealWidth: searchWidth, maxWidth: searchWidth)
                // It takes its width first; the chips get the rest (Qt's RowLayout).
                .layoutPriority(1)
            chipsAndFilter
                .frame(maxWidth: .infinity, alignment: .topLeading)
            SortButton(sort: Binding(get: { list.sort }, set: { list.sort = $0 }))
                .padding(.top, 3)
        }
    }

    private var titleBlock: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(tr("tracks"))
                .font(.system(size: 26, weight: .bold))
                .tracking(-0.52)
                .foregroundStyle(Theme.text)
            Text(summary)
                .font(.system(size: 13))
                .monospacedDigit()
                .foregroundStyle(Theme.text2)
                .lineLimit(1)
                .frame(minHeight: 16)
        }
        .fixedSize()
    }

    private var pills: some View {
        HStack(spacing: 8) {
            Button { list.playAll(shuffle: false) } label: {
                Label(tr("play"), systemImage: "play.fill")
            }
            .buttonStyle(PillButtonStyle(kind: .primary))
            Button { list.playAll(shuffle: true) } label: {
                Label(tr("shuffle"), systemImage: "shuffle")
            }
            .buttonStyle(PillButtonStyle(kind: .plain))
        }
        .disabled(list.count == 0)
        .fixedSize()
    }

    /// The applied rules as chips, then the dashed Filter pill; they wrap onto more lines.
    private var chipsAndFilter: some View {
        TracksChipFlow(spacing: 8) {
            ForEach(model.chips, id: \.index) { chip in
                TracksFilterChip(name: chip.name, text: chip.text) {
                    navigation.openFilterDrawer(.tracks)
                } remove: {
                    model.removeRule(at: chip.index)
                }
            }
            TracksFilterButton { navigation.openFilterDrawer(.tracks) }
        }
        .padding(.top, 3)
    }

    // MARK: Body

    @ViewBuilder
    private func content(compact: Bool) -> some View {
        ZStack {
            if list.ready {
                if list.count > 0 {
                    TrackTable(list: list, layout: .library)
                } else if list.total == 0 && store.scan != nil {
                    EmptyStateView(symbol: "arrow.clockwise", title: tr("scanning_library"))
                } else if list.total == 0 {
                    EmptyStateView(
                        symbol: "folder", title: tr("empty_library"), message: tr("empty_library_hint"),
                        buttonTitle: tr("open_sources")
                    ) { navigation.go(PageID.sources) }
                } else {
                    EmptyStateView(symbol: "magnifyingglass", title: tr("no_results"))
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    /// Dims the page under the filter drawer; a click on it closes the drawer without applying.
    private var scrim: some View {
        Theme.bg
            .opacity(drawerOpen ? 0.7 : 0)
            .allowsHitTesting(drawerOpen)
            .onTapGesture { navigation.filterDrawer = nil }
            .animation(Motion.standard(Motion.fade), value: drawerOpen)
            .accessibilityHidden(true)
    }
}
