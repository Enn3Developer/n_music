import AppKit
import Observation

/// The interface's own preferences, kept next to the core's in `settings.json` under the Qt
/// app's section `desktop.ui` with the same fields and defaults (spec §6.4), so they carry over.
/// Each is saved as soon as it changes; fields this app doesn't know are kept.
@MainActor
@Observable
final class UISettings {
    static let shared = UISettings()

    static let key = "desktop.ui"

    enum ThemeChoice: String, CaseIterable, Sendable {
        case system = "System"
        case light = "Light"
        case dark = "Dark"
    }

    var theme: ThemeChoice = .system { didSet { save(); applyTheme() } }
    /// A bundled language's code; `nil` follows the system.
    var locale: String? { didSet { save(); Tr.shared.load(locale) } }
    var saveWindowSize = false { didSet { save() } }
    /// The main window's size when not maximized (zoomed), and whether it was.
    var windowSize = WindowSize() { didSet { save() } }
    /// Looks for a new version once a launch.
    var checkUpdates = true { didSet { save() } }
    /// The mini player floats above other windows.
    var miniOnTop = true { didSet { save() } }
    /// One of `Accent.presets`' names; an unknown one shows amber.
    var accent = "amber" { didSet { save() } }
    /// Track lists use smaller covers and tighter rows.
    var compactRows = false { didSet { save() } }
    /// The app's own setting; `Motion.reduced` also follows the system's.
    var reduceMotion = false { didSet { save() } }
    /// Track table columns left out, by the Qt names (`number`, `album`, `genre`, `year`,
    /// `added`, `plays`, `lastPlayed`, `format`, `time`).
    var hiddenColumns: [String] = [] { didSet { save() } }
    var showHiddenFolders = false { didSet { save() } }

    struct WindowSize: Equatable, Sendable {
        var width = 1280.0
        var height = 800.0
        var maximized = false
    }

    @ObservationIgnored private var loading = true
    @ObservationIgnored private var stored: [String: Any] = [:]

    private init() {
        if let json = Store.shared.core.setting(key: Self.key),
           let object = try? JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any] {
            stored = object
            theme = (object["theme"] as? String).flatMap(ThemeChoice.init(rawValue:)) ?? .system
            locale = object["locale"] as? String
            saveWindowSize = object["save_window_size"] as? Bool ?? false
            if let size = object["window_size"] as? [String: Any] {
                windowSize = WindowSize(
                    width: (size["width"] as? NSNumber)?.doubleValue ?? 1280,
                    height: (size["height"] as? NSNumber)?.doubleValue ?? 800,
                    maximized: size["maximized"] as? Bool ?? false
                )
            }
            checkUpdates = object["check_updates"] as? Bool ?? true
            miniOnTop = object["mini_on_top"] as? Bool ?? true
            accent = object["accent"] as? String ?? "amber"
            compactRows = object["compact_rows"] as? Bool ?? false
            reduceMotion = object["reduce_motion"] as? Bool ?? false
            hiddenColumns = object["hidden_columns"] as? [String] ?? []
            showHiddenFolders = object["show_hidden_folders"] as? Bool ?? false
        }
        loading = false
        Tr.shared.load(locale)
        applyTheme()
    }

    /// Shows or hides a track table column (Qt name).
    func setColumn(_ name: String, visible: Bool) {
        var hidden = hiddenColumns.filter { $0 != name }
        if !visible { hidden.append(name) }
        hiddenColumns = hidden
    }

    func isColumnVisible(_ name: String) -> Bool { !hiddenColumns.contains(name) }

    private func applyTheme() {
        guard let app = NSApp else { return }
        app.appearance = switch theme {
        case .system: nil
        case .light: NSAppearance(named: .aqua)
        case .dark: NSAppearance(named: .darkAqua)
        }
    }

    /// Applies the theme once the application object exists (the store may load first).
    func applyAppearance() { applyTheme() }

    private func save() {
        guard !loading else { return }
        var object = stored
        object["theme"] = theme.rawValue
        object["locale"] = locale ?? NSNull()
        object["save_window_size"] = saveWindowSize
        object["window_size"] = [
            "width": Int(windowSize.width.rounded()),
            "height": Int(windowSize.height.rounded()),
            "maximized": windowSize.maximized,
        ] as [String: Any]
        object["check_updates"] = checkUpdates
        object["mini_on_top"] = miniOnTop
        object["accent"] = accent
        object["compact_rows"] = compactRows
        object["reduce_motion"] = reduceMotion
        object["hidden_columns"] = hiddenColumns
        object["show_hidden_folders"] = showHiddenFolders
        stored = object
        guard let data = try? JSONSerialization.data(withJSONObject: object, options: [.sortedKeys]),
              let json = String(data: data, encoding: .utf8)
        else { return }
        Store.shared.core.setSetting(key: Self.key, json: json)
    }
}
