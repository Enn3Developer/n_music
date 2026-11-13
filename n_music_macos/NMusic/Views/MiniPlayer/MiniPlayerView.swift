import SwiftUI

/// The mini player window's content (spec §4.4, brief §14); the window itself (420 × 128,
/// 360–640 wide, fixed height, floating with "Keep the mini player on top", close = expand,
/// titled "<title> – N Music") is `MiniPlayerWindow` in App/Windows.swift.
///
/// Margins 12, 14 apart: the 104-point cover (radius 8); beside it the title (15 semibold) over
/// the artist (13) with the 32-point Expand button at the top right, the seek row (11-point
/// clocks, the knob only on hover, seeking on release), and the transport row spread evenly
/// (Shuffle 32, Previous 36, Play/Pause 40, Next 36, Repeat 32). No volume, no queue. Space
/// plays and pauses (the app delegate). Background `surface`.
struct MiniPlayerView: View {
    private let store = Store.shared

    var body: some View {
        HStack(spacing: 14) {
            SwapCover(size: 104, radius: 8)
            VStack(alignment: .leading, spacing: 0) {
                HStack(alignment: .top, spacing: 6) {
                    SwapWords(store.current?.track) { track in
                        VStack(alignment: .leading, spacing: 2) {
                            if let track {
                                Text(track.title)
                                    .font(.system(size: 15, weight: .semibold))
                                    .foregroundStyle(Theme.text)
                                if !track.artist.isEmpty {
                                    Text(track.artist)
                                        .font(.system(size: 13))
                                        .foregroundStyle(Theme.text2)
                                }
                            } else {
                                Text(tr("not_playing"))
                                    .font(.system(size: 15, weight: .semibold))
                                    .foregroundStyle(Theme.text3)
                            }
                        }
                        .lineLimit(1)
                        .truncationMode(.tail)
                    }
                    IconButton(
                        symbol: "arrow.up.left.and.arrow.down.right", label: tr("full_window"),
                        size: 32, iconSize: 16
                    ) {
                        Windows.shared.expand()
                    }
                }
                Spacer(minLength: 0)
                MiniSeekRow()
                Spacer(minLength: 0)
                HStack(spacing: 0) {
                    IconButton(
                        symbol: "shuffle", label: tr(store.shuffle ? "shuffle_on" : "shuffle_off"),
                        size: 32, iconSize: 16, tint: store.shuffle ? Theme.accentText : Theme.text2
                    ) { store.toggleShuffle() }
                    Spacer(minLength: 0)
                    IconButton(symbol: "backward.end.fill", label: tr("previous"), size: 36, iconSize: 18, tint: Theme.text) {
                        store.previous()
                    }
                    Spacer(minLength: 0)
                    PlayPauseButton(size: 40)
                    Spacer(minLength: 0)
                    IconButton(symbol: "forward.end.fill", label: tr("next"), size: 36, iconSize: 18, tint: Theme.text) {
                        store.next()
                    }
                    Spacer(minLength: 0)
                    IconButton(
                        symbol: store.loopStatus.symbol, label: tr(store.loopStatus.labelKey),
                        size: 32, iconSize: 16, tint: store.loopStatus == .off ? Theme.text2 : Theme.accentText
                    ) { store.cycleRepeat() }
                }
            }
            .frame(maxHeight: .infinity)
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Theme.surface)
        .tint(Theme.accent)
    }
}

/// The elapsed time (the dragged one while dragging), the position slider (knob on hover,
/// seeks on release; disabled without a track or a length) and the length, in 11 points.
private struct MiniSeekRow: View {
    private let store = Store.shared
    @State private var dragged: Double?

    var body: some View {
        let length = store.length
        let enabled = store.current != nil && length > 0
        HStack(spacing: 8) {
            PlayClockText(override: dragged.map { $0 * length })
            BarSlider(
                value: length > 0 ? store.basePosition / length : 0,
                knobOnHover: true,
                label: tr("position"),
                valueText: Format.clock(store.basePosition),
                rate: store.playing && length > 0 ? 1 / length : 0,
                anchor: store.positionAt,
                onDrag: { dragged = $0 },
                onChange: { store.seek(to: $0 * length) }
            )
            .disabled(!enabled)
            Text(Format.clock(length.rounded()))
        }
        .font(.system(size: 11).monospacedDigit())
        .foregroundStyle(Theme.text2)
    }
}
