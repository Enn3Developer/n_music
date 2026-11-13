import SwiftUI

/// What the filter drawer edits.
enum FilterDrawerContext: Equatable, Sendable {
    /// The Tracks page's filter and order (`TracksModel.shared.list`).
    case tracks
    /// A smart playlist's rules and order ("Edit rules"): Apply saves them to the playlist.
    case playlistRules(PlaylistId)
}

/// The filter drawer and rule editor (spec §3.7), which the shell lays over the page area's right
/// edge while `Navigation.shared.filterDrawer` is set. It works on a copy: closing (×, Escape, the page's
/// scrim) never applies. Opening without rules starts with one empty "Artist is ___" rule.
///
/// Body: "Tracks matching [all/any] of these rules", the rules and groups, "+ Add rule",
/// "+ Add group", then "Sort by" with a row per key and "+ Then by…". Footer: the live count of
/// matches (with the page's search), Save as smart playlist (Tracks only) and Apply.
struct FilterDrawer: View {
    let context: FilterDrawerContext

    private let store = Store.shared
    private let navigation = Navigation.shared

    @State private var spec = FilterSpec()
    /// The order being written.
    @State private var keys: [SortKeySpec] = []
    /// The order started from: kept when no key shows (an order the editor can't show).
    @State private var startSort = ""
    /// What the matches must also pass: the Tracks page's search.
    @State private var search = ""
    @State private var matches: Int?
    @State private var loaded: FilterDrawerContext?
    @State private var counting: Task<Void, Never>?
    /// Focus moves into the drawer as it opens, so Escape closes it and Return no longer
    /// reaches the page under it (`qml/FilterDrawer.qml:35-44, 122`).
    @FocusState private var focused: Bool

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(title)
                    .font(.system(size: 19, weight: .bold))
                    .foregroundStyle(Theme.text)
                Spacer()
                IconButton(symbol: "xmark", label: tr("close"), size: 36, iconSize: 16) { close() }
            }
            // The close button at 18 from the top, the body at 66 (`qml/FilterDrawer.qml`).
            .padding(.horizontal, 20)
            .padding(.top, 18)
            .padding(.bottom, 12)

            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    matchLine
                    ForEach($spec.entries) { $entry in
                        entryView($entry)
                    }
                    HStack(spacing: 6) {
                        RuleDashedButton(title: tr("add_rule")) { spec.entries.append(.rule(Filters.newRule())) }
                        RuleDashedButton(title: tr("add_group")) { spec.entries.append(.group(Filters.newGroup())) }
                    }
                    .padding(.top, 2)
                    sortSection
                }
                .padding(.horizontal, 20)
                .padding(.top, 4)
                .padding(.bottom, 20)
            }
            .scrollIndicators(.automatic)

            Rectangle().fill(Theme.line2).frame(height: 1)
            footer
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Theme.surface)
        // Takes keys whatever the keyboard navigation setting (Escape closes it).
        .focusable(interactions: .edit)
        .focusEffectDisabled()
        .focused($focused)
        .onExitCommand { close() }
        .onKeyPress(.escape) {
            close()
            return .handled
        }
        .task(id: context) { await load() }
        .onAppear { focused = true }
        .onChange(of: context) { focused = true }
        .onChange(of: spec) { count() }
        // It belongs to its page: leaving the page closes it without applying.
        .onChange(of: navigation.page) { _, page in
            switch context {
            case .tracks:
                if PageID.section(of: page) != PageID.tracks { close() }
            case .playlistRules(let id):
                if page != PageID.playlist(id) { close() }
            }
        }
    }

    private var title: String {
        switch context {
        case .tracks: tr("filter_tracks")
        case .playlistRules: tr("edit_rules")
        }
    }

    // MARK: Rules

    private var matchLine: some View {
        HStack(spacing: 8) {
            Text(tr("matching_before"))
                .font(.system(size: 14))
                .foregroundStyle(Theme.text2)
            RuleSelectBox(
                options: Filters.matchValues.map { RuleSelectOption(value: $0, label: tr("match_" + $0)) },
                value: $spec.match,
                height: 32,
                weight: .semibold,
                label: tr("combine_rules")
            )
            .fixedSize()
            Text(tr("matching_after"))
                .font(.system(size: 14))
                .foregroundStyle(Theme.text2)
        }
        .frame(height: 32)
    }

    @ViewBuilder
    private func entryView(_ entry: Binding<FilterEntry>) -> some View {
        let id = entry.wrappedValue.id
        switch entry.wrappedValue {
        case .rule:
            RuleRowView(rule: Binding(
                get: { if case .rule(let rule) = entry.wrappedValue { rule } else { Filters.newRule() } },
                set: { entry.wrappedValue = .rule($0) }
            )) {
                spec.entries.removeAll { $0.id == id }
            }
        case .group:
            RuleGroupView(group: Binding(
                get: { if case .group(let group) = entry.wrappedValue { group } else { Filters.newGroup() } },
                set: { entry.wrappedValue = .group($0) }
            )) {
                spec.entries.removeAll { $0.id == id }
            }
        }
    }

    // MARK: Sort

    private var sortSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Rectangle().fill(Theme.line2).frame(height: 1)
                .padding(.top, 10)
                .padding(.bottom, 8)
            Text(tr("sort_by"))
                .font(.system(size: 14, weight: .bold))
                .foregroundStyle(Theme.text)
            ForEach(Array(keys.enumerated()), id: \.element.id) { index, key in
                HStack(spacing: 8) {
                    RuleSelectBox(
                        options: Filters.sortFields.map { RuleSelectOption(value: $0.field, label: tr($0.labelKey)) },
                        value: Binding(get: { key.field }, set: { field in update(key.id) { $0.field = field } }),
                        label: tr(index == 0 ? "sort_field" : "then_by")
                    )
                    .frame(maxWidth: .infinity)
                    RuleSelectBox(
                        options: [false, true].map {
                            RuleSelectOption(value: $0 ? "down" : "up", label: Filters.directionLabel(key.field, descending: $0))
                        },
                        value: Binding(get: { key.descending ? "down" : "up" },
                                       set: { direction in update(key.id) { $0.descending = direction == "down" } }),
                        muted: true,
                        label: tr("sort_direction")
                    )
                    .frame(width: 132)
                    RuleRemoveButton(label: tr("remove_sort_key")) {
                        keys.removeAll { $0.id == key.id }
                    }
                    .disabled(keys.count <= 1)
                }
                .frame(height: 36)
            }
            RuleFlatButton(title: tr("then_by")) { addKey() }
                .disabled(keys.count >= Filters.sortFields.count)
        }
    }

    private func update(_ id: UUID, _ change: (inout SortKeySpec) -> Void) {
        guard let index = keys.firstIndex(where: { $0.id == id }) else { return }
        change(&keys[index])
    }

    /// The first field not used yet, ascending.
    private func addKey() {
        guard let field = Filters.sortFields.first(where: { field in !keys.contains { $0.field == field.field } }) else { return }
        keys.append(SortKeySpec(field: field.field, descending: false))
    }

    /// The order written; the one started from while no key shows.
    private var writtenSort: String {
        keys.isEmpty ? startSort : Filters.sortString(keys)
    }

    // MARK: Footer

    private var footer: some View {
        HStack(spacing: 8) {
            if let matches {
                HStack(spacing: 4) {
                    Text(Format.number(matches))
                        .font(.system(size: 14, weight: .bold))
                        .monospacedDigit()
                        .foregroundStyle(Theme.text)
                    Text(tr(matches == 1 ? "match_one" : "match_many"))
                        .font(.system(size: 14))
                        .foregroundStyle(Theme.text2)
                }
                .lineLimit(1)
            }
            Spacer(minLength: 8)
            if context == .tracks {
                Button(tr("save_smart_playlist")) { save() }
                    .buttonStyle(DrawerPillStyle(primary: false))
                    .lineLimit(1)
                    .fixedSize()
            }
            Button(tr("apply")) { apply() }
                .buttonStyle(DrawerPillStyle(primary: true))
                .fixedSize()
        }
        .padding(.horizontal, 20)
        .frame(height: 67)
    }

    // MARK: Opening, counting, applying

    private func load() async {
        FilterCatalog.shared.refresh()
        var filter = ""
        var sort = ""
        switch context {
        case .tracks:
            let list = TracksModel.shared.list
            filter = list.filter
            sort = list.sort
            search = list.search
        case .playlistRules(let id):
            search = ""
            guard let playlist = await store.playlist(id) else { return }
            filter = playlist.rule.flatMap { filterSpec(filter: $0) } ?? ""
            sort = playlist.sort.isEmpty ? (playlist.isSmart ? "artist,album" : "-added") : sortString(sort: playlist.sort)
        }
        var applied = Filters.parse(filter)
        if applied.entries.isEmpty {
            applied.entries.append(.rule(Filters.newRule()))
        }
        matches = nil
        spec = applied
        keys = Filters.parseSort(sort)
        startSort = sort
        loaded = context
        count()
    }

    /// Counts the matches of the rules as they are; only the latest count shows.
    private func count() {
        let query = tracksQuery(search: search, filter: Filters.json(Filters.clean(spec)), sort: "", playlist: nil)
        counting?.cancel()
        counting = Task {
            let count = await store.count(query)
            guard !Task.isCancelled else { return }
            matches = Int(count)
        }
    }

    private func close() {
        navigation.filterDrawer = nil
    }

    private func apply() {
        let filter = Filters.json(Filters.clean(spec))
        let sort = writtenSort
        switch context {
        case .tracks:
            let list = TracksModel.shared.list
            list.filter = filter
            list.sort = sort
        case .playlistRules(let id):
            store.setPlaylistRule(id, rule: parseFilter(json: filter) ?? .everything)
            store.setPlaylistSort(id, sort: parseSort(sort: sort, playlist: id))
        }
        close()
    }

    /// Saving doesn't apply the rules to Tracks: the page asks for a name, and the playlist
    /// opens once made.
    private func save() {
        let request = SmartPlaylistRequest(filter: Filters.json(Filters.clean(spec)), sort: writtenSort)
        close()
        TracksModel.shared.smartPlaylist = request
    }
}

/// The footer's pills (`qml/FilterDrawer.qml` footer): 38 points, 14-point semibold, padding 14
/// (Save as smart playlist) or 18 (Apply, the accent), as `PillButtonStyle` otherwise.
private struct DrawerPillStyle: ButtonStyle {
    let primary: Bool
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 14, weight: .semibold))
            .padding(.horizontal, primary ? 18 : 14)
            .frame(height: 38)
            .foregroundStyle(primary ? Theme.accentInk : Theme.text)
            .background(Capsule().fill(fill(pressed: configuration.isPressed)))
            .overlay(Capsule().strokeBorder(primary ? Color.clear : Theme.border, lineWidth: primary ? 0 : 1))
            .contentShape(Capsule())
            .scaleEffect(configuration.isPressed ? 0.95 : 1)
            .onHover { hovering = $0 }
            .animation(Motion.standard(Motion.move), value: configuration.isPressed)
    }

    private func fill(pressed: Bool) -> Color {
        if primary {
            let accent = Theme.accent.opacity(pressed ? 0.85 : 1)
            guard hovering && !pressed else { return accent }
            // Lightened toward white by 8 % under the pointer, as `PillButtonStyle`.
            return Color(nsColor: NSColor(accent).blended(withFraction: 0.08, of: .white) ?? NSColor(accent))
        }
        return pressed ? Theme.selected : hovering ? Theme.hover : .clear
    }
}
