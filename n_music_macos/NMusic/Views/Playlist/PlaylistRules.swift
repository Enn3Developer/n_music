import SwiftUI

/// A read-only FilterChip (28 points, 10 padding, 5 between its words; `qml/FilterChip.qml`): the name in `text2` and the detail in `text`, on a
/// `raised` pill (`menuHover` on hover). Clicking it opens the rules drawer.
struct PlaylistRuleChip: View {
    let name: String
    let detail: String
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 5) {
                Text(name).foregroundStyle(Theme.text2)
                Text(detail).foregroundStyle(Theme.text)
            }
            .font(.system(size: 13))
            .lineLimit(1)
            .padding(.horizontal, 10)
            .frame(height: 28)
            .background(Capsule().fill(hovering ? Theme.menuHover : Theme.raised))
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .animation(Motion.standard(Motion.fade), value: hovering)
        .accessibilityLabel(name + " " + detail)
    }
}

/// Lays its children out in rows, wrapping at the width it gets (Qt's Flow).
struct PlaylistFlow: Layout {
    var spacing: CGFloat = 6
    var lineSpacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, line: CGFloat = 0, widest: CGFloat = 0
        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x > 0 && x + size.width > width {
                y += line + lineSpacing
                x = 0
                line = 0
            }
            x += size.width + spacing
            line = max(line, size.height)
            widest = max(widest, x - spacing)
        }
        return CGSize(width: min(widest, width), height: y + line)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, line: CGFloat = 0
        var row: [(LayoutSubview, CGSize, CGFloat)] = []
        func flush() {
            for (subview, size, at) in row {
                subview.place(at: CGPoint(x: at, y: y + (line - size.height) / 2), proposal: ProposedViewSize(size))
            }
            row.removeAll()
        }
        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x > bounds.minX && x + size.width > bounds.maxX {
                flush()
                y += line + lineSpacing
                x = bounds.minX
                line = 0
            }
            row.append((subview, size, x))
            x += size.width + spacing
            line = max(line, size.height)
        }
        flush()
    }
}
