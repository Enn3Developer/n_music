import AppKit
import SwiftUI

/// A row of the queue page (Qt's QueueRow, spec §3.8) or of the Now Playing panel's Up next
/// (spec §4.3), inside `QueueTable`'s reused cells.
///
/// Page: a grip on the rows that drag (the current and upcoming ones), the cover (60 % on
/// played rows), title (accent when current, quieter when played) over the artist, the length,
/// and a 36-point action slot: "×" on a queued, not-playing item. The current row's ground and
/// bars are the table's playing highlight, which glides between rows; the row draws them itself
/// only with `ownGround` (while a row is dragged, rows making way carry it) or lifted.
/// Grounds: `surface` turning `raised` on hover for queued items, `hover` on hover for the rest;
/// a lifted row is `raised` with a border. Panel: cover, title, artist, and the length or
/// "QUEUED".
struct QueueRowContent: View {
    let mode: QueueTableController.Mode
    let row: QueueLayoutRow
    let compact: Bool
    /// Newly queued: tinted with the accent for 200 ms, fading over 400 ms.
    var tint: Bool
    var lifted = false
    /// The current row draws the playing ground and bars itself.
    var ownGround = false
    /// The cover already decoded, for the drag image (rendered at once, without loading).
    var liftedCover: Picture? = nil
    var onRemove: () -> Void

    var body: some View {
        Group {
            switch mode {
            case .page:
                PageRow(
                    row: row, compact: compact, tint: tint, lifted: lifted, ownGround: ownGround,
                    liftedCover: liftedCover, onRemove: onRemove
                )
            case .panel: PanelRow(row: row, compact: compact, tint: tint)
            }
        }
        // Reused cells show another item: its hover and tint start afresh.
        .id(row.entry.item)
    }
}

private struct PageRow: View {
    let row: QueueLayoutRow
    let compact: Bool
    let tint: Bool
    let lifted: Bool
    let ownGround: Bool
    let liftedCover: Picture?
    let onRemove: () -> Void

    private let store = Store.shared
    @State private var hovering = false
    @State private var tinted = 0.0

    var body: some View {
        let track = row.entry.track
        let history = row.history
        let current = row.current
        HStack(spacing: 0) {
            GripDots(highlighted: hovering || lifted)
                .frame(width: 16, height: 16)
                .padding(.leading, 4)
                .opacity(history ? 0 : 1)
            Group {
                if let liftedCover {
                    Image(decorative: liftedCover.image, scale: 1)
                        .resizable()
                        .aspectRatio(contentMode: .fill)
                        .frame(width: compact ? 32 : 40, height: compact ? 32 : 40)
                        .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
                } else {
                    CoverView(path: track.cover, size: compact ? 32 : 40, radius: 4)
                }
            }
            .opacity(history ? 0.6 : 1)
            .padding(.leading, 4)
            VStack(alignment: .leading, spacing: compact ? 0 : 2) {
                Text(track.title)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(current ? Theme.accentText : history ? Theme.text2 : Theme.text)
                Text(track.artist.isEmpty ? tr("unknown_artist") : track.artist)
                    .font(.system(size: compact ? 12 : 13))
                    .foregroundStyle(track.artist.isEmpty ? Theme.text3 : Theme.text2)
            }
            .lineLimit(1)
            .truncationMode(.tail)
            .padding(.leading, 12)
            Spacer(minLength: 12)
            Text(Format.length(track.length))
                .font(.system(size: 13).monospacedDigit())
                .foregroundStyle(Theme.text2)
                .frame(width: 52, alignment: .trailing)
            ZStack {
                if current {
                    if ownGround || lifted {
                        EqualizerBars(playing: store.playing, size: 14, color: Theme.accentText)
                    }
                    Color.clear.frame(width: 14, height: 14)
                        .accessibilityLabel(tr("now_playing"))
                } else if row.entry.queued {
                    IconButton(
                        symbol: "xmark", label: tr("remove_from_up_next", track.title),
                        size: 32, iconSize: 14, tint: Theme.text3, action: onRemove
                    )
                }
            }
            .frame(width: 36)
            .padding(.leading, 12)
            .padding(.trailing, 8)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(ground)
                .overlay {
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .strokeBorder(lifted ? Theme.line2 : .clear, lineWidth: 1)
                }
                .overlay {
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .fill(Theme.accent.opacity(0.12 * tinted))
                }
        }
        .padding(.vertical, 3)
        .contentShape(Rectangle())
        .onHover { hovering = $0 }
        .animation(Motion.standard(Motion.fade), value: hovering)
        .animation(Motion.standard(Motion.fade), value: history)
        .animation(Motion.standard(Motion.fade), value: current)
        .onAppear(perform: flash)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(current ? .isSelected : [])
    }

    private var ground: Color {
        if lifted { return Theme.raised }
        if row.current { return ownGround ? Theme.selected : .clear }
        if row.entry.queued { return hovering ? Theme.raised : Theme.surface }
        return hovering ? Theme.hover : .clear
    }

    private func flash() {
        guard tint else { return }
        tinted = 1
        withAnimation(.linear(duration: 0.4).delay(Motion.move)) { tinted = 0 }
    }
}

private struct PanelRow: View {
    let row: QueueLayoutRow
    let compact: Bool
    let tint: Bool

    @State private var hovering = false
    @State private var tinted = 0.0

    var body: some View {
        let track = row.entry.track
        HStack(spacing: 10) {
            CoverView(path: track.cover, size: compact ? 32 : 40, radius: 4)
            VStack(alignment: .leading, spacing: compact ? 0 : 2) {
                Text(track.title)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(Theme.text)
                Text(track.artist.isEmpty ? tr("unknown_artist") : track.artist)
                    .font(.system(size: 12))
                    .foregroundStyle(Theme.text2)
            }
            .lineLimit(1)
            .truncationMode(.tail)
            Spacer(minLength: 0)
            if row.entry.queued {
                Text(tr("queued").uppercased())
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(Theme.accentText)
            } else {
                Text(Format.length(track.length))
                    .font(.system(size: 12).monospacedDigit())
                    .foregroundStyle(Theme.text3)
            }
        }
        .padding(.horizontal, 6)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(hovering ? Theme.hover : .clear)
                .overlay {
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .fill(Theme.accent.opacity(0.12 * tinted))
                }
        }
        .padding(.vertical, 1)
        .contentShape(Rectangle())
        .onHover { hovering = $0 }
        .animation(Motion.standard(Motion.fade), value: hovering)
        .onAppear {
            guard tint else { return }
            tinted = 1
            withAnimation(.linear(duration: 0.4).delay(Motion.move)) { tinted = 0 }
        }
        .accessibilityElement(children: .combine)
    }
}

/// Qt's grip icon: two columns of three dots. The pointer over it is an open hand (the cell's
/// cursor rect, `QueueTable`).
private struct GripDots: View {
    var highlighted: Bool

    var body: some View {
        Canvas { context, size in
            let dot: CGFloat = 2.6
            let columns: [CGFloat] = [size.width / 2 - 3, size.width / 2 + 3]
            let rows: [CGFloat] = [size.height / 2 - 5, size.height / 2, size.height / 2 + 5]
            for x in columns {
                for y in rows {
                    context.fill(
                        Path(ellipseIn: CGRect(x: x - dot / 2, y: y - dot / 2, width: dot, height: dot)),
                        with: .foreground
                    )
                }
            }
        }
        .foregroundStyle(highlighted ? Theme.text2 : Theme.text3)
        .accessibilityHidden(true)
    }
}
