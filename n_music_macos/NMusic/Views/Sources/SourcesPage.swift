import SwiftUI

/// The Sources page (Qt `SourcesPage.qml`, spec §5.2): the library's sources as cards by kind.
/// The header has the title and count, a search (name or location), the sort (Name / Most
/// tracks), Library actions (Update library, Reload all metadata, the last complete update)
/// and Add source; the SourcesScanCard shows under it while a scan runs. Search and sort are kept for
/// the session (the page stays alive), not saved.
struct SourcesPage: View {
    private let store = Store.shared
    private let navigation = Navigation.shared

    /// `nil` until the first answer: nothing shows before (spec §9.2).
    @State private var sources: [SourceInfo]?
    @State private var search = ""
    @State private var sort = SourceSort.name
    @State private var width: CGFloat = 800
    @State private var renaming: SourceInfo?
    @State private var removing: SourceInfo?
    @State private var addingWeb = false
    @State private var addingTelegram = false
    /// Goes up to open the Add source menu (the empty state's button).
    @State private var openKinds = 0
    @State private var sortAnchor = SourceMenuAnchor()
    @State private var sortOpen = false
    @State private var actionsAnchor = SourceMenuAnchor()

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            if let scan = store.scan {
                SourcesScanCard(scan: scan)
                    .padding(.horizontal, 28)
                    .padding(.bottom, 20)
            }
            content
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        // Read again on what changes it, only while the page shows (and once it shows again):
        // `source_info` walks every track, and the version ticks every second while scanning.
        .task(id: RefreshKey(
            version: store.libraryVersion,
            roots: store.roots,
            scanning: store.scan?.libraries,
            signedIn: { if case .signedIn = store.telegram?.status { true } else { false } }(),
            shown: shown
        )) {
            guard shown || sources == nil else { return }
            let list = await store.sourceList()
            guard !Task.isCancelled else { return }
            if sources != list { sources = list }
        }
        .sourceAddSheets(web: $addingWeb, telegram: $addingTelegram, taken: taken) { root, name in
            Task { await store.addSource(root, name: name) }
        }
        .sourceDialogs(renaming: $renaming, removing: $removing)
        .onAppear(perform: addRequested)
        .onChange(of: navigation.addSourceRequest) { addRequested() }
        #if DEBUG
        .onAppear { SourcesDebug.install() }
        .onChange(of: SourcesDebug.shared.request) { _, _ in debugRequest() }
        #endif
    }

    /// The page is the one showing (hidden pages stay alive in the shell).
    private var shown: Bool { navigation.section == PageID.sources }

    // MARK: Header

    private var header: some View {
        HStack(alignment: .bottom, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text(tr("sources"))
                    .font(.system(size: 26, weight: .bold))
                    .tracking(-0.52)
                    .foregroundStyle(Theme.text)
                Text(sources.map { Format.count($0.count, "sources_one", "sources_many") } ?? " ")
                    .font(.system(size: 13).monospacedDigit())
                    .foregroundStyle(Theme.text2)
            }
            .accessibilityElement(children: .combine)
            .accessibilityAddTraits(.isHeader)
            Spacer(minLength: 0)
            SearchBox(text: $search, placeholder: tr("search_sources"), styled: true)
                .frame(minWidth: 160, idealWidth: 260, maxWidth: 260)
                .frame(height: 38)
            sortButton
            actionsButton
            AddSourceButton(height: 38, request: openKinds, choose: add)
        }
        .padding(.leading, 28)
        .padding(.trailing, 28)
        .padding(.top, 22)
        .padding(.bottom, 16)
    }

    private var sortButton: some View {
        Button(action: openSortMenu) {
            HStack(spacing: 6) {
                Text(sort.label)
                    .font(.system(size: 13, weight: .medium))
                Image(systemName: "chevron.down")
                    .font(.system(size: 11, weight: .semibold))
            }
            .foregroundStyle(Theme.text2)
            .padding(.horizontal, 12)
            .frame(height: 38)
            .contentShape(Rectangle())
        }
        .buttonStyle(SourceMenuButtonStyle(open: sortOpen))
        .background(SourceMenuAnchorView(anchor: sortAnchor))
        .accessibilityLabel(tr("sort_by") + ": " + sort.label)
    }

    private func openSortMenu() {
        sortOpen = true
        SourceMenus.popUp(SourceSort.allCases.map { order in
            .item(title: order.label, symbol: nil, checked: order == sort) { sort = order }
        }, under: sortAnchor.view) { sortOpen = false }
    }

    /// The round outlined "⋯": what updates every source. Disabled without sources; its tooltip
    /// is the one Qt shows.
    private var actionsButton: some View {
        Button {
            SourceMenus.popUp(libraryActions, under: actionsAnchor.view)
        } label: {
            Image(systemName: "ellipsis")
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(Theme.text2)
                .frame(width: 38, height: 38)
                .overlay(Circle().strokeBorder(Theme.border, lineWidth: 1))
                .contentShape(Circle())
        }
        .buttonStyle(IconButtonStyle(radius: 19))
        .background(SourceMenuAnchorView(anchor: actionsAnchor))
        .disabled(sources?.isEmpty ?? true)
        .help(tr("library_actions"))
        .accessibilityLabel(tr("library_actions"))
    }

    private var libraryActions: [SourceMenuEntry] {
        let scanning = store.scan != nil
        var entries: [SourceMenuEntry] = [
            .item(title: tr("update_library"), symbol: "arrow.clockwise", enabled: !scanning) {
                store.scan(nil, checkCache: true)
            },
            .item(title: tr("reload_metadata_title"), symbol: "tag", enabled: !scanning) {
                store.scan(nil, checkCache: false)
            },
        ]
        if let updated = store.lastCompleteScan {
            entries.append(.separator)
            entries.append(.note(tr("last_update", Self.when(updated))))
        }
        return entries
    }

    /// When the last complete update finished: "today, 17:38", "yesterday, 17:38", else the
    /// locale's short date and time.
    static func when(_ date: Date) -> String {
        let time = date.formatted(date: .omitted, time: .shortened)
        switch Format.daysSince(date.timeIntervalSince1970) {
        case 0: return tr("today_at", time)
        case 1: return tr("yesterday_at", time)
        default: return date.formatted(date: .numeric, time: .shortened)
        }
    }

    // MARK: Cards

    private var gap: CGFloat { 20 }
    private var columns: Int { max(1, Int((width + gap) / 188)) }
    private var cardWidth: CGFloat { max(1, ((width - CGFloat(columns - 1) * gap) / CGFloat(columns)).rounded(.down)) }

    @ViewBuilder
    private var content: some View {
        let sections = self.sections
        if !sections.isEmpty {
            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    ForEach(sections, id: \.kind) { section in
                        VStack(alignment: .leading, spacing: 14) {
                            HStack(spacing: 8) {
                                Text(section.kind.group.uppercased())
                                    .font(.system(size: 12, weight: .semibold))
                                    .tracking(0.96)
                                    .foregroundStyle(Theme.text2)
                                Text(Format.number(section.sources.count))
                                    .font(.system(size: 12, weight: .semibold).monospacedDigit())
                                    .foregroundStyle(Theme.text3)
                            }
                            .accessibilityElement(children: .combine)
                            .accessibilityAddTraits(.isHeader)
                            LazyVGrid(
                                columns: Array(repeating: GridItem(.fixed(cardWidth), spacing: gap, alignment: .topLeading), count: columns),
                                alignment: .leading,
                                spacing: 24
                            ) {
                                ForEach(section.sources, id: \.root.key) { source in
                                    SourceCard(
                                        source: source,
                                        width: cardWidth,
                                        shown: shown,
                                        rename: { renaming = source },
                                        remove: { removing = source }
                                    )
                                }
                            }
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { width = $0 }
                .padding(.horizontal, 28)
                .padding(.bottom, 24)
            }
            .accessibilityLabel(tr("sources"))
        } else if let sources {
            if sources.isEmpty {
                EmptyStateView(
                    symbol: "folder",
                    title: tr("no_sources_title"),
                    message: tr("no_sources_hint"),
                    buttonTitle: tr("add_source")
                ) { openKinds += 1 }
            } else {
                EmptyStateView(symbol: "magnifyingglass", title: tr("no_results"))
            }
        } else {
            Color.clear
        }
    }

    /// The sources matching the search, sorted, by kind in `SourceKind` order; only kinds with
    /// some.
    private var sections: [SourceSection] {
        guard let sources else { return [] }
        let text = search.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        var found = text.isEmpty ? sources : sources.filter {
            $0.name.lowercased().contains(text) || $0.location.lowercased().contains(text)
        }
        // As Qt's `localeCompare`: the locale's collation, numbers compared as text.
        let byName = { (a: SourceInfo, b: SourceInfo) in a.name.localizedCompare(b.name) == .orderedAscending }
        switch sort {
        case .name: found.sort(by: byName)
        case .mostTracks: found.sort { $0.tracks != $1.tracks ? $0.tracks > $1.tracks : byName($0, $1) }
        }
        return SourceKind.allCases.compactMap { kind in
            let matching = found.filter { SourceKind($0.kind) == kind }
            return matching.isEmpty ? nil : SourceSection(kind: kind, sources: matching)
        }
    }

    private var taken: Set<Locator> { Set(store.roots ?? []) }

    // MARK: Adding

    /// The requests answered (File › Add Source…): the page may be made after one was asked.
    @MainActor private static var answeredRequest = 0

    private func addRequested() {
        guard navigation.addSourceRequest > Self.answeredRequest else { return }
        Self.answeredRequest = navigation.addSourceRequest
        openKinds += 1
    }

    private func add(_ kind: SourceKind) {
        switch kind {
        case .folder:
            FolderPanel.choose(.sources, taken: taken) { root, name in
                Task { await store.addSource(root, name: name) }
            }
        case .web: addingWeb = true
        case .telegram: addingTelegram = true
        case .spotify, .youtube, .deezer: break
        }
    }

    #if DEBUG
    private func debugRequest() {
        guard navigation.section == PageID.sources, let command = SourcesDebug.shared.command else { return }
        let index = Int(command.argument) ?? 0
        let all = sections.flatMap(\.sources)
        switch command.name {
        case "close":
            renaming = nil
            removing = nil
            addingWeb = false
            addingTelegram = false
        case "web": addingWeb = true
        case "telegram": addingTelegram = true
        case "folder": add(.folder)
        case "kinds": openKinds += 1
        case "reload": store.scan(nil, checkCache: false)
        case "search": search = command.argument
        case "sort": sort = command.argument == "tracks" ? .mostTracks : .name
        case "rename": if all.indices.contains(index) { renaming = all[index] }
        case "remove": if all.indices.contains(index) { removing = all[index] }
        case "actions": SourceMenus.popUp(libraryActions, under: actionsAnchor.view)
        case "sort-menu": openSortMenu()
        case "card-menu":
            if all.indices.contains(index) {
                SourceMenus.popUp(SourceCard.menuEntries(all[index], rename: {}, remove: {}), under: actionsAnchor.view)
            }
        default: break
        }
    }
    #endif
}

/// The Sources page's orders.
enum SourceSort: CaseIterable {
    case name, mostTracks

    @MainActor var label: String {
        switch self {
        case .name: tr("sort_name")
        case .mostTracks: tr("sort_most_tracks")
        }
    }
}

private struct SourceSection {
    let kind: SourceKind
    let sources: [SourceInfo]
}

/// What the source list depends on: it is read again when any of it changes.
private struct RefreshKey: Equatable {
    let version: Int
    let roots: [Locator]?
    let scanning: [Locator]?
    let signedIn: Bool
    let shown: Bool
}
