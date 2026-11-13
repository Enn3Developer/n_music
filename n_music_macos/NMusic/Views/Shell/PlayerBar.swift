import SwiftUI

/// The player bar across the bottom of the window (spec §4.1): 88 points, `surface`, a 1-point
/// line on top. With `unit = (width − 88) / 3.6`: the current track on the left in `unit`, the
/// transport and seek bar centred in `1.6 × unit`, and mini player, queue, mute and volume
/// anchored right.
struct PlayerBar: View {
    var body: some View {
        GeometryReader { geometry in
            let unit = max(0, geometry.size.width - 88) / 3.6
            ZStack {
                HStack(spacing: 0) {
                    NowPlayingInfo()
                        .frame(width: max(0, unit - 20), alignment: .leading)
                        .padding(.leading, 20)
                    Spacer(minLength: 0)
                }
                Transport(width: 1.6 * unit)
                HStack(spacing: 0) {
                    Spacer(minLength: 0)
                    RightGroup()
                        .padding(.trailing, 20)
                }
            }
            .frame(width: geometry.size.width, height: geometry.size.height)
        }
        .frame(height: 88)
        .background(Theme.surface)
        .overlay(alignment: .top) { Rectangle().fill(Theme.line).frame(height: 1) }
    }
}

/// The cover (56, radius 6) and the title over "artist · album"; "Nothing playing" without a
/// track. The cover cross-fades and the words swap (rising, or falling when going back in the
/// queue); a change within 250 ms of the last is instant.
private struct NowPlayingInfo: View {
    private let store = Store.shared
    @State private var lastChange = ContinuousClock.now
    @State private var animate = true

    var body: some View {
        let track = store.current?.track
        let reduced = Motion.shared.reduced
        let rise: CGFloat = reduced ? 0 : (store.trackChangeBack ? -8 : 8)
        HStack(spacing: 14) {
            ZStack {
                CoverView(path: track?.cover, size: 56, radius: 6)
                    .id(track?.locator.key ?? "none")
                    .transition(.opacity)
            }
            .frame(width: 56, height: 56)
            ZStack(alignment: .leading) {
                VStack(alignment: .leading, spacing: 3) {
                    if let track {
                        Text(track.title)
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(Theme.text)
                        let second = track.artistAndAlbum
                        if !second.isEmpty {
                            Text(second)
                                .font(.system(size: 13))
                                .foregroundStyle(Theme.text2)
                        }
                    } else {
                        Text(tr("not_playing"))
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(Theme.text3)
                    }
                }
                .lineLimit(1)
                .truncationMode(.tail)
                .id(store.trackChange)
                .transition(.asymmetric(
                    insertion: .opacity.combined(with: .offset(y: rise)),
                    removal: .opacity.animation(Motion.leaving(Motion.exit))
                ))
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .clipped()
        }
        .animation(animate ? Motion.standard(Motion.move) : nil, value: store.trackChange)
        .onChange(of: store.trackChange) {
            let now = ContinuousClock.now
            animate = now - lastChange >= .milliseconds(250)
            lastChange = now
        }
        .accessibilityElement(children: .combine)
    }
}

/// Shuffle, previous, play/pause, next, repeat; under them the seek bar between the two clocks.
private struct Transport: View {
    let width: CGFloat
    private let store = Store.shared

    var body: some View {
        VStack(spacing: 6) {
            HStack(spacing: 10) {
                IconButton(
                    symbol: "shuffle", label: tr(store.shuffle ? "shuffle_on" : "shuffle_off"),
                    size: 36, iconSize: 18, tint: store.shuffle ? Theme.accentText : Theme.text2
                ) { store.toggleShuffle() }
                IconButton(symbol: "backward.end.fill", label: tr("previous"), size: 40, iconSize: 20, tint: Theme.text) {
                    store.previous()
                }
                PlayPauseButton(size: 44)
                IconButton(symbol: "forward.end.fill", label: tr("next"), size: 40, iconSize: 20, tint: Theme.text) {
                    store.next()
                }
                IconButton(
                    symbol: store.loopStatus.symbol, label: tr(store.loopStatus.labelKey),
                    size: 36, iconSize: 18, tint: store.loopStatus == .off ? Theme.text2 : Theme.accentText
                ) { store.cycleRepeat() }
            }
            SeekBar()
                .frame(width: min(width, 560))
        }
    }
}

/// The elapsed time (the dragged one while dragging), the position slider (seeks on release;
/// disabled without a track or a length) and the length. Clocks are floored.
private struct SeekBar: View {
    private let store = Store.shared
    @State private var dragged: Double?

    var body: some View {
        let length = store.length
        let enabled = store.current != nil && length > 0
        // The bar glides by itself (Core Animation) and the clock ticks once a second: nothing
        // here is rebuilt between position changes.
        HStack(spacing: 8) {
            PlayClockText(override: dragged.map { $0 * length })
                .frame(width: 36, alignment: .trailing)
            BarSlider(
                value: length > 0 ? store.basePosition / length : 0,
                label: tr("position"),
                valueText: Format.clock(store.basePosition),
                rate: store.playing && length > 0 ? 1 / length : 0,
                anchor: store.positionAt,
                onDrag: { dragged = $0 },
                onChange: { store.seek(to: $0 * length) }
            )
            .disabled(!enabled)
            Text(Format.clock(length))
                .frame(width: 36, alignment: .leading)
        }
        .font(.system(size: 12).monospacedDigit())
        .foregroundStyle(Theme.text2)
    }
}

/// Mini player, queue (accent while the queue shows or for 750 ms after a track was queued out
/// of sight), mute and the volume slider (110 points, knob on hover, live).
private struct RightGroup: View {
    private let store = Store.shared
    private let navigation = Navigation.shared
    @State private var pinging = false

    var body: some View {
        HStack(spacing: 6) {
            IconButton(symbol: "pip", label: tr("mini_player"), size: 36, iconSize: 18) {
                Windows.shared.showMiniPlayer()
            }
            IconButton(
                symbol: "text.line.first.and.arrowtriangle.forward", label: tr("queue"), size: 36, iconSize: 18,
                tint: navigation.section == PageID.queue || pinging ? Theme.accentText : Theme.text2
            ) { navigation.toggleQueue() }
            IconButton(
                symbol: store.muted ? "speaker.slash" : "speaker.wave.2",
                label: tr(store.muted ? "unmute" : "mute"), size: 36, iconSize: 18
            ) { store.toggleMute() }
            BarSlider(
                value: store.volume, fill: Theme.text2, knobOnHover: true, live: true,
                label: tr("volume"), onChange: { store.setVolume($0) }
            )
            .frame(width: 110)
        }
        .onChange(of: navigation.queuePing) {
            pinging = true
            Task {
                try? await Task.sleep(for: .milliseconds(750))
                pinging = false
            }
        }
        .animation(Motion.standard(Motion.fade), value: pinging)
    }
}
