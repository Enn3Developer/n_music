import SwiftUI

/// The Qt SearchField (spec §2.8): 38 points tall, radius 10, `field` ground with a `line2`
/// border (`border` while focused), a magnifier, and a round "×" clearing it (Clear search)
/// once there is text. Escape clears the text when there is any, else passes on.
struct TracksSearchField: View {
    @Binding var text: String
    var placeholder: String
    var height: CGFloat = 38
    var radius: CGFloat = 10
    var focused: FocusState<Bool>.Binding

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.text3)
                .padding(.leading, 12)
            TextField("", text: $text, prompt: Text(placeholder).foregroundStyle(Theme.text3))
                .textFieldStyle(.plain)
                .font(.system(size: 14))
                .foregroundStyle(Theme.text)
                .focused(focused)
                // Escape clears the text; with none it passes on (`qml/SearchField.qml:48-51`).
                .onKeyPress(.escape) {
                    guard !text.isEmpty else { return .ignored }
                    text = ""
                    return .handled
                }
                .accessibilityLabel(placeholder)
            if !text.isEmpty {
                Button {
                    text = ""
                    focused.wrappedValue = true
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundStyle(Theme.text2)
                        .frame(width: 18, height: 18)
                        .background(Circle().fill(Theme.raised))
                        .frame(width: 24, height: 24)
                        .contentShape(Circle())
                }
                .buttonStyle(.plain)
                .focusable(false)
                .help(tr("clear_search"))
                .accessibilityLabel(tr("clear_search"))
                .padding(.trailing, 6)
            }
        }
        .frame(height: height)
        .background(RoundedRectangle(cornerRadius: radius, style: .continuous).fill(Theme.field))
        .overlay(
            RoundedRectangle(cornerRadius: radius, style: .continuous)
                .strokeBorder(focused.wrappedValue ? Theme.border : Theme.line2, lineWidth: 1)
        )
        .contentShape(Rectangle())
        .onTapGesture { focused.wrappedValue = true }
    }
}

/// An applied rule (spec §3.1, `qml/TracksFilterChip.qml`): a 32-point pill, `raised` (`menuHover`
/// on hover), the name in `text2` and the rest in `text`; clicking it opens the drawer, its
/// "×" removes the rule at once.
struct TracksFilterChip: View {
    let name: String
    let text: String
    let open: () -> Void
    let remove: () -> Void

    @State private var hovering = false

    var body: some View {
        HStack(spacing: 2) {
            Button(action: open) {
                (Text(name + " ").foregroundStyle(Theme.text2) + Text(text).foregroundStyle(Theme.text))
                    .font(.system(size: 13))
                    .lineLimit(1)
                    .padding(.leading, 12)
                    .frame(height: 32)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            Button(action: remove) {
                Image(systemName: "xmark")
                    .font(.system(size: 10, weight: .bold))
                    .foregroundStyle(Theme.text2)
                    .frame(width: 24, height: 24)
                    .contentShape(Circle())
            }
            .buttonStyle(IconButtonStyle(radius: 12))
            .help(tr("remove_filter", name + " " + text))
            .accessibilityLabel(tr("remove_filter", name + " " + text))
            .padding(.trailing, 4)
        }
        .background(Capsule().fill(hovering ? Theme.menuHover : Theme.raised))
        .onHover { hovering = $0 }
    }
}

/// The dashed "Filter" pill opening the drawer (32 points, radius 16).
struct TracksFilterButton: View {
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Label(tr("filter"), systemImage: "line.3.horizontal.decrease")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.text2)
                .padding(.horizontal, 12)
                .frame(height: 32)
                .background(Capsule().fill(hovering ? Theme.hover : .clear))
                .overlay(Capsule().strokeBorder(Theme.border, style: StrokeStyle(lineWidth: 1, dash: [4, 3])))
                .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityLabel(tr("filter"))
    }
}

/// Compact windows' filter button: 36 points, outlined, radius 9, with an accent badge
/// counting the applied rules.
struct TracksFilterIconButton: View {
    let count: Int
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        let label = count > 0 ? tr("filters_active", count) : tr("filter")
        Button(action: action) {
            Image(systemName: "line.3.horizontal.decrease")
                .font(.system(size: 14, weight: .medium))
                .foregroundStyle(Theme.text2)
                .frame(width: 36, height: 36)
                .background(RoundedRectangle(cornerRadius: 9, style: .continuous).fill(hovering ? Theme.hover : .clear))
                .overlay(RoundedRectangle(cornerRadius: 9, style: .continuous).strokeBorder(Theme.line2, lineWidth: 1))
                .overlay(alignment: .topTrailing) {
                    if count > 0 {
                        Text(String(count))
                            .font(.system(size: 10, weight: .bold))
                            .monospacedDigit()
                            .foregroundStyle(Theme.accentInk)
                            .padding(.horizontal, 4)
                            .frame(minWidth: 16, minHeight: 16)
                            .background(Capsule().fill(Theme.accent))
                            .offset(x: 5, y: -5)
                    }
                }
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
    }
}

/// Lays its children out in rows, wrapping when a row is full (Qt's Flow).
struct TracksChipFlow: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        // Never narrower than its widest child: a chip doesn't shrink.
        let widest = subviews.map { $0.sizeThatFits(.unspecified).width }.max() ?? 0
        let width = max(proposal.width ?? .infinity, widest)
        let rows = arrange(width: width, subviews: subviews)
        let height = rows.map(\.height).reduce(0, +) + spacing * CGFloat(max(0, rows.count - 1))
        let used = rows.map(\.width).max() ?? 0
        return CGSize(width: min(used, width), height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var y = bounds.minY
        for row in arrange(width: bounds.width, subviews: subviews) {
            var x = bounds.minX
            for index in row.items {
                let size = subviews[index].sizeThatFits(.unspecified)
                subviews[index].place(at: CGPoint(x: x, y: y + (row.height - size.height) / 2), proposal: ProposedViewSize(size))
                x += size.width + spacing
            }
            y += row.height + spacing
        }
    }

    private struct Row {
        var items: [Int] = []
        var width: CGFloat = 0
        var height: CGFloat = 0
    }

    private func arrange(width: CGFloat, subviews: Subviews) -> [Row] {
        var rows: [Row] = []
        var row = Row()
        for index in subviews.indices {
            let size = subviews[index].sizeThatFits(.unspecified)
            let needed = row.items.isEmpty ? size.width : row.width + spacing + size.width
            if !row.items.isEmpty, needed > width {
                rows.append(row)
                row = Row()
            }
            row.width = row.items.isEmpty ? size.width : row.width + spacing + size.width
            row.height = max(row.height, size.height)
            row.items.append(index)
        }
        if !row.items.isEmpty { rows.append(row) }
        return rows
    }
}
