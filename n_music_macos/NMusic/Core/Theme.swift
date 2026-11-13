import AppKit
import SwiftUI

/// The Qt app's colour tokens (spec §12.1), as dynamic colours that follow the window's
/// appearance by themselves: `Theme.surface`, `Theme.text2`…
///
/// Prefer native materials and semantic styles where the page is native (sidebar, tables,
/// menus); use these where the Qt look is part of the UX (the player bar, cards, pills, toasts).
enum Theme {
    static let bg = token(dark: 0x111316, light: 0xF6F7F9)
    static let side = token(dark: 0x0C0E10, light: 0xECEEF2)
    static let panel = token(dark: 0x0E1013, light: 0xEEF0F3)
    static let surface = token(dark: 0x16191D, light: 0xFFFFFF)
    static let field = token(dark: 0x1A1D21, light: 0xFFFFFF)
    static let input = token(dark: 0x1F2328, light: 0xFFFFFF)
    static let raised = token(dark: 0x23272D, light: 0xE4E7EC)
    static let hover = token(dark: 0x181B1F, light: 0xEEF0F3)
    static let selected = token(dark: 0x1F2329, light: 0xE6EAF0)
    static let menu = token(dark: 0x23272D, light: 0xFFFFFF)
    static let menuHover = token(dark: 0x2F343C, light: 0xEEF0F3)
    static let line = token(dark: 0x22262C, light: 0xDCE0E6)
    static let line2 = token(dark: 0x2A2E35, light: 0xD3D8DF)
    static let border = token(dark: 0x343941, light: 0xC4CAD3)
    static let track = token(dark: 0x2E333A, light: 0xD3D8DF)
    static let shadow = token(dark: 0x000000, darkAlpha: 0x8C / 255, light: 0x000000, lightAlpha: 0x33 / 255)
    static let text = token(dark: 0xECEEF1, light: 0x15171A)
    static let textSoft = token(dark: 0xC9CED6, light: 0x2F353D)
    static let text2 = token(dark: 0xA4ABB6, light: 0x4D5560)
    static let text3 = token(dark: 0x868E9A, light: 0x626A76)
    static let danger = token(dark: 0xFF8A80, light: 0xB3261E)
    static let dangerBg = token(dark: 0x2A1F1F, light: 0xFCEEEE)
    static let dangerLine = token(dark: 0x5A3434, light: 0xE8B4B0)
    static let dangerTitle = token(dark: 0xFFD9D5, light: 0x7A1C16)
    static let dangerText = token(dark: 0xE2C4C0, light: 0x8C3B35)
    static let success = token(dark: 0x7FD1AE, light: 0x1E7A55)

    /// The chosen accent's fill (`accent`), also the app's tint.
    @MainActor static var accent: Color { Accent.current.fill }
    /// Text and icons in the accent: the fill in dark mode, a darker shade in light mode.
    @MainActor static var accentText: Color { Accent.current.text }
    /// Text on the accent fill.
    @MainActor static var accentInk: Color { Accent.current.ink }

    static func token(dark: UInt32, darkAlpha: Double = 1, light: UInt32, lightAlpha: Double = 1) -> Color {
        Color(nsColor: dynamic(dark: rgb(dark, darkAlpha), light: rgb(light, lightAlpha)))
    }

    static func dynamic(dark: NSColor, light: NSColor) -> NSColor {
        NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua ? dark : light
        }
    }

    static func rgb(_ hex: UInt32, _ alpha: Double = 1) -> NSColor {
        NSColor(
            srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
            green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255,
            alpha: alpha
        )
    }
}

/// The six accent presets (spec §12.2). `Accent.current` follows `UISettings.accent`.
struct Accent: Identifiable, Sendable {
    /// Saved in the settings, like `amber`.
    let name: String
    let fillHex: UInt32
    let inkHex: UInt32
    let lightTextHex: UInt32

    var id: String { name }
    var fill: Color { Color(nsColor: Theme.rgb(fillHex)) }
    var ink: Color { Color(nsColor: Theme.rgb(inkHex)) }
    var text: Color { Color(nsColor: Theme.dynamic(dark: Theme.rgb(fillHex), light: Theme.rgb(lightTextHex))) }
    /// The translation key of its accessible name, like `accent_amber`.
    var labelKey: String { "accent_\(name)" }

    static let presets: [Accent] = [
        Accent(name: "amber", fillHex: 0xF0A13A, inkHex: 0x1A1206, lightTextHex: 0x9A5800),
        Accent(name: "green", fillHex: 0x4FC98E, inkHex: 0x04150D, lightTextHex: 0x176E46),
        Accent(name: "teal", fillHex: 0x3CC4C0, inkHex: 0x031615, lightTextHex: 0x0D6C69),
        Accent(name: "blue", fillHex: 0x6AA6FF, inkHex: 0x06111F, lightTextHex: 0x1F5CC2),
        Accent(name: "violet", fillHex: 0xA68BFA, inkHex: 0x120A26, lightTextHex: 0x6B45D1),
        Accent(name: "rose", fillHex: 0xF2779B, inkHex: 0x22070F, lightTextHex: 0xB52A5B),
    ]

    @MainActor static var current: Accent {
        let name = UISettings.shared.accent
        return presets.first { $0.name == name } ?? presets[0]
    }
}

/// Motion tokens (spec §12.4) and whether motion is reduced (brief §9: the app setting OR the
/// system's Reduce Motion).
@MainActor
@Observable
final class Motion {
    static let shared = Motion()

    /// Things leaving: a menu or dialog closing, old words, a removed row, a toast.
    static let exit = 0.08
    /// Fades and changes in place.
    static let fade = 0.15
    /// Things moving where they already are: rows, pills, thumbs.
    static let move = 0.2
    /// New surfaces arriving: menus, dialogs, drawers.
    static let enter = 0.25
    /// A track change sooner than this after the last shows its end state at once.
    static let quick = 0.25

    /// Most moves and fades in: cubic Bézier (0.2, 0, 0, 1).
    static func standard(_ duration: Double) -> Animation { .timingCurve(0.2, 0, 0, 1, duration: duration) }
    /// Things leaving: (0.3, 0, 1, 1).
    static func leaving(_ duration: Double) -> Animation { .timingCurve(0.3, 0, 1, 1, duration: duration) }
    /// A dropped row landing: (0.3, 0.7, 0.4, 1).
    static func landing(_ duration: Double) -> Animation { .timingCurve(0.3, 0.7, 0.4, 1, duration: duration) }

    private(set) var systemReduced = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion

    /// Things change in place instead of travelling; fades stay.
    var reduced: Bool { systemReduced || UISettings.shared.reduceMotion }
    /// Multiplies every translation and scale: 0 when reduced.
    var travel: Double { reduced ? 0 : 1 }

    private init() {
        Self.watchSystem()
    }

    nonisolated private static func watchSystem() {
        NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil, queue: .main
        ) { _ in
            Task { @MainActor in
                Motion.shared.systemReduced = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
            }
        }
    }
}
