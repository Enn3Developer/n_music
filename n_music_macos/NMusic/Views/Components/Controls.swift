import SwiftUI

/// The Qt app's IconButton: a square button with an SF Symbol, a hover tile (`raised`), the
/// icon shrinking to 86 % while pressed, 45 % opacity when disabled. `label` is its tooltip and
/// accessible name (brief §17: tooltips on every icon button).
struct IconButton: View {
    let symbol: String
    let label: String
    var size: CGFloat = 40
    var iconSize: CGFloat = 20
    var tint: Color = Theme.text2
    var action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: iconSize * 0.8, weight: .medium))
                .foregroundStyle(tint)
                .frame(width: size, height: size)
                .contentShape(Rectangle())
        }
        .buttonStyle(IconButtonStyle(radius: size >= 40 ? 8 : 6))
        .help(label)
        .accessibilityLabel(label)
    }
}

struct IconButtonStyle: ButtonStyle {
    var radius: CGFloat = 8
    @Environment(\.isEnabled) private var enabled
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.86 : 1)
            .background(
                RoundedRectangle(cornerRadius: radius, style: .continuous)
                    .fill(configuration.isPressed ? Theme.selected : hovering ? Theme.raised : .clear)
            )
            .opacity(enabled ? 1 : 0.45)
            .onHover { hovering = $0 && enabled }
            .animation(Motion.standard(Motion.move), value: configuration.isPressed)
    }
}

/// The Qt app's PillButton: fully rounded, 40 points tall (36 `small`); primary is the accent
/// fill with `accentInk` content, danger the red pill, plain an outline.
struct PillButtonStyle: ButtonStyle {
    enum Kind { case primary, plain, danger }
    var kind: Kind = .plain
    var small = false

    @Environment(\.isEnabled) private var enabled
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: small ? 13 : 14, weight: .semibold))
            .padding(.horizontal, kind == .primary ? 18 : small ? 14 : 16)
            .frame(height: small ? 36 : 40)
            .foregroundStyle(foreground)
            .background(PillOutline().fill(fill(pressed: configuration.isPressed)))
            .overlay(PillOutline().strokeBorder(stroke, lineWidth: kind == .primary ? 0 : 1))
            .scaleEffect(configuration.isPressed ? 0.95 : 1)
            .opacity(enabled ? 1 : 0.45)
            .onHover { hovering = $0 && enabled }
            .animation(Motion.standard(Motion.move), value: configuration.isPressed)
    }

    private var foreground: Color {
        switch kind {
        case .primary: Theme.accentInk
        case .plain: Theme.text
        case .danger: Theme.danger
        }
    }

    private var stroke: Color {
        switch kind {
        case .primary: .clear
        case .plain: Theme.border
        case .danger: Theme.dangerLine
        }
    }

    private func fill(pressed: Bool) -> Color {
        switch kind {
        case .primary:
            Theme.accent.opacity(pressed ? 0.85 : 1).mix(hovering && !pressed ? 0.08 : 0)
        case .plain: pressed ? Theme.selected : hovering ? Theme.hover : .clear
        case .danger: Theme.dangerBg
        }
    }
}

/// A fully rounded rectangle (radius half the shorter side) for pill outlines: `Capsule`'s
/// `strokeBorder` draws short ticks where its round ends meet the straight sides.
struct PillOutline: InsettableShape {
    var inset: CGFloat = 0

    func path(in rect: CGRect) -> Path {
        let rect = rect.insetBy(dx: inset, dy: inset)
        return RoundedRectangle(cornerRadius: min(rect.width, rect.height) / 2, style: .circular).path(in: rect)
    }

    func inset(by amount: CGFloat) -> PillOutline {
        var shape = self
        shape.inset += amount
        return shape
    }
}

private extension Color {
    /// This colour lightened toward white by `amount` (0…1), for hover tints.
    func mix(_ amount: Double) -> Color {
        guard amount > 0 else { return self }
        return Color(nsColor: NSColor(self).blended(withFraction: amount, of: .white) ?? NSColor(self))
    }
}

/// The Qt app's BarSlider: a 4-point rounded track filled from the left with `fill`, and a
/// 12-point knob (shown only on hover, drag or focus with `knobOnHover`).
///
/// - `live`: `onChange` runs as it drags (volume). Otherwise it runs once on release (seek),
///   while `onDrag` reports the value being dragged to (`nil` once released), so a clock can
///   show it. Keyboard arrows (when focused) change it at once either way.
struct BarSlider: View {
    /// 0…1.
    var value: Double
    var fill: Color = Theme.text
    var knobOnHover = false
    var live = false
    var step = 0.05
    var label: String
    var valueText: String? = nil
    /// Fraction of the track per second from `anchor` (the playback position), so the bar
    /// moves by itself between changes; 0 for a value that only changes when set.
    var rate: Double = 0
    var anchor: ContinuousClock.Instant = .now
    var onDrag: (Double?) -> Void = { _ in }
    var onChange: (Double) -> Void

    @Environment(\.isEnabled) private var enabled
    @State private var dragging: Double?
    @State private var hovering = false
    @FocusState private var focused: Bool

    var body: some View {
        GeometryReader { geometry in
            let width = max(1, geometry.size.width)
            let shown = min(max(dragging ?? value, 0), 1)
            BarTrack(
                value: shown,
                rate: dragging == nil ? rate : 0,
                anchor: anchor,
                fill: fill,
                track: Theme.track,
                knob: Theme.text,
                ring: Theme.accent,
                showKnob: enabled && (!knobOnHover || hovering || dragging != nil || focused),
                focused: focused
            )
            .frame(height: 16)
            .frame(maxHeight: .infinity)
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { gesture in
                        guard enabled else { return }
                        let fraction = min(max(gesture.location.x / width, 0), 1)
                        dragging = fraction
                        onDrag(fraction)
                        if live { onChange(fraction) }
                    }
                    .onEnded { gesture in
                        guard enabled else { return }
                        let fraction = min(max(gesture.location.x / width, 0), 1)
                        dragging = nil
                        onDrag(nil)
                        onChange(fraction)
                    }
            )
        }
        .frame(height: 16)
        .opacity(enabled ? 1 : 0.45)
        .onHover { hovering = $0 }
        // Keyboard focus only with the Mac's keyboard navigation on, as NSSlider: otherwise it
        // would take a window's first focus (the mini player's, or the main window's when the
        // filter drawer opens), ringing its knob.
        .modifier(SliderFocus(enabled: enabled && NSApp.isFullKeyboardAccessEnabled, focused: $focused) { direction in
            switch direction {
            case .left, .down: onChange(max(0, value - step))
            case .right, .up: onChange(min(1, value + step))
            default: break
            }
        })
        .accessibilityElement()
        .accessibilityLabel(label)
        .accessibilityValue(valueText ?? "\(Int((value * 100).rounded())) %")
        .accessibilityAdjustableAction { direction in
            switch direction {
            case .increment: onChange(min(1, value + step))
            case .decrement: onChange(max(0, value - step))
            @unknown default: break
            }
        }
    }
}

/// A slider's keyboard focus, only while it may have it: a `.focused` binding on a view that
/// can't be focused still turns true when its window's focus lands nowhere else.
private struct SliderFocus: ViewModifier {
    let enabled: Bool
    var focused: FocusState<Bool>.Binding
    let move: (MoveCommandDirection) -> Void

    func body(content: Content) -> some View {
        if enabled {
            content
                .focusable()
                .focusEffectDisabled()
                .focused(focused)
                .onMoveCommand(perform: move)
        } else {
            content
        }
    }
}

/// The player's Play/Pause: a `size` circle in `text` with the icon in `surface`; 2-point accent
/// ring on keyboard focus; pressed it scales to 90 %. Pauses, or plays the current track, or the
/// library (`Store.togglePause`).
struct PlayPauseButton: View {
    var size: CGFloat = 44
    private let store = Store.shared

    var body: some View {
        let label = tr(store.playing ? "pause" : "play")
        Button { store.togglePause() } label: {
            Image(systemName: store.playing ? "pause.fill" : "play.fill")
                .font(.system(size: size * 0.36, weight: .bold))
                .contentTransition(.symbolEffect(.replace))
                .foregroundStyle(Theme.surface)
                .offset(x: store.playing ? 0 : size * 0.03)
                .frame(width: size, height: size)
                .background(Circle().fill(Theme.text))
                .contentShape(Circle())
        }
        .buttonStyle(PressScaleStyle(scale: 0.9))
        .help(label)
        .accessibilityLabel(label)
    }
}

/// Scales the label while pressed.
struct PressScaleStyle: ButtonStyle {
    var scale: CGFloat = 0.95

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? scale : 1)
            .animation(Motion.standard(Motion.move), value: configuration.isPressed)
    }
}

/// The playback position as a floored clock. While playing (and a window is on screen) it is
/// redrawn exactly when the shown second changes, once a second; otherwise only when the
/// position is reported or sought.
struct PlayClockText: View {
    /// Shown instead of the position, as while dragging the slider.
    var override: Double? = nil
    private let store = Store.shared

    var body: some View {
        if let override {
            Text(Format.clock(override))
        } else if store.playing && store.windowVisible {
            TimelineView(SecondTicks(base: store.basePosition, at: store.positionAt)) { _ in
                Text(Format.clock(store.position()))
            }
        } else {
            Text(Format.clock(store.position()))
        }
    }
}

/// The instants a position that was `base` at `at` crosses each whole second.
private struct SecondTicks: TimelineSchedule {
    let base: Double
    let at: ContinuousClock.Instant

    func entries(from startDate: Date, mode: TimelineScheduleMode) -> AnyIterator<Date> {
        let nowClock = ContinuousClock.now
        let nowDate = Date()
        let elapsed = nowClock - at
        let atDate = nowDate.addingTimeInterval(
            -(Double(elapsed.components.seconds) + Double(elapsed.components.attoseconds) / 1e18)
        )
        // A hair past the boundary, so flooring never shows the second before.
        var second = floor(base) + 1
        var first = true
        return AnyIterator {
            if first {
                first = false
                return startDate
            }
            var next = atDate.addingTimeInterval(second - base + 0.01)
            while next <= startDate {
                second += 1
                next = atDate.addingTimeInterval(second - base + 0.01)
            }
            second += 1
            return next
        }
    }
}

/// The Qt app's EmptyState (spec §9.1): a 64-point `raised` circle with a 28-point icon, a
/// title, an optional message and an optional primary pill, 360 points wide, centred 40 points
/// above the middle.
struct EmptyStateView: View {
    let symbol: String
    let title: String
    var message: String? = nil
    var buttonTitle: String? = nil
    var action: (() -> Void)? = nil

    var body: some View {
        VStack(spacing: 0) {
            Image(systemName: symbol)
                .font(.system(size: 24, weight: .medium))
                .foregroundStyle(Theme.text2)
                .frame(width: 64, height: 64)
                .background(Circle().fill(Theme.raised))
            Text(title)
                .font(.system(size: 17, weight: .semibold))
                .foregroundStyle(Theme.text)
                .multilineTextAlignment(.center)
                .padding(.top, 18)
            if let message {
                Text(message)
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.text2)
                    .multilineTextAlignment(.center)
                    .padding(.top, 6)
            }
            if let buttonTitle, let action {
                Button(buttonTitle, action: action)
                    .buttonStyle(PillButtonStyle(kind: .primary))
                    .padding(.top, 20)
            }
        }
        .frame(width: 360)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .offset(y: -40)
    }
}

/// An upper-case group header, as the sidebar's LIBRARY and PLAYLISTS.
struct SectionHeader: View {
    let text: String

    var body: some View {
        Text(text.uppercased())
            .font(.system(size: 11, weight: .semibold))
            .tracking(0.9)
            .foregroundStyle(Theme.text3)
            .accessibilityAddTraits(.isHeader)
    }
}
