import AppKit
import QuartzCore
import SwiftUI

/// A slider's track, fill and knob drawn by Core Animation layers.
///
/// While `rate` is above zero the fill and the knob glide on their own: one linear animation
/// to the end of the track per change of anchor (play, pause, seek, new track), run by the
/// window server. The main thread draws nothing between changes, where a SwiftUI timeline
/// would rebuild the view every frame or every tick.
struct BarTrack: NSViewRepresentable {
    /// 0…1 at `anchor`.
    var value: Double
    /// Fraction of the track per second; 0 holds still.
    var rate: Double = 0
    var anchor: ContinuousClock.Instant = .now
    var fill: Color
    var track: Color
    var knob: Color
    var ring: Color
    var showKnob: Bool
    var focused: Bool

    func makeNSView(context: Context) -> BarTrackView { BarTrackView() }

    func updateNSView(_ view: BarTrackView, context: Context) {
        view.colors = (NSColor(fill), NSColor(track), NSColor(knob), NSColor(ring))
        view.knobShown = showKnob
        view.focused = focused
        view.set(value: value, rate: rate, anchor: anchor)
    }
}

final class BarTrackView: NSView {
    private let trackLayer = CALayer()
    private let fillLayer = CALayer()
    private let knobLayer = CALayer()

    private var value = 0.0
    private var rate = 0.0
    private var anchor = ContinuousClock.now

    var colors: (fill: NSColor, track: NSColor, knob: NSColor, ring: NSColor) = (.white, .gray, .white, .orange) {
        didSet { applyColors() }
    }
    var knobShown = true {
        didSet { if knobShown != oldValue { knobLayer.opacity = knobShown ? 1 : 0 } }
    }
    var focused = false {
        didSet { if focused != oldValue { applyColors() } }
    }

    private static let bar: CGFloat = 4
    private static let knobSize: CGFloat = 12

    init() {
        super.init(frame: .zero)
        wantsLayer = true
        layerContentsRedrawPolicy = .never
        for layer in [trackLayer, fillLayer, knobLayer] {
            layer.actions = ["position": NSNull(), "bounds": NSNull(), "backgroundColor": NSNull()]
            self.layer?.addSublayer(layer)
        }
        trackLayer.cornerRadius = Self.bar / 2
        fillLayer.cornerRadius = Self.bar / 2
        fillLayer.anchorPoint = CGPoint(x: 0, y: 0.5)
        knobLayer.cornerRadius = Self.knobSize / 2
        knobLayer.bounds = CGRect(x: 0, y: 0, width: Self.knobSize, height: Self.knobSize)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    override var isFlipped: Bool { true }
    /// Clicks and drags belong to the SwiftUI gesture above.
    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    func set(value: Double, rate: Double, anchor: ContinuousClock.Instant) {
        guard value != self.value || rate != self.rate || anchor != self.anchor else { return }
        self.value = value
        self.rate = rate
        self.anchor = anchor
        restart()
    }

    override func layout() {
        super.layout()
        let height = bounds.height
        let width = bounds.width
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        trackLayer.frame = CGRect(x: 0, y: (height - Self.bar) / 2, width: width, height: Self.bar)
        fillLayer.position = CGPoint(x: 0, y: height / 2)
        CATransaction.commit()
        restart()
    }

    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        applyColors()
    }

    private func applyColors() {
        effectiveAppearance.performAsCurrentDrawingAppearance {
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            trackLayer.backgroundColor = colors.track.cgColor
            fillLayer.backgroundColor = colors.fill.cgColor
            knobLayer.backgroundColor = colors.knob.cgColor
            knobLayer.borderColor = colors.ring.cgColor
            knobLayer.borderWidth = focused ? 2 : 0
            CATransaction.commit()
        }
    }

    /// Puts the fill and knob where the value is now and, while moving, starts one linear
    /// glide to the end of the track.
    private func restart() {
        let width = bounds.width
        guard width > 0 else { return }
        let elapsed = ContinuousClock.now - anchor
        let seconds = max(0, Double(elapsed.components.seconds) + Double(elapsed.components.attoseconds) / 1e18)
        let now = min(max(value + rate * seconds, 0), 1)

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        fillLayer.removeAllAnimations()
        knobLayer.removeAllAnimations()
        let start = fillWidth(now, in: width)
        let startKnob = knobX(now, in: width)
        fillLayer.bounds = CGRect(x: 0, y: 0, width: start, height: Self.bar)
        knobLayer.position = CGPoint(x: startKnob, y: bounds.height / 2)

        if rate > 0, now < 1 {
            let duration = (1 - now) / rate
            let end = fillWidth(1, in: width)
            let endKnob = knobX(1, in: width)
            let grow = CABasicAnimation(keyPath: "bounds.size.width")
            grow.fromValue = start
            grow.toValue = end
            let slide = CABasicAnimation(keyPath: "position.x")
            slide.fromValue = startKnob
            slide.toValue = endKnob
            for animation in [grow, slide] {
                animation.duration = duration
                animation.timingFunction = CAMediaTimingFunction(name: .linear)
                animation.fillMode = .forwards
                animation.isRemovedOnCompletion = false
            }
            fillLayer.bounds.size.width = end
            knobLayer.position.x = endKnob
            fillLayer.add(grow, forKey: "glide")
            knobLayer.add(slide, forKey: "glide")
        }
        CATransaction.commit()
    }

    private func fillWidth(_ fraction: Double, in width: CGFloat) -> CGFloat {
        max(Self.bar, width * fraction)
    }

    private func knobX(_ fraction: Double, in width: CGFloat) -> CGFloat {
        Self.knobSize / 2 + (width - Self.knobSize) * fraction
    }
}
