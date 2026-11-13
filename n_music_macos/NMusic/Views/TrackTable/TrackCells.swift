import AppKit
import SwiftUI

/// A track row's ground (spec §3.2, Row; `qml/TrackRow.qml:38-41`), in priority order:
/// nothing while it plays (the table's playing highlight shows under it), `raised` while
/// selected, `selected` while its menu is open, `hover` under the pointer. Rounded 8, inset
/// from the table's sides (16, 10 in compact windows). Colour changes fade as Qt's TintFade:
/// in over 80 ms, out over 150 ms.
final class TrackRowView: NSTableRowView {
    var playing = false {
        didSet { if oldValue != playing { updateGround(animated: false) } }
    }
    /// The row's menu is open.
    var menuOpen = false {
        didSet { if oldValue != menuOpen { updateGround(animated: true) } }
    }
    var inset: CGFloat = 16 {
        didSet { if oldValue != inset { needsLayout = true } }
    }
    private var hovering = false {
        didSet { if oldValue != hovering { updateGround(animated: true) } }
    }
    private var tracking: NSTrackingArea?
    private let ground = CALayer()
    private var shownColor: Color?

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        ground.cornerRadius = 8
        layer?.insertSublayer(ground, at: 0)
    }

    required init?(coder: NSCoder) { fatalError() }

    override var interiorBackgroundStyle: NSView.BackgroundStyle { .normal }
    override var isEmphasized: Bool {
        get { false }
        set {}
    }
    override var isSelected: Bool {
        didSet { if oldValue != isSelected { updateGround(animated: true) } }
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let tracking { removeTrackingArea(tracking) }
        let area = NSTrackingArea(rect: bounds, options: [.mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect], owner: self)
        addTrackingArea(area)
        tracking = area
    }

    override func mouseEntered(with event: NSEvent) { hovering = true }
    override func mouseExited(with event: NSEvent) { hovering = false }

    override func prepareForReuse() {
        super.prepareForReuse()
        hovering = false
        playing = false
        menuOpen = false
        updateGround(animated: false)
    }

    override func layout() {
        super.layout()
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        if ground.superlayer !== layer { layer?.insertSublayer(ground, at: 0) }
        ground.frame = bounds.insetBy(dx: inset, dy: 0)
        CATransaction.commit()
    }

    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        shownColor = nil
        updateGround(animated: false)
    }

    /// The colours may have changed (accent, theme): shows them again.
    func restyle() {
        shownColor = nil
        updateGround(animated: false)
    }

    private func updateGround(animated: Bool) {
        let color: Color? = playing ? nil : isSelected ? Theme.raised : menuOpen ? Theme.selected : hovering ? Theme.hover : nil
        guard color != shownColor || !animated else { return }
        let previous = shownColor
        shownColor = color
        var cgColor: CGColor?
        effectiveAppearance.performAsCurrentDrawingAppearance {
            // Fading to nothing goes to the same colour without alpha, not to black.
            cgColor = color.map { NSColor($0).cgColor }
                ?? previous.map { NSColor($0).withAlphaComponent(0).cgColor }
        }
        CATransaction.begin()
        if animated && !Motion.shared.reduced {
            CATransaction.setAnimationDuration(color == nil ? Motion.fade : Motion.exit)
        } else {
            CATransaction.setDisableActions(true)
        }
        ground.backgroundColor = cgColor
        CATransaction.commit()
    }

    override func drawBackground(in dirtyRect: NSRect) {}
    override func drawSelection(in dirtyRect: NSRect) {}
}

/// One cell of a track row. Laid out by hand: rows scroll by the hundred.
class TrackCellView: NSTableCellView {
    struct Style {
        var dense: Bool
        var playing: Bool
        /// The bars move (playing, on screen, motion not reduced).
        var animating: Bool
        var compact: Bool
        /// The row's menu is open.
        var menuOpen = false
    }

    static func make(_ kind: TrackColumn) -> TrackCellView {
        switch kind {
        case .number: NumberCell()
        case .title: TitleCell()
        case .format: FormatCell()
        case .more: MoreCell()
        default: TextCell(kind: kind)
        }
    }

    func fill(entry: TrackEntry?, row: Int, style: Style, controller: TrackTableController) {}

    static func label(size: CGFloat = 13, weight: NSFont.Weight = .regular, color: Color = Theme.text2, right: Bool = false) -> NSTextField {
        let label = NSTextField(labelWithString: "")
        label.font = .monospacedDigitSystemFont(ofSize: size, weight: weight)
        label.textColor = NSColor(color)
        label.lineBreakMode = .byTruncatingTail
        label.alignment = right ? .right : .left
        label.cell?.truncatesLastVisibleLine = true
        label.maximumNumberOfLines = 1
        label.allowsDefaultTighteningForTruncation = false
        return label
    }

    /// `label` vertically centred in the cell, `inset` from the sides.
    func center(_ label: NSTextField, leading: CGFloat = 0, trailing: CGFloat = 8) {
        let height = ceil(label.intrinsicContentSize.height)
        label.frame = NSRect(x: leading, y: (bounds.height - height) / 2, width: max(0, bounds.width - leading - trailing), height: height)
    }
}

/// The row number; the playing row shows the bars instead (spec §3.2).
final class NumberCell: TrackCellView {
    private let number = TrackCellView.label(color: Theme.text3, right: true)
    private let bars = PlayingBarsView()

    override init(frame: NSRect) {
        super.init(frame: frame)
        addSubview(number)
        addSubview(bars)
        bars.isHidden = true
    }

    required init?(coder: NSCoder) { fatalError() }

    override func fill(entry: TrackEntry?, row: Int, style: Style, controller: TrackTableController) {
        number.stringValue = String(row + 1)
        number.isHidden = style.playing
        bars.isHidden = !style.playing
        bars.setAccessibilityLabel(tr("now_playing"))
        if style.playing {
            bars.color = NSColor(Theme.accentText)
            bars.animating = style.animating
        } else {
            bars.animating = false
        }
        needsLayout = true
    }

    override func layout() {
        super.layout()
        center(number, trailing: 14)
        let size = NSSize(width: 14, height: 16)
        bars.frame = NSRect(x: bounds.width - 14 - size.width, y: (bounds.height - size.height) / 2, width: size.width, height: size.height)
    }
}

/// Cover, title (accent while playing) over the artist (`Unknown artist` in `text3`).
final class TitleCell: TrackCellView {
    private let cover = CoverLayerView()
    private let title = TrackCellView.label(size: 14, weight: .semibold, color: Theme.text)
    private let artist = TrackCellView.label()
    private var dense = false

    override init(frame: NSRect) {
        super.init(frame: frame)
        title.font = .systemFont(ofSize: 14, weight: .semibold)
        artist.font = .systemFont(ofSize: 13)
        addSubview(cover)
        addSubview(title)
        addSubview(artist)
    }

    required init?(coder: NSCoder) { fatalError() }

    override func fill(entry: TrackEntry?, row: Int, style: Style, controller: TrackTableController) {
        dense = style.dense
        guard let track = entry?.track else {
            title.stringValue = ""
            artist.stringValue = ""
            cover.show(path: nil, loading: true, pixels: 0)
            return
        }
        title.stringValue = track.title
        title.textColor = NSColor(style.playing ? Theme.accentText : Theme.text)
        if track.artist.isEmpty {
            artist.stringValue = tr("unknown_artist")
            artist.textColor = NSColor(Theme.text3)
        } else {
            artist.stringValue = track.artist
            artist.textColor = NSColor(Theme.text2)
        }
        artist.font = .systemFont(ofSize: style.dense ? 12 : 13)
        let size: CGFloat = style.dense ? 28 : 36
        let scale = window?.backingScaleFactor ?? 2
        cover.show(path: track.cover, loading: false, pixels: Int((size * scale).rounded(.up)))
        needsLayout = true
    }

    override func layout() {
        super.layout()
        let size: CGFloat = dense ? 28 : 36
        cover.frame = NSRect(x: 0, y: (bounds.height - size) / 2, width: size, height: size)
        let x = size + 12
        let width = max(0, bounds.width - x - 12)
        let titleHeight = ceil(title.intrinsicContentSize.height)
        let artistHeight = ceil(artist.intrinsicContentSize.height)
        // 2 points between them (none in dense rows), as `TrackRow.qml:111`.
        let spacing: CGFloat = dense ? 0 : 2
        let total = titleHeight + spacing + artistHeight
        let top = (bounds.height - total) / 2
        // Not flipped: y grows upward.
        artist.frame = NSRect(x: x, y: top, width: width, height: artistHeight)
        title.frame = NSRect(x: x, y: top + artistHeight + spacing, width: width, height: titleHeight)
    }
}

/// Album, genre, year, added, plays, last played or time.
final class TextCell: TrackCellView {
    private let kind: TrackColumn
    private let text: NSTextField

    init(kind: TrackColumn) {
        self.kind = kind
        text = TrackCellView.label(right: kind.alignRight)
        if kind == .album || kind == .genre {
            text.font = .systemFont(ofSize: 13)
        }
        super.init(frame: .zero)
        addSubview(text)
    }

    required init?(coder: NSCoder) { fatalError() }

    override func fill(entry: TrackEntry?, row: Int, style: Style, controller: TrackTableController) {
        guard let entry else {
            text.stringValue = ""
            return
        }
        let track = entry.track
        text.stringValue = switch kind {
        case .album: track.album ?? ""
        case .genre: entry.genre
        case .year: track.year.map(String.init) ?? ""
        case .added: entry.added.map { $0 > 0 ? Format.ago(Double($0)) : "" } ?? ""
        case .plays: String(track.plays)
        case .lastPlayed: entry.lastPlayed.map { Format.ago(Double($0)) } ?? tr("never_played")
        case .time: Format.length(track.length)
        default: ""
        }
        needsLayout = true
    }

    override func layout() {
        super.layout()
        // Qt's Value labels have no padding; album and genre keep 12 on the right.
        if kind == .album || kind == .genre {
            center(text, leading: 0, trailing: 12)
        } else {
            center(text, leading: 0, trailing: 0)
        }
    }
}

/// The codec in capitals in an outlined badge; nothing when unknown.
final class FormatCell: TrackCellView {
    private let box = BadgeBox()
    private let badge = TrackCellView.label(size: 12)

    override init(frame: NSRect) {
        super.init(frame: frame)
        badge.font = .systemFont(ofSize: 12)
        badge.alignment = .center
        addSubview(box)
        box.addSubview(badge)
    }

    required init?(coder: NSCoder) { fatalError() }

    override func fill(entry: TrackEntry?, row: Int, style: Style, controller: TrackTableController) {
        badge.stringValue = entry?.format ?? ""
        box.isHidden = badge.stringValue.isEmpty
        needsLayout = true
    }

    override func layout() {
        super.layout()
        badge.textColor = NSColor(Theme.text2)
        let size = badge.intrinsicContentSize
        // 14 in, 6 + 6 around the codec, 2 + 2 above and below (`TrackRow.qml:177-186`).
        let width = ceil(size.width) + 12
        let height = ceil(size.height) + 4
        box.frame = NSRect(x: 14, y: (bounds.height - height) / 2, width: max(0, min(width, bounds.width - 14)), height: height)
        let textHeight = ceil(size.height)
        badge.frame = NSRect(x: 0, y: (height - textHeight) / 2, width: box.frame.width, height: textHeight)
    }
}

/// The badge's outline.
private final class BadgeBox: NSView {
    override var wantsUpdateLayer: Bool { true }

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.cornerRadius = 4
        layer?.borderWidth = 1
    }

    required init?(coder: NSCoder) { fatalError() }

    override func updateLayer() {
        layer?.borderColor = NSColor(Theme.border).cgColor
    }
}

/// The row's "⋯", opening the track menu below it: 32 points, radius 6, at the column's right
/// edge; `text3`, `text` on a `menuHover` square while its menu is open (`TrackRow.qml:201-223`).
final class MoreCell: TrackCellView {
    private let button = MoreButton()
    private weak var controller: TrackTableController?
    private var row = -1

    override init(frame: NSRect) {
        super.init(frame: frame)
        button.target = self
        button.action = #selector(pressed)
        addSubview(button)
    }

    required init?(coder: NSCoder) { fatalError() }

    override func fill(entry: TrackEntry?, row: Int, style: Style, controller: TrackTableController) {
        self.controller = controller
        self.row = row
        let name = tr("track_actions", entry?.track.title ?? "")
        button.toolTip = name
        button.setAccessibilityLabel(name)
        button.isHidden = entry == nil
        button.open = style.menuOpen
    }

    override func layout() {
        super.layout()
        button.frame = NSRect(x: bounds.width - 32, y: (bounds.height - 32) / 2, width: 32, height: 32)
    }

    @objc private func pressed() {
        controller?.openMenu(row: row, from: button)
    }
}

/// The "⋯" button itself.
final class MoreButton: NSButton {
    var open = false {
        didSet { if oldValue != open { needsDisplay = true; restyle() } }
    }

    override init(frame: NSRect) {
        super.init(frame: frame)
        isBordered = false
        bezelStyle = .regularSquare
        image = NSImage(systemSymbolName: "ellipsis", accessibilityDescription: nil)?
            .withSymbolConfiguration(.init(pointSize: 14, weight: .semibold))
        imagePosition = .imageOnly
        refusesFirstResponder = true
        restyle()
    }

    required init?(coder: NSCoder) { fatalError() }

    private func restyle() {
        contentTintColor = NSColor(open ? Theme.text : Theme.text3)
    }

    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        restyle()
    }

    override func draw(_ dirtyRect: NSRect) {
        if open {
            NSColor(Theme.menuHover).setFill()
            NSBezierPath(roundedRect: bounds, xRadius: 6, yRadius: 6).fill()
        }
        super.draw(dirtyRect)
    }
}

/// A cover thumbnail on a layer: a `raised` tile, the picture decoded off the main thread at
/// the size shown (`Artwork`), a note without one.
final class CoverLayerView: NSView {
    /// What shows: the path and size, or nothing yet.
    private var shown: String?
    private var path: String?
    private var pixels = 0
    private let placeholder = NSImageView()

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.cornerRadius = 4
        layer?.masksToBounds = true
        layer?.contentsGravity = .resizeAspectFill
        placeholder.image = NSImage(systemSymbolName: "music.note", accessibilityDescription: nil)
        placeholder.contentTintColor = NSColor(Theme.text3)
        placeholder.symbolConfiguration = .init(pointSize: 12, weight: .regular)
        placeholder.isHidden = true
        addSubview(placeholder)
    }

    required init?(coder: NSCoder) { fatalError() }

    override var wantsUpdateLayer: Bool { true }

    override func updateLayer() {
        layer?.backgroundColor = NSColor(Theme.raised).cgColor
    }

    override func layout() {
        super.layout()
        placeholder.frame = bounds
    }

    func show(path: String?, loading: Bool, pixels: Int) {
        let key = loading ? "" : "\(pixels)\u{1}\(path ?? "-")"
        guard key != shown else { return }
        shown = key
        self.path = path
        self.pixels = pixels
        layer?.contents = nil
        placeholder.isHidden = true
        guard let path else {
            placeholder.isHidden = loading
            return
        }
        if let cached = Artwork.shared.cached(path: path, pixels: pixels) {
            layer?.contents = cached.image
            return
        }
        Task { @MainActor [weak self] in
            let picture = await Artwork.shared.thumbnail(path: path, pixels: pixels)
            guard let self, self.path == path, self.pixels == pixels else { return }
            if let picture {
                self.layer?.contents = picture.image
            } else {
                self.placeholder.isHidden = false
            }
        }
    }
}

/// Three bars of the playing row (spec §3.2, `qml/PlayingBars.qml`): while playing each eases
/// to a random height every 180–320 ms, run by Core Animation (no timer here); paused, they
/// settle at 10/16/12 over 200 ms. Frozen under reduced motion, while no window is on screen,
/// and 10 s after the window stops being the key one.
final class PlayingBarsView: NSView {
    var color: NSColor = .controlAccentColor {
        didSet { bars.forEach { $0.backgroundColor = color.cgColor } }
    }
    /// The bars move: playing, on screen, motion not reduced.
    var animating = false {
        didSet { if oldValue != animating { apply() } }
    }

    private let bars = (0..<3).map { _ in CALayer() }
    private let rest: [CGFloat] = [10, 16, 12]
    private var keyWindow = true
    private var idle: Task<Void, Never>?

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        for bar in bars {
            bar.anchorPoint = CGPoint(x: 0.5, y: 0)
            bar.cornerRadius = 1
            bar.backgroundColor = color.cgColor
            layer?.addSublayer(bar)
        }
        setAccessibilityElement(true)
        setAccessibilityRole(.image)
    }

    required init?(coder: NSCoder) { fatalError() }

    override func layout() {
        super.layout()
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for (index, bar) in bars.enumerated() {
            bar.bounds = CGRect(x: 0, y: 0, width: 3, height: rest[index])
            bar.position = CGPoint(x: 1.5 + CGFloat(index) * 5.5, y: 0)
        }
        CATransaction.commit()
        apply()
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        NotificationCenter.default.removeObserver(self)
        guard let window else {
            idle?.cancel()
            return
        }
        NotificationCenter.default.addObserver(self, selector: #selector(becameKey), name: NSWindow.didBecomeKeyNotification, object: window)
        NotificationCenter.default.addObserver(self, selector: #selector(resignedKey), name: NSWindow.didResignKeyNotification, object: window)
        // An inactive window's bars still move for 10 s, as after it stops being the key one.
        keyWindow = true
        if !window.isKeyWindow { resignedKey() }
        apply()
    }

    @objc private func becameKey() {
        idle?.cancel()
        if !keyWindow {
            keyWindow = true
            apply()
        }
    }

    @objc private func resignedKey() {
        idle?.cancel()
        idle = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .seconds(10))
            guard let self, !Task.isCancelled else { return }
            self.keyWindow = false
            self.apply()
        }
    }

    private func apply() {
        let moving = animating && keyWindow && window != nil
        for (index, bar) in bars.enumerated() {
            if moving {
                guard bar.animation(forKey: "dance") == nil else { continue }
                var values: [CGFloat] = [rest[index]]
                var times: [Double] = [0]
                var total = 0.0
                for _ in 0..<12 {
                    total += Double.random(in: 0.18...0.32)
                    times.append(total)
                    values.append(CGFloat.random(in: 4...16))
                }
                total += Double.random(in: 0.18...0.32)
                times.append(total)
                values.append(rest[index])
                let animation = CAKeyframeAnimation(keyPath: "bounds.size.height")
                animation.values = values
                animation.keyTimes = times.map { NSNumber(value: $0 / total) }
                animation.duration = total
                animation.calculationMode = .cubic
                animation.repeatCount = .infinity
                bar.add(animation, forKey: "dance")
            } else if bar.animation(forKey: "dance") != nil {
                let current = bar.presentation()?.bounds.height ?? rest[index]
                bar.removeAnimation(forKey: "dance")
                let settle = CABasicAnimation(keyPath: "bounds.size.height")
                settle.fromValue = current
                settle.toValue = rest[index]
                settle.duration = Motion.move
                settle.timingFunction = CAMediaTimingFunction(controlPoints: 0.2, 0, 0, 1)
                bar.add(settle, forKey: "settle")
            }
        }
    }
}

/// The playing row's ground under the rows (`qml/PlayingHighlight.qml`): `selected`, or
/// `raised` when that row is also selected, rounded 8; the table moves it (glide, crossfade or
/// jump) and tints it with the accent for a moment after a sort.
final class PlayingHighlightView: NSView {
    var picked = false {
        didSet { if oldValue != picked { restyle() } }
    }
    private let tint = CALayer()

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.cornerRadius = 8
        tint.cornerRadius = 8
        tint.opacity = 0
        layer?.addSublayer(tint)
        restyle()
    }

    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        restyle()
    }

    override func layout() {
        super.layout()
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        tint.frame = bounds
        CATransaction.commit()
    }

    func restyle() {
        effectiveAppearance.performAsCurrentDrawingAppearance {
            layer?.backgroundColor = NSColor(picked ? Theme.raised : Theme.selected).cgColor
            tint.backgroundColor = NSColor(Theme.accent).withAlphaComponent(0.12).cgColor
        }
    }

    /// The accent tint: holds 200 ms, then fades over 400 ms (`PlayingHighlight.qml:187-203`).
    func flash() {
        let animation = CAKeyframeAnimation(keyPath: "opacity")
        animation.values = [1, 1, 0]
        animation.keyTimes = [0, NSNumber(value: 0.2 / 0.6), 1]
        animation.duration = 0.6
        animation.timingFunctions = [CAMediaTimingFunction(name: .linear), CAMediaTimingFunction(controlPoints: 0.2, 0, 0, 1)]
        tint.opacity = 0
        tint.add(animation, forKey: "flash")
    }
}
