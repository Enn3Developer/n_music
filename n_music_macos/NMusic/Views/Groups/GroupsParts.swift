import AppKit
import SwiftUI

// Pieces the grids, the collection pages and the playlist pages share (spec §3.3, §3.4, §3.6).

/// What a cover tile shows when there is no cover: the MonoLogo (album art) or an SF Symbol.
enum GroupsPlaceholder: Equatable {
    case logo
    case symbol(String)
}

/// The Qt Cover as the grids and headers use it: a thumbnail (`CoverView`, ImageIO), a 2 × 2
/// mosaic of `mosaic` when it has 4 covers, or the placeholder on a `raised` tile. `radius` nil
/// makes it a circle (artists).
///
/// Covers decode at a stepped size (the side rounded up to 32 points, 64 pixels on Retina) and
/// are scaled into the frame: while the window is resized the grid's side changes every frame,
/// and decoding at each exact size would miss the cache and re-decode every card on screen.
struct GroupsArt: View {
    var path: String?
    var mosaic: [String] = []
    var size: CGFloat
    var radius: CGFloat?
    var placeholder: GroupsPlaceholder = .logo

    var body: some View {
        let decoded = Self.step(size)
        content(decoded)
            .frame(width: decoded, height: decoded)
            .scaleEffect(size / decoded)
            .frame(width: size, height: size)
            .clipShape(shape)
    }

    /// The size covers decode at for a side of `size`.
    static func step(_ size: CGFloat) -> CGFloat {
        max(32, (size / 32).rounded(.up) * 32)
    }

    @ViewBuilder private func content(_ side: CGFloat) -> some View {
        if mosaic.count >= 4 {
            MosaicView(covers: Array(mosaic.prefix(4)), size: side, radius: 0)
        } else if let path, !path.isEmpty {
            CoverView(path: path, size: side, radius: 0, placeholder: symbol)
        } else {
            ZStack {
                Rectangle().fill(Theme.raised)
                switch placeholder {
                case .logo:
                    MonoLogo(size: side)
                case .symbol(let name):
                    Image(systemName: name)
                        .font(.system(size: (min(size * 0.45, 72) * 0.75).rounded(), weight: .regular))
                        .scaleEffect(side / size)
                        .foregroundStyle(Theme.text3)
                }
            }
        }
    }

    private var symbol: String {
        if case .symbol(let name) = placeholder { return name }
        return "music.note"
    }

    private var shape: AnyShape {
        if let radius {
            AnyShape(RoundedRectangle(cornerRadius: radius, style: .continuous))
        } else {
            AnyShape(Circle())
        }
    }
}

/// The Qt MenuButton (spec §13): a label and a chevron in `text2` (13 Medium) on a 38-point
/// pill: `hover` on hover, `selected` while pressed or while its menu is open. The native menu
/// (220 wide, as Qt's PopupMenu) opens right-aligned 4 points below it, its current option
/// checked. Its accessible name is "Sort by: <label>".
struct GroupsMenuButton<Value: Hashable>: View {
    let options: [(value: Value, label: String)]
    let value: Value
    let onSelect: (Value) -> Void

    @State private var hovering = false
    @State private var open = false
    @State private var anchor = SourceMenuAnchor()

    var body: some View {
        let label = options.first { $0.value == value }?.label ?? ""
        Button(action: show) {
            HStack(spacing: 6) {
                Text(label)
                    .font(.system(size: 13, weight: .medium))
                    .lineLimit(1)
                Image(systemName: "chevron.down")
                    .font(.system(size: 10, weight: .semibold))
            }
            .foregroundStyle(Theme.text2)
            .padding(.horizontal, 12)
            .frame(height: 38)
            .contentShape(Rectangle())
        }
        .buttonStyle(GroupsMenuButtonStyle(open: open, hovering: hovering))
        .background(SourceMenuAnchorView(anchor: anchor))
        .fixedSize()
        .onHover { hovering = $0 }
        .help(tr("sort_by") + ": " + label)
        .accessibilityLabel(tr("sort_by") + ": " + label)
    }

    private func show() {
        let onSelect = self.onSelect
        let entries: [SourceMenuEntry] = options.map { option in
            .item(title: option.label, symbol: nil, checked: option.value == value) { onSelect(option.value) }
        }
        // The menu tracks in its own loop: the pill shows `selected` until it closes.
        open = true
        SourceMenus.popUp(entries, under: anchor.view) { open = false }
    }
}

private struct GroupsMenuButtonStyle: ButtonStyle {
    let open: Bool
    let hovering: Bool

    func makeBody(configuration: Configuration) -> some View {
        let fill = configuration.isPressed || open ? Theme.selected : hovering ? Theme.hover : .clear
        configuration.label
            .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(fill))
            .animation(Motion.standard(Motion.fade), value: fill)
    }
}

/// The 40-point round outlined "more" button of the headers (Qt's outlined IconButton with
/// radius 20): `raised` on hover, `selected` and the icon at 86 % while pressed. It pops up
/// `entries()` (built when it opens) right-aligned 4 points below it.
struct GroupsMoreButton: View {
    let label: String
    /// Qt: 18 on the collection page, 16 on the playlist page.
    var iconSize: CGFloat
    /// Qt: `text2` on the collection page, `text` on the playlist page.
    var tint: Color
    let entries: () -> [SourceMenuEntry]

    @State private var hovering = false
    @State private var anchor = SourceMenuAnchor()

    var body: some View {
        Button {
            let entries = entries()
            if !entries.isEmpty { SourceMenus.popUp(entries, under: anchor.view) }
        } label: {
            Image(systemName: "ellipsis")
                .font(.system(size: iconSize * 0.85, weight: .semibold))
                .foregroundStyle(tint)
                .frame(width: 40, height: 40)
                .contentShape(Circle())
        }
        .buttonStyle(GroupsMoreButtonStyle(hovering: hovering))
        .background(SourceMenuAnchorView(anchor: anchor))
        .fixedSize()
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
    }
}

private struct GroupsMoreButtonStyle: ButtonStyle {
    let hovering: Bool

    func makeBody(configuration: Configuration) -> some View {
        let fill = configuration.isPressed ? Theme.selected : hovering ? Theme.raised : .clear
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.86 : 1)
            .background(Circle().fill(fill))
            .overlay(Circle().strokeBorder(Theme.border, lineWidth: 1))
            .animation(Motion.standard(Motion.fade), value: fill)
            .animation(Motion.standard(Motion.move), value: configuration.isPressed)
    }
}

extension View {
    /// Reports this view's width to `width` as it changes.
    func groupsWidth(_ width: Binding<CGFloat>) -> some View {
        background(
            GeometryReader { geometry in
                Color.clear
                    .onAppear { width.wrappedValue = geometry.size.width }
                    .onChange(of: geometry.size.width) { _, new in width.wrappedValue = new }
            }
        )
    }

    /// Keeps `narrow` (Qt's narrow header: a 96-point cover, a 22-point name, no eyebrow,
    /// icon-only Shuffle and sort) up to date with the page's width. Qt's narrow class is a
    /// window under 720, where its rail (64) is gone: its pages keep the full header down to
    /// 656 points of page width, so ours switch below that. The filter drawer covers the page
    /// without narrowing it, so it never changes the header.
    func groupsNarrowHeader(_ narrow: Binding<Bool>) -> some View {
        modifier(GroupsNarrowHeader(narrow: narrow))
    }
}

private struct GroupsNarrowHeader: ViewModifier {
    @Binding var narrow: Bool
    @State private var width: CGFloat = 0

    func body(content: Content) -> some View {
        content
            .groupsWidth($width)
            .onChange(of: width) { update() }
    }

    private func update() {
        guard width > 0 else { return }
        let new = width < groupsNarrowHeader
        if narrow != new { narrow = new }
    }
}

/// Below this page width, collection and playlist headers take Qt's narrow form (see
/// `groupsNarrowHeader(_:)`).
let groupsNarrowHeader: CGFloat = 656

/// The headers' sort: with its label when it fits, else icon only.
struct GroupsSortButton: View {
    @Binding var sort: String
    var options: [SortOption] = SortOption.tracks
    let narrow: Bool

    var body: some View {
        if narrow {
            SortButton(sort: $sort, options: options, iconOnly: true)
        } else {
            ViewThatFits(in: .horizontal) {
                SortButton(sort: $sort, options: options, iconOnly: false)
                SortButton(sort: $sort, options: options, iconOnly: true)
            }
        }
    }
}
