import Foundation
import Observation

/// The interface strings, by the Qt app's keys, in the chosen language.
///
/// The bundle carries the Qt app's own translation files (`Resources/lang/<code>_<Name>.json`,
/// copied from `n_music_desktop/assets/lang` at build time). English fills every key a language
/// lacks, one key at a time. Changing `language` switches the whole interface live: views read
/// the strings through `tr`, which Observation tracks.
@MainActor
@Observable
final class Tr {
    static let shared = Tr()

    /// A bundled language.
    struct Language: Identifiable, Hashable, Sendable {
        /// The file's prefix, like `de`.
        let code: String
        /// Its own name, untranslated, like `Deutsch`.
        let name: String
        var id: String { code }
    }

    /// The bundled languages, by name.
    let languages: [Language]
    /// The language the system asks for, when bundled; else English.
    let systemCode: String
    /// The strings shown now, English filled in.
    private(set) var strings: [String: String] = [:]
    /// The chosen language's code; `nil` follows the system. Saved as `desktop.ui.locale`.
    private(set) var language: String?

    @ObservationIgnored private let files: [String: URL]

    private init() {
        var files: [String: URL] = [:]
        var languages: [Language] = []
        let folder = Bundle.main.resourceURL?.appendingPathComponent("lang", isDirectory: true)
        let names = folder.flatMap { try? FileManager.default.contentsOfDirectory(atPath: $0.path) } ?? []
        for file in names where file.hasSuffix(".json") {
            let stem = String(file.dropLast(5))
            guard let split = stem.firstIndex(of: "_") else { continue }
            let code = String(stem[..<split])
            let name = String(stem[stem.index(after: split)...])
            files[code] = folder!.appendingPathComponent(file)
            languages.append(Language(code: code, name: name))
        }
        self.files = files
        self.languages = languages.sorted { $0.name.lowercased() < $1.name.lowercased() }
        let wanted = Locale.preferredLanguages.first
            .map { $0.split(whereSeparator: { $0 == "-" || $0 == "_" }).first.map(String.init) ?? "en" } ?? "en"
        systemCode = files[wanted] != nil ? wanted : "en"
        load(nil)
    }

    /// Shows `code`'s strings, or the system language's when `nil`. Does not save it: the
    /// settings do, see `UISettings.locale`.
    func load(_ code: String?) {
        language = code.flatMap { files[$0] != nil ? $0 : nil }
        var strings = Self.parse(files["en"])
        strings.merge(Self.parse(files[language ?? systemCode]), uniquingKeysWith: { _, new in new })
        self.strings = strings
    }

    /// The label of the "system" choice in the language picker: `System default (Deutsch)`.
    var systemChoiceLabel: String {
        let name = languages.first { $0.code == systemCode }?.name ?? "English"
        return "\(t("language_system")) (\(name))"
    }

    /// The string for `key`; `fallback` (or the key itself) when no file has it.
    func t(_ key: String, fallback: String? = nil) -> String {
        strings[key] ?? fallback ?? key
    }

    private static func parse(_ url: URL?) -> [String: String] {
        guard let url, let data = try? Data(contentsOf: url),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: String]
        else { return [:] }
        return object
    }

    /// `template` with Qt's `%1`, `%2`… replaced by `arguments`, in one pass (an argument that
    /// itself holds `%2` stays as it is).
    nonisolated static func substitute(_ template: String, _ arguments: [String]) -> String {
        guard !arguments.isEmpty, template.contains("%") else { return template }
        var result = ""
        var index = template.startIndex
        while index < template.endIndex {
            let character = template[index]
            if character == "%" {
                var end = template.index(after: index)
                while end < template.endIndex, template[end].isASCII, template[end].isNumber {
                    end = template.index(after: end)
                }
                if end > template.index(after: index),
                   let number = Int(template[template.index(after: index)..<end]),
                   number >= 1, number <= arguments.count {
                    result += arguments[number - 1]
                    index = end
                    continue
                }
            }
            result.append(character)
            index = template.index(after: index)
        }
        return result
    }
}

/// The interface string `key`, with Qt's `%1 %2…` placeholders filled in by `arguments`.
///
///     tr("tracks_many", Format.number(1234))   // "1,234 tracks"
@MainActor
func tr(_ key: String, _ arguments: any CustomStringConvertible...) -> String {
    Tr.Substitution.apply(Tr.shared.t(key), arguments)
}

/// `tr` for a key the Qt app never had (new native strings, such as menu names): `fallback` is
/// the English text until a translation file adds the key.
@MainActor
func tr(_ key: String, fallback: String, _ arguments: any CustomStringConvertible...) -> String {
    Tr.Substitution.apply(Tr.shared.t(key, fallback: fallback), arguments)
}

extension Tr {
    enum Substitution {
        static func apply(_ template: String, _ arguments: [any CustomStringConvertible]) -> String {
            Tr.substitute(template, arguments.map(\.description))
        }
    }
}
