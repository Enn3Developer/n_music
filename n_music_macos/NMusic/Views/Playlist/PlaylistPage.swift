import SwiftUI

/// A regular or smart playlist's page (spec §3.6, `qml/PlaylistPage.qml`): the art (a cover
/// or mosaic; a filter tile for smart ones), the eyebrow, the name (with a rename pencil for
/// regular ones), a smart playlist's rule chips, the summary, Play / Shuffle / "more" / sort,
/// then the shared track table or the empty state. The sort is saved per playlist. The page
/// stays alive: `id` changes when another playlist opens.
struct PlaylistPage: View {
    let id: PlaylistId

    private let store = Store.shared
    private let navigation = Navigation.shared
    /// The playlist as last read; `nil` until read, or after it was deleted.
    @State private var playlist: PlaylistRow?
    @State private var list = TrackList(sort: "")
    /// The saved order the list's sort was last set from: a newer saved one replaces it.
    @State private var appliedSort: String?
    /// Qt's narrow header (see `groupsNarrowHeader(_:)`).
    @State private var narrow = false
    @State private var renaming = false
    @State private var deleting = false

    var body: some View {
        // Until another playlist opened here is read, the one before stays laid out (Qt's page
        // never blanks while it reloads), its controls off.
        let playlist = self.playlist
        let stale = playlist != nil && playlist?.id != id
        VStack(spacing: 0) {
            if let playlist {
                Group {
                    header(playlist)
                    if list.ready && list.count > 0 && list.playlist == playlist.id {
                        TrackTable(
                            list: list,
                            layout: playlist.isSmart ? .smart : .playlist,
                            playlist: playlist.isSmart ? nil : (playlist.id, playlist.name)
                        )
                    } else if list.ready && list.playlist == playlist.id {
                        emptyState(playlist)
                            .offset(y: 20)
                    } else {
                        Spacer()
                    }
                }
                .disabled(stale)
            } else {
                Spacer()
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .groupsNarrowHeader($narrow)
        .overlay { scrim }
        .task(id: RowKey(id: id, version: store.playlistsVersion)) {
            let id = self.id
            let row = await store.playlist(id)
            guard !Task.isCancelled else { return }
            apply(row, id: id)
        }
        #if DEBUG
        .onAppear { GroupsDebug.shared.install() }
        .onChange(of: GroupsDebug.shared.command?.serial) {
            guard let command = GroupsDebug.shared.command, Navigation.shared.section == "playlist" else { return }
            switch command.verb {
            case "track-sort": list.sort = command.argument
            case "rename": renaming = true
            case "delete": deleting = true
            case "close": renaming = false; deleting = false
            default: break
            }
        }
        #endif
        .onChange(of: navigation.page) {
            // The rules drawer belongs to its playlist's page (Qt's drawer is the page's own):
            // leaving the page closes it without applying.
            if case .playlistRules(let shown) = navigation.filterDrawer, navigation.page != PageID.playlist(shown) {
                navigation.filterDrawer = nil
            }
        }
        .onChange(of: id) {
            // Another playlist opened in this page: its own saved order applies.
            appliedSort = nil
        }
        .onChange(of: list.sort) { _, sort in
            // The sort is saved per playlist (`qml/PlaylistPage.qml:49-52`).
            guard let playlist = self.playlist, playlist.id == id, list.playlist == id,
                  !sort.isEmpty, sort != appliedSort
            else { return }
            appliedSort = sort
            store.setPlaylistSort(playlist.id, sort: parseSort(sort: sort, playlist: playlist.id))
        }
        .sheet(isPresented: $renaming) {
            PromptDialog(
                title: tr("rename_playlist"),
                text: playlist?.name ?? "",
                placeholder: tr("playlist_name"),
                confirmTitle: tr("rename")
            ) { name in
                store.renamePlaylist(id, to: name)
                return nil
            }
        }
        .sheet(isPresented: $deleting) {
            PromptDialog(
                title: tr("delete_playlist_title", playlist?.name ?? ""),
                message: tr("delete_playlist_message"),
                text: nil,
                confirmTitle: tr("delete_playlist"),
                danger: true
            ) { _ in
                store.deletePlaylist(id)
                navigation.go(PageID.tracks)
                return nil
            }
        }
    }

    /// Shows `row`, the playlist `id` as just read: its saved order, name and tracks.
    private func apply(_ row: PlaylistRow?, id: PlaylistId) {
        guard let row else {
            playlist = nil
            return
        }
        playlist = row
        let saved = savedSort(row)
        if appliedSort != saved || list.playlist != id {
            appliedSort = saved
            list.sort = saved
        }
        list.label = row.name
        list.origin = PageID.playlist(id)
        list.detail = ""
        if list.playlist != id { list.playlist = id }
        list.activate()
        if row.isSmart { FilterCatalog.shared.refresh() }
    }

    // MARK: Header (`qml/PlaylistPage.qml:65-296`)

    private func header(_ playlist: PlaylistRow) -> some View {
        @Bindable var list = list
        let smart = playlist.isSmart
        let shown = list.playlist == playlist.id
        let narrow = self.narrow
        let radius: CGFloat = narrow ? 8 : 10
        return HStack(alignment: .bottom, spacing: narrow ? 16 : 24) {
            if smart {
                RoundedRectangle(cornerRadius: radius, style: .continuous)
                    .fill(Theme.field)
                    .overlay(RoundedRectangle(cornerRadius: radius, style: .continuous).strokeBorder(Theme.track, lineWidth: 1))
                    .overlay(
                        Image(systemName: "line.3.horizontal.decrease")
                            .font(.system(size: narrow ? 32 : 52, weight: .light))
                            .foregroundStyle(Theme.accentText)
                    )
                    .frame(width: narrow ? 96 : 168, height: narrow ? 96 : 168)
            } else {
                GroupsArt(
                    path: shown ? list.covers.first : nil,
                    mosaic: shown ? list.covers : [],
                    size: narrow ? 96 : 168,
                    radius: radius,
                    placeholder: .symbol("music.note.list")
                )
            }
            VStack(alignment: .leading, spacing: 8) {
                if !narrow {
                    Text(tr(smart ? "smart_playlist" : "playlist").uppercased())
                        .font(.system(size: 12, weight: .semibold))
                        .tracking(0.96)
                        .foregroundStyle(Theme.text2)
                }
                HStack(spacing: 8) {
                    Text(playlist.name)
                        .font(.system(size: narrow ? 22 : 34, weight: .bold))
                        .tracking(narrow ? -0.44 : -0.68)
                        .foregroundStyle(Theme.text)
                        .lineLimit(1)
                        .truncationMode(.tail)
                        .accessibilityAddTraits(.isHeader)
                    if !smart {
                        IconButton(symbol: "pencil", label: tr("rename_playlist"), size: narrow ? 28 : 34, iconSize: 16, tint: Theme.text3) {
                            renaming = true
                        }
                    }
                }
                if smart { ruleChips(playlist) }
                Text(list.ready && shown ? summary(playlist) : " ")
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
                    .disabled(list.count == 0 || !shown)
                    // Text only, without its icon (`qml/PlaylistPage.qml:235-241`).
                    // Icon only when narrow.
                    Button { list.playAll(shuffle: true) } label: {
                        if narrow { Image(systemName: "shuffle") } else { Text(tr("shuffle")) }
                    }
                        .buttonStyle(PillButtonStyle(kind: .plain))
                        .fixedSize()
                        .help(tr("shuffle"))
                        .accessibilityLabel(tr("shuffle"))
                        .disabled(list.count == 0 || !shown)
                    moreMenu(playlist)
                    Spacer(minLength: 0)
                    GroupsSortButton(sort: $list.sort, options: smart ? SortOption.tracks : SortOption.playlist, narrow: narrow)
                }
                .padding(.top, 6)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, narrow ? 16 : 28)
        .padding(.top, narrow ? 16 : 28)
        .padding(.bottom, narrow ? 12 : 18)
    }

    /// The rules as read-only chips joined by "and"/"or", or why there are none, then the
    /// "Edit rules" link (`qml/PlaylistPage.qml:146-212`).
    @ViewBuilder private func ruleChips(_ playlist: PlaylistRow) -> some View {
        let spec = editorRule(playlist)
        let rules = Filters.parse(spec ?? "")
        let joiner = tr(rules.match == "any" ? "rules_or" : "rules_and")
        let count = rules.entries.count
        PlaylistFlow(spacing: 6) {
            // Each chip and its "and"/"or" wrap together (Qt's Row per rule), so a joiner never
            // starts a line; a rule the editor can't describe shows nothing.
            ForEach(Array(rules.entries.enumerated()), id: \.offset) { index, entry in
                if let words = Filters.describe(entry) {
                    HStack(spacing: 6) {
                        PlaylistRuleChip(name: words.name, detail: words.text, action: openRules)
                        if index < count - 1 {
                            Text(joiner)
                                .font(.system(size: 12))
                                .foregroundStyle(Theme.text3)
                        }
                    }
                }
            }
            if spec == nil || rules.entries.isEmpty {
                Text(tr(spec == nil ? "rules_not_editable" : "rules_every_track"))
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.text3)
                    .frame(height: 28)
            }
            if spec != nil {
                PlaylistEditRulesLink(action: openRules)
            }
        }
    }

    /// "More" (`qml/PlaylistPage.qml:253-275`): Rename playlist, Edit rules (smart ones only;
    /// disabled when the editor can't show them), Delete playlist (danger, no separator).
    private func moreMenu(_ playlist: PlaylistRow) -> some View {
        let smart = playlist.isSmart
        let editable = editorRule(playlist) != nil
        let rename = { renaming = true }
        let delete = { deleting = true }
        let rules = openRules
        return GroupsMoreButton(label: tr("playlist_actions"), iconSize: 16, tint: Theme.text) {
            var entries: [SourceMenuEntry] = [.item(title: tr("rename_playlist"), symbol: "pencil", action: rename)]
            if smart {
                entries.append(.item(title: tr("edit_rules"), symbol: "line.3.horizontal.decrease", enabled: editable, action: rules))
            }
            entries.append(.item(title: tr("delete_playlist"), symbol: "trash", danger: true, action: delete))
            return entries
        }
    }

    // MARK: Empty states (spec §9.1)

    @ViewBuilder private func emptyState(_ playlist: PlaylistRow) -> some View {
        if playlist.isSmart {
            let editable = editorRule(playlist) != nil
            let edit: (() -> Void)? = editable ? { openRules() } : nil
            EmptyStateView(
                symbol: "line.3.horizontal.decrease",
                title: tr("smart_playlist_empty"),
                message: tr("smart_playlist_empty_hint"),
                buttonTitle: editable ? tr("edit_rules") : nil,
                action: edit
            )
        } else {
            EmptyStateView(
                symbol: "music.note.list",
                title: tr("playlist_empty"),
                message: tr("playlist_empty_hint"),
                buttonTitle: tr("open_tracks")
            ) { navigation.go(PageID.tracks) }
        }
    }

    // MARK: The rules drawer

    private func openRules() {
        navigation.openFilterDrawer(.playlistRules(id))
    }

    /// Dims the page under the rules drawer; a click on it closes the drawer without applying.
    private var scrim: some View {
        let shown = navigation.filterDrawer == .playlistRules(id)
        return Theme.bg
            .opacity(shown ? 0.7 : 0)
            .allowsHitTesting(shown)
            .contentShape(Rectangle())
            .onTapGesture { navigation.filterDrawer = nil }
            .animation(Motion.standard(Motion.fade), value: shown)
            .accessibilityHidden(true)
    }

    // MARK: Data

    /// The rule as the editor writes it; `nil` when the editor can't show it ("These rules
    /// can't be edited here").
    private func editorRule(_ playlist: PlaylistRow) -> String? {
        playlist.rule.flatMap { filterSpec(filter: $0) }
    }

    /// The order the playlist was left in, or its kind's usual one.
    private func savedSort(_ playlist: PlaylistRow) -> String {
        let saved = sortString(sort: playlist.sort)
        return !saved.isEmpty ? saved : playlist.isSmart ? "artist,album" : "-added"
    }

    private func summary(_ playlist: PlaylistRow) -> String {
        var parts = [Format.count(list.count, "track_one", "tracks_many")]
        if list.count > 0 { parts.append(Format.duration(list.length)) }
        if playlist.isSmart {
            parts.append(tr("updates_as_you_listen"))
        } else if playlist.modified > 0 {
            let seconds = Double(playlist.modified)
            let ago = Format.ago(seconds)
            // Qt's `toLocaleLowerCase()`: in the language shown.
            parts.append(tr("changed_ago", Format.daysSince(seconds) < 35 ? ago.lowercased(with: Locale(identifier: Tr.shared.language ?? Tr.shared.systemCode)) : ago))
        }
        return parts.joined(separator: " · ")
    }

    private struct RowKey: Equatable {
        var id: PlaylistId
        var version: Int
    }
}

/// The underlined "Edit rules" link: `text2`, `text` with a `hover` pill on hover.
private struct PlaylistEditRulesLink: View {
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Text(tr("edit_rules"))
                .font(.system(size: 13))
                .underline()
                .foregroundStyle(hovering ? Theme.text : Theme.text2)
                .padding(.horizontal, 10)
                .frame(height: 28)
                .background(Capsule().fill(hovering ? Theme.hover : .clear))
                .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .animation(Motion.standard(Motion.fade), value: hovering)
    }
}
