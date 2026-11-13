import Foundation
import Observation

// What filter rules test and how they read, and the keys tracks sort by: a port of the Qt
// app's `qml/Filters.qml`, writing and reading the same editor JSON (`parseFilter`,
// `tracksQuery` read it in the core).
//
// A filter is `{ match, rules }`, `match` being `all` or `any`. A rule is
// `{ field, op, value, from, to, amount, unit }`, a group `{ group, rules }` with `group`
// `all`, `any` or `none`. A sort is a string like `artist,-year`.

/// A rule being written. Every value is text, as the editor's boxes hold it.
struct FilterRule: Identifiable, Equatable, Sendable {
    var id = UUID()
    var field: String
    var op: String
    var value: String = ""
    var from: String = ""
    var to: String = ""
    var amount: String = "30"
    var unit: String = "days"
}

/// Rules that count as one: `all`, `any` or `none` of them must match.
struct FilterGroup: Identifiable, Equatable, Sendable {
    var id = UUID()
    var group: String
    var rules: [FilterRule]
}

enum FilterEntry: Identifiable, Equatable, Sendable {
    case rule(FilterRule)
    case group(FilterGroup)

    var id: UUID {
        switch self {
        case .rule(let rule): rule.id
        case .group(let group): group.id
        }
    }
}

/// A filter as the editor writes it.
struct FilterSpec: Equatable, Sendable {
    var match = "all"
    var entries: [FilterEntry] = []
}

/// A sort key as the editor writes it.
struct SortKeySpec: Identifiable, Equatable, Sendable {
    var id = UUID()
    var field: String
    var descending: Bool
}

@MainActor
enum Filters {
    enum Kind { case text, range, source, playlist, period }

    struct Field {
        let key: String
        let labelKey: String
        /// The shorter name a box and a chip use, when there is one.
        let nameKey: String?
        let kind: Kind
        let ops: [String]
        /// The library values offered while typing.
        let values: ValueField?
    }

    static let fields: [String: Field] = {
        let list = [
            Field(key: "search", labelKey: "field_search_long", nameKey: "field_search", kind: .text, ops: ["contains", "not_contains"], values: nil),
            Field(key: "artist", labelKey: "field_artist", nameKey: nil, kind: .text, ops: ["is", "is_not"], values: .artist),
            Field(key: "album_artist", labelKey: "field_album_artist", nameKey: nil, kind: .text, ops: ["is", "is_not"], values: .albumArtist),
            Field(key: "album", labelKey: "field_album", nameKey: nil, kind: .text, ops: ["is", "is_not"], values: .album),
            Field(key: "genre", labelKey: "field_genre", nameKey: nil, kind: .text, ops: ["is", "is_not"], values: .genre),
            Field(key: "year", labelKey: "field_year", nameKey: nil, kind: .range, ops: ["between", "from", "until"], values: nil),
            Field(key: "codec", labelKey: "field_codec", nameKey: nil, kind: .text, ops: ["is", "is_not"], values: .codec),
            Field(key: "source", labelKey: "field_source", nameKey: nil, kind: .source, ops: ["is", "is_not"], values: nil),
            Field(key: "playlist", labelKey: "field_playlist_long", nameKey: "field_playlist", kind: .playlist, ops: ["in", "not_in"], values: nil),
            Field(key: "plays", labelKey: "field_plays", nameKey: nil, kind: .range, ops: ["at_least", "at_most", "between"], values: nil),
            Field(key: "played_within", labelKey: "field_played_within", nameKey: nil, kind: .period, ops: ["the_last"], values: nil),
            Field(key: "not_played_within", labelKey: "field_not_played_within", nameKey: nil, kind: .period, ops: ["the_last"], values: nil),
        ]
        return Dictionary(uniqueKeysWithValues: list.map { ($0.key, $0) })
    }()

    /// The fields under their headings, as the field picker lists them.
    static let fieldGroups: [(headingKey: String, fields: [String])] = [
        ("filter_text", ["search"]),
        ("filter_tags", ["artist", "album_artist", "album", "genre", "year", "codec"]),
        ("filter_where", ["source", "playlist"]),
        ("filter_listening", ["plays", "played_within", "not_played_within"]),
    ]

    static func label(_ field: String) -> String {
        fields[field].map { tr($0.labelKey) } ?? field
    }

    /// How a rule calls the field when shorter than its label: "Search", "Playlist".
    static func shortName(_ field: String) -> String {
        guard let definition = fields[field] else { return field }
        return tr(definition.nameKey ?? definition.labelKey)
    }

    static func opLabel(_ op: String) -> String { tr("op_" + op) }

    static let matchValues = ["all", "any"]
    static let groupValues = ["all", "any", "none"]
    static let units = ["days", "weeks", "months", "years"]

    // MARK: Sorting

    enum SortKind: String { case text, date, number }

    /// The fields tracks sort by, in the editor's order.
    static let sortFields: [(field: String, labelKey: String, nameKey: String?, kind: SortKind)] = [
        ("title", "sort_key_title", nil, .text),
        ("artist", "sort_key_artist", nil, .text),
        ("album", "sort_key_album", "sort_key_album_short", .text),
        ("genre", "sort_key_genre", nil, .text),
        ("year", "sort_key_year", nil, .date),
        ("length", "sort_key_length", nil, .number),
        ("format", "sort_key_format", nil, .text),
        ("plays", "sort_key_plays", nil, .number),
        ("lastPlayed", "sort_key_last_played", nil, .date),
        ("location", "sort_key_location", nil, .text),
    ]

    static func sortFieldLabel(_ field: String) -> String {
        sortFields.first { $0.field == field }.map { tr($0.labelKey) } ?? field
    }

    /// "A → Z" / "Z → A", "Oldest first" / "Newest first", "Low → high" / "High → low".
    static func directionLabel(_ field: String, descending: Bool) -> String {
        let kind = sortFields.first { $0.field == field }?.kind ?? .text
        return tr("sort_\(kind.rawValue)_\(descending ? "down" : "up")")
    }

    /// A sort as keys, leaving out the fields the editor doesn't know (like `added`).
    static func parseSort(_ sort: String) -> [SortKeySpec] {
        sort.split(separator: ",").compactMap { part in
            let key = part.trimmingCharacters(in: .whitespaces)
            let descending = key.hasPrefix("-")
            let field = descending ? String(key.dropFirst()) : key
            guard sortFields.contains(where: { $0.field == field }) else { return nil }
            return SortKeySpec(field: field, descending: descending)
        }
    }

    static func sortString(_ keys: [SortKeySpec]) -> String {
        keys.map { ($0.descending ? "-" : "") + $0.field }.joined(separator: ",")
    }

    /// A sort in short words, like `Year ↑, Album ↑`.
    static func sortLabel(_ sort: String) -> String {
        parseSort(sort).map { key in
            let field = sortFields.first { $0.field == key.field }!
            return tr(field.nameKey ?? field.labelKey) + (key.descending ? " ↓" : " ↑")
        }.joined(separator: ", ")
    }

    // MARK: Rules

    /// What a rule testing `field` starts with: the first playlist or source, if it picks one.
    static func firstValue(_ field: String) -> String {
        if field == "playlist", let first = Store.shared.playlists.first {
            return String(first.id)
        }
        if field == "source", let first = FilterCatalog.shared.sources.first {
            return first.value
        }
        return ""
    }

    static func newRule(_ field: String = "artist") -> FilterRule {
        FilterRule(field: field, op: fields[field]?.ops.first ?? "is", value: firstValue(field))
    }

    static func newGroup() -> FilterGroup {
        FilterGroup(group: "any", rules: [newRule()])
    }

    /// `rule` testing `field` instead, its condition and value started over.
    static func retarget(_ rule: FilterRule, to field: String) -> FilterRule {
        var fresh = newRule(field)
        fresh.id = rule.id
        return fresh
    }

    // MARK: JSON

    static func parse(_ json: String) -> FilterSpec {
        guard !json.isEmpty,
              let object = try? JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any],
              let rules = object["rules"] as? [[String: Any]]
        else { return FilterSpec() }
        var spec = FilterSpec(match: object["match"] as? String ?? "all")
        for entry in rules {
            if let group = entry["group"] as? String {
                let inner = (entry["rules"] as? [[String: Any]] ?? []).map(rule(from:))
                spec.entries.append(.group(FilterGroup(group: group, rules: inner)))
            } else {
                spec.entries.append(.rule(rule(from: entry)))
            }
        }
        return spec
    }

    private static func rule(from object: [String: Any]) -> FilterRule {
        let text = { (key: String) -> String? in
            switch object[key] {
            case let string as String: string
            case let number as NSNumber: number.stringValue
            default: nil
            }
        }
        let field = text("field") ?? "artist"
        var rule = fields[field] != nil ? newRule(field) : FilterRule(field: field, op: "")
        if let op = text("op") { rule.op = op }
        if let value = text("value") { rule.value = value }
        if let from = text("from") { rule.from = from }
        if let to = text("to") { rule.to = to }
        if let amount = text("amount") { rule.amount = amount }
        if let unit = text("unit") { rule.unit = unit }
        return rule
    }

    /// The filter as JSON; empty without rules.
    static func json(_ spec: FilterSpec) -> String {
        guard !spec.entries.isEmpty else { return "" }
        let rules: [[String: Any]] = spec.entries.map { entry in
            switch entry {
            case .rule(let rule): object(rule)
            case .group(let group): ["group": group.group, "rules": group.rules.map(object)]
            }
        }
        let root: [String: Any] = ["match": spec.match, "rules": rules]
        guard let data = try? JSONSerialization.data(withJSONObject: root, options: [.sortedKeys]) else { return "" }
        return String(decoding: data, as: UTF8.self)
    }

    private static func object(_ rule: FilterRule) -> [String: Any] {
        ["field": rule.field, "op": rule.op, "value": rule.value, "from": rule.from, "to": rule.to,
         "amount": rule.amount, "unit": rule.unit]
    }

    /// A copy without the rules missing a value, nor the groups left empty.
    static func clean(_ spec: FilterSpec) -> FilterSpec {
        var cleaned = FilterSpec(match: spec.match)
        for entry in spec.entries {
            switch entry {
            case .rule(let rule):
                if describe(rule) != nil { cleaned.entries.append(entry) }
            case .group(var group):
                group.rules = group.rules.filter { describe($0) != nil }
                if !group.rules.isEmpty { cleaned.entries.append(.group(group)) }
            }
        }
        return cleaned
    }

    /// A rule in words, as a name and a text like `Genre` and `is J-Pop`; `nil` while it
    /// misses a value.
    static func describe(_ rule: FilterRule) -> (name: String, text: String)? {
        guard let field = fields[rule.field] else { return nil }
        let name = shortName(rule.field)
        switch field.kind {
        case .text:
            let value = rule.value.trimmingCharacters(in: .whitespacesAndNewlines)
            return value.isEmpty ? nil : (name, opLabel(rule.op) + " " + value)
        case .source:
            guard let source = FilterCatalog.shared.sourceOption(rule.value) else { return nil }
            return (name, opLabel(rule.op) + " " + source.label)
        case .playlist:
            guard let playlist = Store.shared.playlists.first(where: { String($0.id) == rule.value.trimmingCharacters(in: .whitespaces) })
            else { return nil }
            return (tr(rule.op == "not_in" ? "not_in_playlist" : "in_playlist"), playlist.name)
        case .range:
            let plays = rule.field == "plays"
            let low = rule.op == "until" || rule.op == "at_most" ? "" : rule.from.trimmingCharacters(in: .whitespaces)
            let high = rule.op == "from" || rule.op == "at_least" ? "" : rule.to.trimmingCharacters(in: .whitespaces)
            if !low.isEmpty, !high.isEmpty { return (name, low + " – " + high) }
            if !low.isEmpty { return (name, tr(plays ? "op_at_least" : "op_from") + " " + low) }
            if !high.isEmpty { return (name, tr(plays ? "op_at_most" : "op_until") + " " + high) }
            return nil
        case .period:
            guard let amount = Int(rule.amount.trimmingCharacters(in: .whitespaces)), amount > 0 else { return nil }
            return (name, tr("op_the_last") + " " + Format.count(amount, "unit_\(rule.unit)_one", "unit_\(rule.unit)_many"))
        }
    }

    static func describe(_ entry: FilterEntry) -> (name: String, text: String)? {
        switch entry {
        case .rule(let rule):
            return describe(rule)
        case .group(let group):
            let complete = group.rules.filter { describe($0) != nil }.count
            guard complete > 0 else { return nil }
            return (tr("group_\(group.group)_of"), Format.count(complete, "rules_one", "rules_many"))
        }
    }

    /// The complete rules of a filter in words, joined with ` · `.
    static func summary(_ json: String) -> String {
        parse(json).entries.compactMap(describe).map { $0.name + " " + $0.text }.joined(separator: " · ")
    }
}

/// What the rule editor picks from that the store doesn't keep: the sources as the Sources page
/// lists them (kinds, names, locations). Read again when the library's sources change.
@MainActor
@Observable
final class FilterCatalog {
    static let shared = FilterCatalog()

    struct SourceOption: Identifiable, Equatable {
        var id: String { value }
        /// The source's location: a `source` rule's value.
        let value: String
        let label: String
        let kind: SourceType
    }

    private(set) var sources: [SourceOption] = []
    @ObservationIgnored private var loading: Task<Void, Never>?
    @ObservationIgnored private var loadedFor: [SourceRow]?

    private init() {}

    /// Reads the sources again when the store's changed since the last read.
    func refresh() {
        let store = Store.shared
        guard loadedFor != store.sources else { return }
        loadedFor = store.sources
        loading?.cancel()
        loading = Task {
            let infos = await store.sourceList()
            guard !Task.isCancelled else { return }
            let options = Self.options(infos)
            if options != sources { sources = options }
        }
    }

    @ObservationIgnored private var valueCache: [ValueField: (version: Int, values: [String])] = [:]

    /// The library's values of `field`, for the rule editor's suggestions; read again once the
    /// library changed.
    func values(_ field: ValueField) async -> [String] {
        let version = Store.shared.libraryVersion
        if let cached = valueCache[field], cached.version == version {
            return cached.values
        }
        let values = await Store.shared.fieldValues(field)
        valueCache[field] = (version, values)
        return values
    }

    func sourceOption(_ value: String) -> SourceOption? {
        sources.first { $0.value == value }
    }

    /// The source kinds' headings, in the Add source menu's order.
    static func heading(_ kind: SourceType) -> String {
        switch kind {
        case .folder: tr("local_folders")
        case .web: tr("web_playlists")
        case .telegram: tr("telegram_chats")
        }
    }

    static let kinds: [SourceType] = [.folder, .web, .telegram]

    /// The sources under their kinds; one named like another also names what holds it.
    private static func options(_ infos: [SourceInfo]) -> [SourceOption] {
        var options: [SourceOption] = []
        for kind in kinds {
            for source in infos where source.kind == kind {
                let shared = infos.contains { $0.location != source.location && $0.name == source.name }
                let label = shared ? "\(source.name) (\(holder(source.location)))" : source.name
                options.append(SourceOption(value: source.location, label: label, kind: kind))
            }
        }
        return options
    }

    /// The name of the folder holding `location`, or the server of a playlist at its root.
    private static func holder(_ location: String) -> String {
        var text = location
        if let range = text.range(of: "^https?://", options: [.regularExpression, .caseInsensitive]) {
            text.removeSubrange(range)
        }
        let parts = text.split(whereSeparator: { $0 == "/" || $0 == "\\" }).map(String.init)
        return parts.count > 1 ? parts[parts.count - 2] : location
    }
}
