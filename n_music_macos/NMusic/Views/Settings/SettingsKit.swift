import SwiftUI

// The pieces of the Settings page (spec §6.2): a section heading, the rounded group card, the
// row with its words on the left and its controls on the right (or below when the row is
// narrow), the accent swatch and the track-column chip.

/// A section's heading: 20 Bold.
struct SettingsHeading: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.system(size: 20, weight: .bold))
            .foregroundStyle(Theme.text)
            .accessibilityAddTraits(.isHeader)
    }
}

/// A rounded card of settings, one row under the other: radius 12, `surface`, 1-point `line2`
/// border.
struct SettingsGroup<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            content
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(1)
        .background(RoundedRectangle(cornerRadius: 12, style: .continuous).fill(Theme.surface))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Theme.line2, lineWidth: 1))
    }
}

/// A 1-point `line` across a group, between two rows.
struct SettingsDivider: View {
    var body: some View {
        Rectangle().fill(Theme.line).frame(height: 1)
    }
}

/// A row of a group (Qt's SettingRow): a 14-point DemiBold title, a 13-point `text2`
/// description with an optional 14-point leading icon, and the controls at the end, 16 points
/// apart; they go under the words when the row leaves the words less than 160 points, or when
/// `controlsBelow`. Padding 16/14; a line under it with `divider`.
struct SettingRow<Controls: View>: View {
    var title: String = ""
    var description: String = ""
    var descriptionIcon: String? = nil
    var descriptionIconColor: Color = Theme.text2
    var divider = false
    var controlsBelow = false
    @ViewBuilder var controls: Controls

    var body: some View {
        VStack(spacing: 0) {
            SettingRowLayout(below: controlsBelow) {
                words
                HStack(spacing: 16) { controls }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            if divider { SettingsDivider() }
        }
        .accessibilityElement(children: .contain)
    }

    private var words: some View {
        VStack(alignment: .leading, spacing: 3) {
            if !title.isEmpty {
                Text(title)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true)
            }
            if !description.isEmpty {
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    if let descriptionIcon {
                        Image(systemName: descriptionIcon)
                            .font(.system(size: 11, weight: .bold))
                            .foregroundStyle(descriptionIconColor)
                            .frame(width: 14)
                    }
                    Text(description)
                        .font(.system(size: 13))
                        .foregroundStyle(Theme.text2)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

/// Lays out a row's words (first subview) and controls (second): side by side, the controls at
/// their ideal width and both centred vertically; or the controls 12 points under the words when
/// the words would get less than 160 points (Qt's `width − 32 − controlsWidth < 160`, where the
/// controls' width counts the 16-point spacing).
struct SettingRowLayout: Layout {
    var below = false

    private func stacked(_ width: CGFloat, _ controls: CGSize) -> Bool {
        below || width - controls.width - 16 < 160
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        guard subviews.count == 2 else { return .zero }
        let width = proposal.width ?? 600
        let controls = subviews[1].sizeThatFits(.unspecified)
        if stacked(width, controls) {
            let words = subviews[0].sizeThatFits(ProposedViewSize(width: width, height: nil))
            let below = subviews[1].sizeThatFits(ProposedViewSize(width: width, height: nil))
            return CGSize(width: width, height: words.height + 12 + below.height)
        }
        let words = subviews[0].sizeThatFits(ProposedViewSize(width: width - controls.width - 16, height: nil))
        return CGSize(width: width, height: max(words.height, controls.height))
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        guard subviews.count == 2 else { return }
        let width = bounds.width
        let controls = subviews[1].sizeThatFits(.unspecified)
        if stacked(width, controls) {
            let proposed = ProposedViewSize(width: width, height: nil)
            let words = subviews[0].sizeThatFits(proposed)
            subviews[0].place(at: bounds.origin, proposal: proposed)
            subviews[1].place(at: CGPoint(x: bounds.minX, y: bounds.minY + words.height + 12), proposal: proposed)
            return
        }
        let wordsWidth = width - controls.width - 16
        let proposed = ProposedViewSize(width: wordsWidth, height: nil)
        let words = subviews[0].sizeThatFits(proposed)
        subviews[0].place(at: CGPoint(x: bounds.minX, y: bounds.midY - words.height / 2), proposal: proposed)
        subviews[1].place(
            at: CGPoint(x: bounds.maxX - controls.width, y: bounds.midY - controls.height / 2),
            proposal: ProposedViewSize(controls)
        )
    }
}

/// Lines its subviews up left to right, wrapping to a new line where they no longer fit.
struct FlowLayout: Layout {
    var spacing: CGFloat = 6
    var lineSpacing: CGFloat? = nil

    private func lines(_ width: CGFloat, _ subviews: Subviews) -> [[(index: Int, size: CGSize)]] {
        var lines: [[(index: Int, size: CGSize)]] = [[]]
        var x: CGFloat = 0
        for (index, subview) in subviews.enumerated() {
            let size = subview.sizeThatFits(.unspecified)
            if x > 0, x + size.width > width {
                lines.append([])
                x = 0
            }
            lines[lines.count - 1].append((index, size))
            x += size.width + spacing
        }
        return lines
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        let rows = lines(width, subviews)
        let between = lineSpacing ?? spacing
        var height: CGFloat = 0
        var widest: CGFloat = 0
        for (number, row) in rows.enumerated() where !row.isEmpty {
            height += (row.map(\.size.height).max() ?? 0) + (number > 0 ? between : 0)
            widest = max(widest, row.reduce(0) { $0 + $1.size.width } + spacing * CGFloat(row.count - 1))
        }
        return CGSize(width: proposal.width ?? widest, height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let between = lineSpacing ?? spacing
        var y = bounds.minY
        for row in lines(bounds.width, subviews) where !row.isEmpty {
            let height = row.map(\.size.height).max() ?? 0
            var x = bounds.minX
            for item in row {
                subviews[item.index].place(
                    at: CGPoint(x: x, y: y + (height - item.size.height) / 2),
                    proposal: ProposedViewSize(item.size)
                )
                x += item.size.width + spacing
            }
            y += height + between
        }
    }
}

/// One of the accent colours (Qt's Swatch): a 32-point circle, the colour inset by 4 points, a
/// 2-point ring in `text` while picked (`border` on hover) and a check in the accent's ink.
/// Pressing shrinks it to 90 %.
struct AccentSwatch: View {
    let accent: Accent
    let picked: Bool
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            ZStack {
                Circle()
                    .strokeBorder(picked ? Theme.text : hovering ? Theme.border : .clear, lineWidth: 2)
                Circle()
                    .fill(accent.fill)
                    .padding(4)
                Image(systemName: "checkmark")
                    .font(.system(size: 11, weight: .bold))
                    .foregroundStyle(accent.ink)
                    .opacity(picked ? 1 : 0)
            }
            .frame(width: 32, height: 32)
            .contentShape(Circle())
            .animation(Motion.standard(Motion.fade), value: picked)
            .animation(Motion.standard(Motion.fade), value: hovering)
        }
        .settingsStyle(Circle(), pressScale: 0.9)
        .onHover { hovering = $0 }
        // Brief §17: icon buttons name themselves in a tooltip.
        .help(tr(accent.labelKey))
    }
}

/// A track table column (Qt's ColumnToggle): a 32-point capsule, on with a `raised` fill and an
/// accent check, off with an outline and a plus.
struct ColumnChip: View {
    let label: String
    let on: Bool
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: on ? "checkmark" : "plus")
                    .font(.system(size: 12, weight: .bold))
                    .foregroundStyle(on ? Theme.accentText : Theme.text3)
                    .frame(width: 14)
                Text(label)
                    .font(.system(size: 13))
                    .foregroundStyle(on ? Theme.text : Theme.text2)
            }
            .padding(.leading, 10)
            .padding(.trailing, 12)
            .frame(height: 32)
            .background(Capsule().fill(on ? Theme.raised : hovering ? Theme.hover : .clear))
            .overlay(Capsule().strokeBorder(on ? Theme.raised : Theme.border, lineWidth: 1))
            .contentShape(Capsule())
            .animation(Motion.standard(Motion.fade), value: on)
        }
        .settingsStyle(Capsule())
        .onHover { hovering = $0 }
        // A check box for VoiceOver, as Qt's Accessible.CheckBox.
        .accessibilityRepresentation {
            Toggle(label, isOn: Binding(get: { on }, set: { _ in action() }))
                .toggleStyle(.checkbox)
        }
    }
}

/// An underlined link in `text2` (`text` on hover), as About's "Source code on GitHub".
struct SettingsLink: View {
    let title: String
    let url: URL
    @State private var hovering = false

    var body: some View {
        Button {
            NSWorkspace.shared.open(url)
        } label: {
            Text(title)
                .font(.system(size: 13))
                .underline()
                .foregroundStyle(hovering ? Theme.text : Theme.text2)
        }
        .settingsStyle(Rectangle())
        .onHover { hovering = $0 }
        .accessibilityAddTraits(.isLink)
    }
}
