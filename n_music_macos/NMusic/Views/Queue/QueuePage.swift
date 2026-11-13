import AppKit
import SwiftUI

/// The queue page (spec §3.8): on the left the "now" column (cover, "Now playing", title,
/// "artist · album · year", the format/ReplayGain/plays chips, "At the end" and Shuffle); on the
/// right "Playing from …" with Clear queued and Open in library, over the session: what played,
/// the current track, what is left. It opens with the current track at the top.
///
/// Always two columns: Qt stacks them only in its narrow window class (under 720 wide), which
/// the window's minimum width (760) rules out here (brief §1).
struct QueuePage: View {
    private let store = Store.shared
    private let model = QueueModel.shared
    private let navigation = Navigation.shared
    private let settings = UISettings.shared
    private let context = PlayContext.shared

    var body: some View {
        GeometryReader { geometry in
            let width = geometry.size.width
            let height = geometry.size.height
            let nowWidth = max(300, min(360, width - 56 - 32 - 420))
            let rows = model.page?.rows ?? []
            ZStack(alignment: .topLeading) {
                HStack(alignment: .top, spacing: 32) {
                    ScrollView(.vertical) {
                        NowColumn(width: nowWidth, height: height)
                            .padding(.top, 28)
                            .padding(.bottom, 24)
                    }
                    .scrollIndicators(.never)
                    .frame(width: nowWidth)
                    VStack(alignment: .leading, spacing: 12) {
                        if !rows.isEmpty {
                            origin
                        }
                        QueueTable(mode: .page, rows: rows, compact: settings.compactRows)
                    }
                    .padding(.top, 28)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                }
                .padding(.horizontal, 28)
                if rows.isEmpty, store.current == nil, model.page != nil {
                    EmptyStateView(
                        // The sidebar's queue icon, as Qt's empty state uses its `queue` icon.
                        symbol: "text.line.first.and.arrowtriangle.forward",
                        title: tr("queue_empty"),
                        message: tr("queue_empty_hint"),
                        buttonTitle: tr("open_tracks")
                    ) { navigation.go(PageID.tracks) }
                    .padding(.leading, 28 + nowWidth + 32)
                    .padding(.trailing, 28)
                }
            }
            .frame(width: width, height: height, alignment: .topLeading)
        }
        .onAppear { model.start() }
        // It opens on the current track.
        .onChange(of: navigation.section == PageID.queue) { _, shown in
            if shown { model.pageTable?.scrollToCurrent() }
        }
    }

    /// What the session plays from, and Clear queued / Open in library; the actions go under it
    /// when both don't fit.
    private var origin: some View {
        let parts = [
            context.label,
            context.detail,
            store.shuffle ? tr("shuffled") : "",
            tr("left_count", Format.number(model.page?.left ?? 0)),
        ].filter { !$0.isEmpty }
        let label = Text(tr("playing_from", parts.joined(separator: " · ")))
            .font(.system(size: 13))
            .foregroundStyle(Theme.text2)
            .lineLimit(1)
            .truncationMode(.tail)
        let actions = HStack(spacing: 14) {
            if (model.page?.upNext ?? 0) > 0 {
                Button(tr("clear_queued")) { store.clearQueued() }
                    .buttonStyle(QuietButtonStyle())
            }
            UnderlinedLink(title: tr("open_in_library")) {
                navigation.go(context.page)
            }
        }
        return ViewThatFits(in: .horizontal) {
            HStack(spacing: 12) {
                label.fixedSize()
                Spacer(minLength: 0)
                actions.fixedSize()
            }
            VStack(alignment: .leading, spacing: 6) {
                label
                HStack {
                    Spacer(minLength: 0)
                    actions
                }
            }
        }
    }

}

/// The "now" column: cover, "Now playing", title, "artist · album · year", chips, "At the end"
/// and Shuffle. The cover shrinks (to 160 at least) to keep the controls in view.
private struct NowColumn: View {
    let width: CGFloat
    let height: CGFloat

    private let store = Store.shared
    private let facts = CurrentFacts.shared
    private let navigation = Navigation.shared
    @State private var aboutHeight: CGFloat = 120
    @State private var chipsHeight: CGFloat = 26
    @State private var endingHeight: CGFloat = 120

    private struct Facts: Equatable {
        let audio: String
        let gain: String
        let plays: UInt32
    }

    var body: some View {
        let loaded = store.current != nil
        let spacing: CGFloat = 18
        let cover = max(160, min(width, height - 28 - 24 - 3 * spacing - aboutHeight - (loaded ? chipsHeight + spacing : 0) - endingHeight))
        VStack(alignment: .leading, spacing: spacing) {
            // Kept alive while another page shows: it reads covers only while shown.
            SwapCover(size: cover, radius: 12, full: true, active: navigation.section == PageID.queue)
            VStack(alignment: .leading, spacing: 6) {
                Text(tr("now_playing").uppercased())
                    .font(.system(size: 12, weight: .semibold))
                    .tracking(0.96)
                    .foregroundStyle(Theme.accentText)
                SwapWords(store.current?.track) { track in
                    VStack(alignment: .leading, spacing: 6) {
                        if let track {
                            Text(track.title)
                                .font(.system(size: 26, weight: .bold))
                                .tracking(-0.26)
                                .foregroundStyle(Theme.text)
                                .lineLimit(3)
                            let second = track.artistAlbumYear
                            if !second.isEmpty {
                                Text(second)
                                    .font(.system(size: 15))
                                    .foregroundStyle(Theme.text2)
                            }
                        } else {
                            Text(tr("not_playing"))
                                .font(.system(size: 26, weight: .bold))
                                .tracking(-0.26)
                                .foregroundStyle(Theme.text3)
                        }
                    }
                    .fixedSize(horizontal: false, vertical: true)
                }
            }
            .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { aboutHeight = $0 }
            if loaded {
                SwapWords(Facts(audio: facts.audio, gain: facts.gain, plays: facts.plays)) { shown in
                    ChipFlow(spacing: 6) {
                        if !shown.audio.isEmpty { FactChip(text: shown.audio) }
                        if !shown.gain.isEmpty { FactChip(text: shown.gain) }
                        FactChip(text: tr("played_times", Format.number(shown.plays)))
                    }
                }
                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { chipsHeight = $0 }
            }
            AtTheEnd()
                .padding(.top, 4)
                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { endingHeight = $0 }
        }
        .frame(width: width, alignment: .leading)
        .task(id: "\(store.current?.track.locator.key ?? "")\u{1}\(store.libraryVersion)") {
            facts.refresh()
        }
    }
}

/// "At the end": Stop / Repeat all / Repeat one, and Shuffle.
private struct AtTheEnd: View {
    private let store = Store.shared

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(tr("at_the_end").uppercased())
                .font(.system(size: 12, weight: .semibold))
                .tracking(0.72)
                .foregroundStyle(Theme.text3)
            EndSegments(
                titles: [tr("end_stop"), tr("end_repeat_all"), tr("end_repeat_one")],
                selected: store.loopStatus.repeatIndex,
                label: tr("at_the_end")
            ) { store.setLoopStatus(LoopStatus(repeatIndex: $0)) }
            .frame(maxWidth: .infinity)
            HStack {
                Text(tr("shuffle"))
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.text)
                Spacer(minLength: 0)
                Toggle(tr("shuffle"), isOn: Binding(
                    get: { store.shuffle },
                    set: { store.setShuffle($0) }
                ))
                .toggleStyle(.switch)
                .labelsHidden()
            }
            .padding(.horizontal, 4)
            .frame(height: 40)
        }
    }
}

/// Lays the chips out in rows, wrapping (Qt's Flow).
private struct ChipFlow: Layout {
    var spacing: CGFloat

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        var x: CGFloat = 0
        var y: CGFloat = 0
        var line: CGFloat = 0
        var widest: CGFloat = 0
        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x > 0, x + size.width > width {
                y += line + spacing
                x = 0
                line = 0
            }
            x += size.width + spacing
            line = max(line, size.height)
            widest = max(widest, x - spacing)
        }
        return CGSize(width: proposal.width ?? widest, height: y + line)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX
        var y = bounds.minY
        var line: CGFloat = 0
        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x > bounds.minX, x + size.width > bounds.maxX {
                y += line + spacing
                x = bounds.minX
                line = 0
            }
            subview.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            line = max(line, size.height)
        }
    }
}

/// A native segmented control whose segments share the whole width (SwiftUI's segmented
/// picker keeps its own width on the Mac). Its selected segment stays neutral, as Qt's
/// SegmentedControl (a `line2` thumb on a `field` ground), not in the accent.
private struct EndSegments: NSViewRepresentable {
    let titles: [String]
    let selected: Int
    let label: String
    let onSelect: (Int) -> Void

    func makeCoordinator() -> Target { Target() }

    func makeNSView(context: Context) -> NSSegmentedControl {
        let control = NSSegmentedControl(labels: titles, trackingMode: .selectOne, target: context.coordinator, action: #selector(Target.changed(_:)))
        control.segmentDistribution = .fillEqually
        control.controlSize = .large
        control.setContentHuggingPriority(.defaultLow, for: .horizontal)
        control.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return control
    }

    func updateNSView(_ control: NSSegmentedControl, context: Context) {
        context.coordinator.onSelect = onSelect
        for (index, title) in titles.enumerated() where control.label(forSegment: index) != title {
            control.setLabel(title, forSegment: index)
        }
        if control.selectedSegment != selected { control.selectedSegment = selected }
        control.setAccessibilityLabel(label)
    }

    @MainActor
    final class Target: NSObject {
        var onSelect: (Int) -> Void = { _ in }
        @objc func changed(_ sender: NSSegmentedControl) { onSelect(sender.selectedSegment) }
    }
}
