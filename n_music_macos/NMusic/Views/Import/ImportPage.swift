import SwiftUI

/// The first run (Qt `ImportPage.qml`, spec §5.1), filling the window while
/// `Store.shared.firstRun`: where the music is, before the library shows. The kind cards add
/// sources to the "Added" list (seeded once with the library's suggested roots, the Music
/// folder on a fresh install); "Build my library" makes them the library's sources, even none,
/// ending the first run.
struct ImportPage: View {
    private let store = Store.shared

    @State private var added: [StagedSource] = []
    @State private var seeded = false
    @State private var addingWeb = false
    @State private var addingTelegram = false

    var body: some View {
        GeometryReader { geometry in
            let width = min(720, geometry.size.width - 32)
            ScrollView {
                VStack(alignment: .leading, spacing: 26) {
                    brand
                    VStack(alignment: .leading, spacing: 10) {
                        Text(tr("import_title"))
                            .font(.system(size: 34, weight: .bold))
                            .tracking(-0.68)
                            .foregroundStyle(Theme.text)
                            .fixedSize(horizontal: false, vertical: true)
                        Text(tr("import_intro"))
                            .font(.system(size: 16))
                            .lineSpacing(3)
                            .foregroundStyle(Theme.text2)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    kinds(width: width)
                    if !added.isEmpty {
                        addedList
                    }
                    HStack {
                        Spacer()
                        BuildButton(action: build)
                    }
                }
                .frame(width: max(1, width))
                .padding(.vertical, 32)
                .frame(maxWidth: .infinity)
                .frame(minHeight: geometry.size.height)
            }
        }
        .background(Theme.bg)
        // Once the library's roots are known, as suggestions with their names (Qt seeds from
        // `Sources.items`).
        .task(id: store.roots) { await seed() }
        .sourceAddSheets(web: $addingWeb, telegram: $addingTelegram, taken: taken) { root, name in
            stage(root, name: name)
        }
        #if DEBUG
        .onAppear { SourcesDebug.install() }
        .onChange(of: SourcesDebug.shared.request) { _, _ in debugRequest() }
        #endif
    }

    private var brand: some View {
        HStack(spacing: 10) {
            LogoView(size: 36, radius: 10)
            Text(verbatim: "N Music")
                .font(.system(size: 20, weight: .bold))
                .tracking(-0.2)
                .foregroundStyle(Theme.text)
        }
    }

    // MARK: Kind cards

    /// floor((width + 10) / 210) columns, 10 apart; the cards of a row are as wide and tall.
    private func kinds(width: CGFloat) -> some View {
        let columns = max(1, Int((width + 10) / 210))
        let rows = stride(from: 0, to: SourceKind.allCases.count, by: columns).map {
            Array(SourceKind.allCases[$0..<min($0 + columns, SourceKind.allCases.count)])
        }
        return VStack(alignment: .leading, spacing: 10) {
            ForEach(rows, id: \.first) { row in
                HStack(alignment: .top, spacing: 10) {
                    ForEach(row) { kind in
                        KindCard(kind: kind) { choose(kind) }
                    }
                    // A short last row keeps the columns' width.
                    ForEach(0..<(columns - row.count), id: \.self) { _ in
                        Color.clear.frame(maxWidth: .infinity, maxHeight: 1)
                    }
                }
                .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    // MARK: Added

    private var addedList: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(tr("added").uppercased())
                .font(.system(size: 13, weight: .semibold))
                .tracking(0.78)
                .foregroundStyle(Theme.text3)
                .padding(.bottom, 2)
                .accessibilityAddTraits(.isHeader)
            ForEach(added) { source in
                StagedRow(source: source) {
                    added.removeAll { $0.id == source.id }
                }
            }
        }
    }

    private var taken: Set<Locator> { Set(added.map(\.root)) }

    /// Stages the library's sources once they are known, as suggestions.
    private func seed() async {
        guard !seeded, store.roots != nil else { return }
        let sources = await store.sourceList()
        guard !seeded, !Task.isCancelled else { return }
        seeded = true
        let suggested = sources.map { source in
            StagedSource(root: source.root, name: source.name, displayName: source.displayName ?? "", suggested: true)
        }
        // Anything staged while the list came stays, after them.
        added = suggested + added.filter { staged in !suggested.contains { $0.root == staged.root } }
    }

    /// Stages `root` unless it is there already (silently, as Qt), named `name` unless empty.
    private func stage(_ root: Locator, name: String) {
        guard !root.location.isEmpty, !added.contains(where: { $0.root == root }) else { return }
        added.append(StagedSource(root: root, name: name.isEmpty ? root.defaultName : name, displayName: name, suggested: false))
    }

    private func choose(_ kind: SourceKind) {
        switch kind {
        case .folder: FolderPanel.choose(.firstRun, taken: taken, chosen: stage)
        case .web: addingWeb = true
        case .telegram: addingTelegram = true
        case .spotify, .youtube, .deezer: break
        }
    }

    /// Never disabled: the first run can end with no sources.
    private func build() {
        store.setSources(added.map(\.root), names: added.map(\.displayName))
    }

    #if DEBUG
    private func debugRequest() {
        guard store.firstRun, let command = SourcesDebug.shared.command else { return }
        switch command.name {
        case "close":
            addingWeb = false
            addingTelegram = false
        case "import-web": addingWeb = true
        case "import-telegram": addingTelegram = true
        case "import-folder": choose(.folder)
        case "import-stage": stage(sourceFromLocation(location: command.argument), name: "")
        case "import-unstage": if !added.isEmpty { added.removeFirst() }
        case "import-build": build()
        default: break
        }
    }
    #endif
}

/// A source staged on the import page, as `SourceInfo` would list it.
private struct StagedSource: Identifiable {
    let root: Locator
    let name: String
    /// The name typed; "" for the default name.
    let displayName: String
    /// The library suggested it (the Music folder).
    let suggested: Bool

    var id: String { root.key }
}

/// A kind of source to add (spec §5.1): padding 14, radius 12; the icon well (accent, `text3`
/// when later) with a "Later" badge; the name (15 DemiBold) over the detail (13). Available:
/// `surface` with an accent border, hover `hover`, pressed `selected`; later: transparent with a
/// neutral border, disabled, the name in `text2`.
private struct KindCard: View {
    let kind: SourceKind
    let action: () -> Void

    var body: some View {
        let later = kind.later
        Button(action: action) {
            VStack(alignment: .leading, spacing: 8) {
                HStack(alignment: .center) {
                    KindWell(symbol: kind.symbol, iconSize: 19, tint: later ? Theme.text3 : Theme.accentText)
                    Spacer(minLength: 0)
                    if later {
                        SourceBadge(text: tr("later"))
                    }
                }
                Text(kind.name)
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(later ? Theme.text2 : Theme.text)
                    .lineLimit(1)
                Text(kind.detail)
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.text2)
                    .lineSpacing(1)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            }
            .padding(14)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
        .buttonStyle(KindCardStyle(later: later))
        .disabled(later)
        .accessibilityLabel(kind.name)
        .accessibilityHint(kind.detail)
    }
}

private struct KindCardStyle: ButtonStyle {
    let later: Bool
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        let shape = RoundedRectangle(cornerRadius: 12, style: .continuous)
        configuration.label
            .background(shape.fill(
                later ? Color.clear : configuration.isPressed ? Theme.selected : hovering ? Theme.hover : Theme.surface
            ))
            .overlay(shape.strokeBorder(later ? Theme.line : Theme.accent, lineWidth: 1))
            .onHover { hovering = $0 && !later }
            .animation(Motion.standard(Motion.fade), value: hovering)
            .animation(Motion.standard(Motion.fade), value: configuration.isPressed)
    }
}

/// A staged source: radius 12, at least 60 points; the kind's icon, the name over
/// "<Kind> · <location>" (elided in the middle), "Suggested" for the library's own, and × to
/// unstage it.
private struct StagedRow: View {
    let source: StagedSource
    let remove: () -> Void

    var body: some View {
        let kind = SourceKind(source.root)
        HStack(spacing: 12) {
            KindWell(symbol: kind.symbol, iconSize: 18)
            VStack(alignment: .leading, spacing: 2) {
                Text(source.name)
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(Theme.text)
                    .lineLimit(1)
                Text(kind.name + " · " + sourceLocation(root: source.root))
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.text2)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if source.suggested {
                SourceBadge(text: tr("suggested"), height: 24, padding: 8, font: .system(size: 12))
            }
            IconButton(symbol: "xmark", label: tr("remove_source", source.name), size: 36, iconSize: 14, tint: Theme.text3, action: remove)
        }
        .padding(.leading, 14)
        .padding(.trailing, 10)
        .padding(.vertical, 10)
        .frame(minHeight: 60)
        .background(RoundedRectangle(cornerRadius: 12, style: .continuous).fill(Theme.surface))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Theme.line2, lineWidth: 1))
    }
}

/// "Build my library": a right-aligned accent pill, 46 points, 15 Bold, with a trailing arrow.
private struct BuildButton: View {
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Text(tr("build_library"))
                    .font(.system(size: 15, weight: .bold))
                Image(systemName: "arrow.right")
                    .font(.system(size: 14, weight: .bold))
            }
            .foregroundStyle(Theme.accentInk)
            .padding(.horizontal, 24)
            .frame(height: 46)
            .contentShape(Capsule())
        }
        .buttonStyle(BuildButtonStyle())
    }
}

private struct BuildButtonStyle: ButtonStyle {
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(Capsule().fill(Theme.accent))
            .overlay(Capsule().fill(Color.black.opacity(configuration.isPressed ? 0.08 : 0)))
            .overlay(Capsule().fill(Color.white.opacity(hovering && !configuration.isPressed ? 0.06 : 0)))
            .onHover { hovering = $0 }
            .animation(Motion.standard(Motion.fade), value: hovering)
            .animation(Motion.standard(Motion.fade), value: configuration.isPressed)
    }
}
