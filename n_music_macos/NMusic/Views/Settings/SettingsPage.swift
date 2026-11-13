import SwiftUI

/// The in-window Settings page (spec §6, brief §4): the section list on the left (Playback,
/// Telegram when the build has it, Appearance, Updates, About) and every section one under the
/// other on the right, at most 704 points wide, pinned left. Below 880 points the section list
/// becomes a wrapping row of chips above the settings.
///
/// The list's highlight follows the scroll: a section is current once its top is within 40
/// points of the top, and at the very end the last one is. As in Qt, `section` (Settings'
/// remembered sub-page) changes only with the route, never by scrolling: clicking a section
/// goes to `settings:<section>` (a history entry), and whenever `section` changes (a click,
/// back/forward, the Go menu) the page scrolls there at once. Coming back to Settings with the
/// same section keeps the scroll where it was; clicking the section already routed scrolls to
/// it without a history entry.
struct SettingsPage: View {
    let section: String

    private let store = Store.shared
    private let navigation = Navigation.shared

    /// The section in view, as the page scrolls.
    @State private var visible = "playback"
    @State private var geometry = ScrollGeometry()
    /// Whether the page was scrolled to its very end when another page was shown.
    @State private var leftAtEnd = false

    nonisolated private static let space = "settings.scroll"
    private static let top = "settings.top"
    private static let end = "settings.end"

    private var sections: [(value: String, label: String)] {
        var sections = [("playback", tr("playback"))]
        if store.telegram != nil { sections.append(("telegram", tr("telegram"))) }
        sections += [("appearance", tr("appearance")), ("updates", tr("updates")), ("about", tr("about"))]
        return sections
    }

    var body: some View {
        GeometryReader { page in
            let stacked = page.size.width < 880
            ScrollViewReader { proxy in
                // One layout for both arrangements, so the scroll view (and where it is
                // scrolled to) stays when the page crosses 880 points.
                let layout = stacked
                    ? AnyLayout(VStackLayout(alignment: .leading, spacing: 0))
                    : AnyLayout(HStackLayout(alignment: .top, spacing: 0))
                layout {
                    Group {
                        if stacked {
                            VStack(alignment: .leading, spacing: 0) {
                                title.padding(.bottom, 12)
                                FlowLayout(spacing: 4) { buttons(proxy) }
                            }
                            .padding(.top, 22)
                            .padding(.horizontal, 28)
                        } else {
                            VStack(alignment: .leading, spacing: 2) {
                                title
                                    .padding(.leading, 10)
                                    .padding(.bottom, 14)
                                buttons(proxy)
                            }
                            .frame(width: 200, alignment: .leading)
                            .padding(.top, 28)
                            .padding(.leading, 20)
                            .padding(.trailing, 12)
                        }
                    }
                    content(width: stacked ? page.size.width : page.size.width - 232, top: stacked ? 0 : 6)
                }
                .onAppear {
                    #if DEBUG
                    SettingsDebug.install()
                    #endif
                    Updates.shared.settingsOpened()
                    reveal(section, proxy, later: true)
                }
                // Qt's `onSectionChanged: reveal(section)`.
                .onChange(of: section) { reveal(section, proxy, later: false) }
                .onChange(of: navigation.section) { old, new in
                    // Back on Settings, at the same section: the scroll stays where it was (the
                    // shell's hidden pages keep theirs, but one at its very end comes back about
                    // 50 points short of it; this puts it back at the end).
                    if old == "settings" {
                        leftAtEnd = geometry.atEnd
                    } else if new == "settings", leftAtEnd {
                        Task { @MainActor in
                            var instant = Transaction()
                            instant.disablesAnimations = true
                            withTransaction(instant) { proxy.scrollTo(Self.end, anchor: .bottom) }
                        }
                    }
                }
                .onChange(of: sections.map(\.value)) { geometry.update(names: sections.map(\.value)) }
            }
        }
    }

    private var title: some View {
        Text(tr("settings"))
            .font(.system(size: 26, weight: .bold))
            .tracking(-0.52)
            .foregroundStyle(Theme.text)
            .accessibilityAddTraits(.isHeader)
    }

    @ViewBuilder
    private func buttons(_ proxy: ScrollViewProxy) -> some View {
        ForEach(sections, id: \.value) { entry in
            SettingsSectionButton(title: entry.label, current: visible == entry.value) {
                if section == entry.value {
                    reveal(entry.value, proxy, later: false)
                } else {
                    navigation.go(PageID.settings(entry.value))
                }
            }
        }
    }

    /// The sections, in a scroll view: each starts with a 28-point anchor, so scrolling to it
    /// leaves its heading 28 points under the top (Qt's `section.y − 28`); Playback scrolls to
    /// the very top. Sections are 34 points apart.
    private func content(width: CGFloat, top: CGFloat) -> some View {
        ScrollView(.vertical) {
            VStack(spacing: 0) {
                VStack(alignment: .leading, spacing: 6) {
                    Color.clear.frame(height: 0).id(Self.top)
                    ForEach(sections, id: \.value) { entry in
                        VStack(alignment: .leading, spacing: 0) {
                            Color.clear.frame(height: 28).id(entry.value)
                            sectionView(entry.value)
                                .onGeometryChange(for: CGFloat.self) { proxy in
                                    proxy.frame(in: .named(Self.space)).minY
                                } action: { minY in
                                    geometry.tops[entry.value] = minY
                                    follow()
                                }
                        }
                    }
                }
                .padding(.top, top)
                .padding(.bottom, 32)
                .frame(width: max(0, min(width - 56, 704)), alignment: .leading)
                .padding(.leading, 28)
                .frame(maxWidth: .infinity, alignment: .leading)
                .onGeometryChange(for: CGFloat.self) { proxy in
                    proxy.frame(in: .named(Self.space)).maxY
                } action: { maxY in
                    geometry.bottom = maxY
                    follow()
                }
                Color.clear.frame(height: 0).id(Self.end)
            }
        }
        .coordinateSpace(.named(Self.space))
        .onGeometryChange(for: CGFloat.self) { proxy in proxy.size.height } action: { height in
            geometry.height = height
            follow()
        }
    }

    @ViewBuilder
    private func sectionView(_ name: String) -> some View {
        switch name {
        case "playback": PlaybackSettings()
        case "telegram":
            if let telegram = store.telegram { TelegramSettings(state: telegram) }
        case "appearance": AppearanceSettings()
        case "updates": UpdatesSettings()
        default: AboutSettings()
        }
    }

    /// Scrolls at once to `name` (instantly, as Qt). `later` waits for the first layout.
    private func reveal(_ name: String, _ proxy: ScrollViewProxy, later: Bool) {
        let target = name == "playback" || !sections.contains(where: { $0.value == name }) ? Self.top : name
        let scroll = {
            var instant = Transaction()
            instant.disablesAnimations = true
            withTransaction(instant) { proxy.scrollTo(target, anchor: .top) }
        }
        if later {
            Task { @MainActor in scroll() }
        } else {
            scroll()
        }
    }

    /// The section in view changed: highlight it (only the highlight: Settings' section is
    /// the route's, as in Qt).
    private func follow() {
        let now = geometry.visible(sections.map(\.value))
        guard now != visible else { return }
        visible = now
        #if DEBUG
        SettingsDebug.visible = now
        #endif
    }
}

/// Where the sections are in the scroll view, updated as it scrolls without redrawing the page:
/// only the section in view is state.
@MainActor
final class ScrollGeometry {
    /// Each section's top, in the scroll view's own coordinates.
    var tops: [String: CGFloat] = [:]
    /// The content's end, likewise.
    var bottom: CGFloat = .infinity
    /// The scroll view's height.
    var height: CGFloat = 0

    /// Scrolled to the very end.
    var atEnd: Bool { height > 0 && bottom <= height + 1 }

    func update(names: [String]) {
        tops = tops.filter { names.contains($0.key) }
    }

    /// At the end the last section is in view, however short; else the last whose top is
    /// within 40 points of the top.
    func visible(_ names: [String]) -> String {
        guard let first = names.first else { return "playback" }
        if atEnd { return names.last ?? first }
        var shown = first
        for name in names where (tops[name] ?? .infinity) <= 40 {
            shown = name
        }
        return shown
    }
}

/// A section to go to (Qt's SectionButton): 36 points tall, radius 8, 14-point text; the one in
/// view `raised` and DemiBold in `text`, hover `hover`, the others `text2` Medium.
struct SettingsSectionButton: View {
    let title: String
    let current: Bool
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Text(title)
                .font(.system(size: 14, weight: current ? .semibold : .medium))
                .foregroundStyle(current || hovering ? Theme.text : Theme.text2)
                .lineLimit(1)
                .padding(.horizontal, 10)
                .frame(height: 36)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .fill(current ? Theme.raised : hovering ? Theme.hover : .clear)
                )
                .contentShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                .animation(Motion.standard(Motion.fade), value: current)
        }
        .settingsStyle(RoundedRectangle(cornerRadius: 8, style: .continuous))
        .onHover { hovering = $0 }
        .accessibilityAddTraits(current ? [.isSelected] : [])
    }
}
