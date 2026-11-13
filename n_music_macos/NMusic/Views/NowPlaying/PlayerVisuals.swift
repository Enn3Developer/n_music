import AppKit
import Observation
import SwiftUI

// The shared playback visuals of the queue page, the Now Playing panel and the mini player
// (spec §4.2): the cover that cross-fades, the words that swap, the bars, the chips.

/// The current track's cover (Qt's CoverSwap): once the new cover has decoded it fades in over
/// 200 ms on top of the old one (150 ms under reduced motion), which stays whole under it; a
/// change within 250 ms of the one before swaps at once. A cover slow to decode shows anyway
/// after 300 ms, as a bare tile its picture comes into. Without a cover: the app's monochrome
/// logo. Large covers (`full`) then sharpen to the picture in the file, decoded at the shown size.
///
/// Nothing loads while `active` is false (a page kept alive but not shown) or while its window
/// is hidden or minimized.
struct SwapCover: View {
    let size: CGFloat
    var radius: CGFloat = 12
    /// Reads the picture in the file at full quality once the library's thumbnail shows.
    var full = false
    var active = true

    private let store = Store.shared
    @Environment(\.displayScale) private var scale
    @State private var layers: [Layer] = []
    @State private var lastChange: ContinuousClock.Instant?
    @State private var shownLocator: String?
    @State private var serial = 0
    @State private var windowShown = true
    /// When it last became able to load: what comes then shows at once.
    @State private var wokeAt: ContinuousClock.Instant?

    private struct Layer: Identifiable {
        let id: Int
        var picture: Picture?
        /// No cover to show (rather than one still decoding).
        var missing: Bool
    }

    private struct Key: Equatable {
        let locator: String?
        let cover: String?
        let pixels: Int
        let live: Bool
    }

    private enum Outcome: Sendable {
        case decoded(Picture?)
        case late
    }

    var body: some View {
        let track = store.current?.track
        let live = active && windowShown
        ZStack {
            Rectangle().fill(Theme.raised)
            ForEach(layers) { layer in
                ZStack {
                    Rectangle().fill(Theme.raised)
                    if let picture = layer.picture {
                        Image(decorative: picture.image, scale: 1)
                            .resizable()
                            .interpolation(.high)
                            .aspectRatio(contentMode: .fill)
                    } else if layer.missing {
                        MonoLogo(size: size)
                    }
                }
                .frame(width: size, height: size)
                .transition(.opacity)
            }
        }
        .frame(width: size, height: size)
        .clipShape(RoundedRectangle(cornerRadius: radius, style: .continuous))
        .background(HostWindowShown(shown: $windowShown))
        .accessibilityHidden(true)
        .onChange(of: live) { _, live in
            if live { wokeAt = .now }
        }
        .task(id: Key(locator: track?.locator.key, cover: track?.cover, pixels: pixels, live: live)) {
            guard live else { return }
            await load(track)
        }
    }

    /// Rounded up to 64 pixels, so a window resize decodes again only now and then.
    private var pixels: Int {
        let exact = Int((size * scale).rounded(.up))
        return (exact + 63) / 64 * 64
    }

    private func load(_ track: TrackRow?) async {
        let locator = track?.locator.key
        let sameTrack = locator == shownLocator && !layers.isEmpty
        if sameTrack {
            // Only the size changed (or the cover was read): settle before decoding again.
            try? await Task.sleep(for: .milliseconds(150))
            guard !Task.isCancelled else { return }
        }
        let pixels = self.pixels
        let path = track?.cover
        var picture = path.flatMap { Artwork.shared.cached(path: $0, pixels: pixels) }
        if sameTrack {
            if picture == nil, let path {
                picture = await Artwork.shared.thumbnail(path: path, pixels: pixels)
                guard !Task.isCancelled else { return }
            }
            if var top = layers.last, picture != nil || path == nil {
                top.picture = picture ?? top.picture
                top.missing = path == nil || (top.picture == nil && top.missing)
                replaceTop(top)
            }
        } else if picture != nil || path == nil {
            show(Layer(id: next(), picture: picture, missing: path == nil), locator: locator)
        } else if let path {
            // Decodes; past 300 ms the new cover shows anyway and its picture comes into it.
            let decoding = Task { await Artwork.shared.thumbnail(path: path, pixels: pixels) }
            let first = await withTaskCancellationHandler {
                await withTaskGroup(of: Outcome.self) { group in
                    group.addTask { .decoded(await decoding.value) }
                    group.addTask {
                        try? await Task.sleep(for: .milliseconds(300))
                        return .late
                    }
                    let first = await group.next() ?? .late
                    group.cancelAll()
                    return first
                }
            } onCancel: {
                decoding.cancel()
            }
            guard !Task.isCancelled else { return }
            switch first {
            case .decoded(let decoded):
                picture = decoded
                show(Layer(id: next(), picture: decoded, missing: decoded == nil), locator: locator)
            case .late:
                let id = next()
                show(Layer(id: id, picture: nil, missing: false), locator: locator)
                let decoded = await decoding.value
                guard !Task.isCancelled, var top = layers.last, top.id == id else { return }
                picture = decoded
                top.picture = decoded
                top.missing = decoded == nil
                replaceTop(top)
            }
        }
        guard full, let track, track.cover != nil, picture != nil else { return }
        let sharp = await Artwork.shared.full(track.locator, pixels: pixels, core: store.core)
        guard !Task.isCancelled, let sharp, var top = layers.last else { return }
        top.picture = sharp
        replaceTop(top)
    }

    private func next() -> Int {
        serial += 1
        return serial
    }

    private func replaceTop(_ layer: Layer) {
        var instant = Transaction()
        instant.disablesAnimations = true
        withTransaction(instant) {
            if !layers.isEmpty { layers[layers.count - 1] = layer }
        }
    }

    private func show(_ layer: Layer, locator: String?) {
        let now = ContinuousClock.now
        let quick = lastChange.map { now - $0 < .milliseconds(250) } ?? true
        // Back on screen: what changed meanwhile shows at once.
        let woke = wokeAt.map { now - $0 < .milliseconds(400) } ?? false
        lastChange = now
        shownLocator = locator
        var instant = Transaction()
        instant.disablesAnimations = true
        if quick || woke || layers.isEmpty {
            withTransaction(instant) { layers = [layer] }
            return
        }
        let duration = Motion.shared.reduced ? Motion.fade : Motion.move
        withAnimation(Motion.standard(duration)) { layers.append(layer) }
        let id = layer.id
        Task { @MainActor in
            try? await Task.sleep(for: .seconds(duration))
            // The old cover goes once covered, without fading.
            withTransaction(instant) {
                if let index = layers.firstIndex(where: { $0.id == id }), index > 0 {
                    layers.removeFirst(index)
                }
            }
        }
    }
}

/// Words about the current track that fade through when it changes (Qt's TrackSwap), so two
/// titles never share a frame: the old words, frozen as they were, go in 80 ms, then the new
/// ones fade in over 150 ms rising 8 points over 200 ms (coming down when going back in the
/// queue). A change within 250 ms of the one before is instant; under reduced motion they only
/// fade. `value` is what the words show (the track, its facts); `content` lays it out. A new
/// value for the same track (facts read later) changes the words in place.
struct SwapWords<Value: Equatable, Content: View>: View {
    let value: Value
    @ViewBuilder var content: (Value) -> Content

    private let store = Store.shared
    @State private var shown = Store.shared.trackChange
    /// What the words in front show; the leaving ones keep their own.
    @State private var held: Value
    @State private var lastChange: ContinuousClock.Instant?

    init(_ value: Value, @ViewBuilder content: @escaping (Value) -> Content) {
        self.value = value
        self.content = content
        _held = State(initialValue: value)
    }

    private struct Key: Equatable {
        let change: Int
        let value: Value
    }

    var body: some View {
        let rise: CGFloat = Motion.shared.reduced ? 0 : (store.trackChangeBack ? -8 : 8)
        ZStack(alignment: .topLeading) {
            content(held)
                .frame(maxWidth: .infinity, alignment: .leading)
                .id(shown)
                .transition(.asymmetric(
                    insertion: .opacity.animation(Motion.standard(Motion.fade).delay(Motion.exit))
                        .combined(with: .offset(y: rise).animation(Motion.standard(Motion.move).delay(Motion.exit))),
                    removal: .opacity.animation(Motion.leaving(Motion.exit))
                ))
        }
        .frame(maxWidth: .infinity, alignment: .topLeading)
        .onChange(of: Key(change: store.trackChange, value: value)) { old, new in
            var instant = Transaction()
            instant.disablesAnimations = true
            guard new.change != old.change else {
                withTransaction(instant) { held = new.value }
                return
            }
            let now = ContinuousClock.now
            let quick = lastChange.map { now - $0 < .milliseconds(250) } ?? false
            lastChange = now
            if quick {
                withTransaction(instant) {
                    held = new.value
                    shown = new.change
                }
            } else {
                withAnimation(Motion.standard(Motion.move)) {
                    held = new.value
                    shown = new.change
                }
            }
        }
    }
}

/// The playing bars (Qt's PlayingBars): three bars that each ease to a random height every
/// 180–320 ms while playing, and settle to 10/16/12 over 200 ms when paused. They hold still
/// unless seen (`visible`: on the page shown; their window neither hidden nor minimized), under
/// reduced motion, and once the window has been inactive for 10 s. Nothing runs while they hold
/// still.
struct EqualizerBars: View {
    var playing: Bool
    var size: CGFloat = 14
    var color: Color = Theme.accentText
    var visible = true

    @Environment(\.controlActiveState) private var activeState
    @State private var idle = false
    @State private var windowShown = true

    var body: some View {
        let moving = playing && visible && windowShown && !idle && !Motion.shared.reduced
        let unit = size / 24
        ZStack(alignment: .topLeading) {
            EqualizerBar(at: 4, rest: 10, unit: unit, moving: moving)
            EqualizerBar(at: 10.25, rest: 16, unit: unit, moving: moving)
            EqualizerBar(at: 16.5, rest: 12, unit: unit, moving: moving)
        }
        .foregroundStyle(color)
        .frame(width: size, height: size, alignment: .topLeading)
        .background(HostWindowShown(shown: $windowShown))
        .task(id: activeState == .inactive) {
            guard activeState == .inactive else {
                idle = false
                return
            }
            try? await Task.sleep(for: .seconds(10))
            if !Task.isCancelled { idle = true }
        }
    }
}

private struct EqualizerBar: View {
    let at: CGFloat
    let rest: CGFloat
    let unit: CGFloat
    let moving: Bool

    @State private var tall: CGFloat

    init(at: CGFloat, rest: CGFloat, unit: CGFloat, moving: Bool) {
        self.at = at
        self.rest = rest
        self.unit = unit
        self.moving = moving
        _tall = State(initialValue: rest)
    }

    var body: some View {
        Rectangle()
            .frame(width: 3.5 * unit, height: tall * unit)
            .offset(x: at * unit, y: (20 - tall) * unit)
            .task(id: moving) {
                guard moving else {
                    withAnimation(Motion.standard(Motion.move)) { tall = rest }
                    return
                }
                while !Task.isCancelled {
                    let duration = 0.18 + Double.random(in: 0...0.14)
                    withAnimation(.easeInOut(duration: duration)) { tall = 4 + CGFloat.random(in: 0...11) }
                    try? await Task.sleep(for: .seconds(duration))
                }
            }
    }
}

/// Tells whether the window hosting it is on screen: neither ordered out (the main window
/// while the mini player shows) nor minimized (Qt's `Window.visibility`).
struct HostWindowShown: NSViewRepresentable {
    @Binding var shown: Bool

    func makeNSView(context: Context) -> WindowWatcher {
        let view = WindowWatcher()
        view.onChange = { shown in
            if self.shown != shown { self.shown = shown }
        }
        return view
    }

    func updateNSView(_ view: WindowWatcher, context: Context) {
        view.onChange = { shown in
            if self.shown != shown { self.shown = shown }
        }
    }

    final class WindowWatcher: NSView {
        var onChange: ((Bool) -> Void)?

        override func hitTest(_ point: NSPoint) -> NSView? { nil }

        override func viewWillMove(toWindow newWindow: NSWindow?) {
            super.viewWillMove(toWindow: newWindow)
            NotificationCenter.default.removeObserver(self)
        }

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            guard let window else { return }
            for name in [
                NSWindow.didChangeOcclusionStateNotification, NSWindow.didMiniaturizeNotification,
                NSWindow.didDeminiaturizeNotification,
            ] {
                NotificationCenter.default.addObserver(self, selector: #selector(changed), name: name, object: window)
            }
            // A window not ordered in yet tells nothing: ordering it in may post no occlusion
            // change (behind other windows), and it starts out shown.
            if window.isVisible { changed() }
        }

        @objc private func changed() {
            let shown = window.map { $0.isVisible && !$0.isMiniaturized } ?? false
            // Not while SwiftUI updates the view that holds it.
            DispatchQueue.main.async { [weak self] in
                self?.onChange?(shown)
            }
        }
    }
}

/// A small rounded fact, like the audio format (Qt's Chip): 26 points, `field` with a `line2`
/// border, 12-point `text2`.
struct FactChip: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.system(size: 12))
            .foregroundStyle(Theme.text2)
            .lineLimit(1)
            .padding(.horizontal, 10)
            .frame(height: 26)
            .background(Capsule().fill(Theme.field))
            .overlay(Capsule().strokeBorder(Theme.line2, lineWidth: 1))
    }
}

/// An underlined text link, `text2` turning `text` on hover ("Open queue", "Open in library").
struct UnderlinedLink: View {
    let title: String
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Text(title)
                .font(.system(size: 13))
                .underline()
                .foregroundStyle(hovering ? Theme.text : Theme.text2)
                .frame(minHeight: 32)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .animation(Motion.standard(Motion.fade), value: hovering)
        .accessibilityAddTraits(.isLink)
    }
}

/// Qt's FlatButton: a quiet 32-point text button, `raised` on hover, `selected` pressed.
struct QuietButtonStyle: ButtonStyle {
    @State private var hovering = false
    @Environment(\.isEnabled) private var enabled

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(hovering ? Theme.text : Theme.text2)
            .padding(.horizontal, 12)
            .frame(height: 32)
            .background(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(configuration.isPressed ? Theme.selected : hovering ? Theme.raised : .clear)
            )
            .contentShape(Rectangle())
            .opacity(enabled ? 1 : 0.45)
            .onHover { hovering = $0 && enabled }
            .animation(Motion.standard(Motion.fade), value: hovering)
    }
}

/// What the library knows of the current track beyond its row: the audio format, ReplayGain
/// and plays (the queue page's chips, the panel's audio line). Read off the main thread when
/// the track or the library changes.
///
/// `Store.details` is `nil` from a track change until the new track's are read; until then the
/// track's library entry (read for its locator) tells, and nothing shows before it came.
@MainActor
@Observable
final class CurrentFacts {
    static let shared = CurrentFacts()

    private(set) var entry: TrackEntry?
    @ObservationIgnored private var loaded: String?
    @ObservationIgnored private var task: Task<Void, Never>?

    private init() {}

    /// Reads the current track's entry again when it or `version` changed.
    func refresh() {
        let store = Store.shared
        let locator = store.current?.track.locator
        let key = "\(locator?.key ?? "")\u{1}\(store.libraryVersion)"
        guard key != loaded else { return }
        loaded = key
        task?.cancel()
        guard let locator else {
            entry = nil
            return
        }
        if entry?.track.locator != locator { entry = nil }
        task = Task {
            let read = await store.trackEntry(locator)
            guard !Task.isCancelled else { return }
            entry = read
        }
    }

    /// The decoder's facts, once read for the current track.
    private var details: TrackDetails? { Store.shared.details }

    /// The library's entry, when it is the current track's.
    private var currentEntry: TrackEntry? {
        guard let entry, entry.track.locator == Store.shared.current?.track.locator else { return nil }
        return entry
    }

    /// Codec, sample rate and bit depth, like `FLAC · 44.1 kHz · 16-bit`.
    var audio: String {
        if let details {
            return Format.audio(
                codec: details.codec?.uppercased(), sampleRate: details.sampleRate, bits: details.bitsPerSample
            )
        }
        guard let entry = currentEntry else { return "" }
        return Format.audio(codec: entry.format, sampleRate: entry.sampleRate, bits: entry.bitsPerSample)
    }

    /// Like `ReplayGain album −6.2 dB`; empty without gains.
    var gain: String {
        guard let entry = currentEntry else { return "" }
        return Format.gain(album: entry.albumGain, track: entry.trackGain)
    }

    var plays: UInt32 {
        details?.plays ?? currentEntry?.track.plays ?? Store.shared.current?.track.plays ?? 0
    }
}

extension TrackRow {
    /// "artist · album · year", empty parts dropped.
    var artistAlbumYear: String {
        [artist, album ?? "", year.map(String.init) ?? ""].filter { !$0.isEmpty }.joined(separator: " · ")
    }
}
