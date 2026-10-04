pragma Singleton
import QtQuick
import NMusic

// What filter rules test and how they read, and the keys tracks sort by.
//
// A filter is `{ match, rules }`, `match` being `all` or `any`. A rule is
// `{ field, op, value, from, to, amount, unit }`, a group of rules `{ group, rules }` with
// `group` being `all`, `any` or `none`; `query::parse_filter` reads them. A sort is a string
// like `artist,-year`, see `query::parse_sort`.
QtObject {
    /// The fields rules test: `kind` picks the value editor, `ops` the conditions, `values` the
    /// library values offered while typing, and `name` how a rule calls it when shorter than
    /// `label`.
    readonly property var fields: ({
            search: {
                label: Tr.t.field_search_long,
                name: Tr.t.field_search,
                kind: "text",
                ops: ["contains", "not_contains"]
            },
            artist: {
                label: Tr.t.field_artist,
                kind: "text",
                ops: ["is", "is_not"],
                values: "artist"
            },
            album_artist: {
                label: Tr.t.field_album_artist,
                kind: "text",
                ops: ["is", "is_not"],
                values: "album_artist"
            },
            album: {
                label: Tr.t.field_album,
                kind: "text",
                ops: ["is", "is_not"],
                values: "album"
            },
            genre: {
                label: Tr.t.field_genre,
                kind: "text",
                ops: ["is", "is_not"],
                values: "genre"
            },
            year: {
                label: Tr.t.field_year,
                kind: "range",
                ops: ["between", "from", "until"]
            },
            codec: {
                label: Tr.t.field_codec,
                kind: "text",
                ops: ["is", "is_not"],
                values: "codec"
            },
            folder: {
                label: Tr.t.field_folder_long,
                name: Tr.t.field_folder,
                kind: "folder",
                ops: ["in", "not_in"],
                values: "folder"
            },
            plays: {
                label: Tr.t.field_plays,
                kind: "range",
                ops: ["at_least", "at_most", "between"]
            },
            played_within: {
                label: Tr.t.field_played_within,
                kind: "period",
                ops: ["the_last"]
            },
            not_played_within: {
                label: Tr.t.field_not_played_within,
                kind: "period",
                ops: ["the_last"]
            }
        })

    /// The fields as choices under headings; a box shows the short name.
    readonly property var fieldOptions: {
        const groups = [[Tr.t.filter_text, ["search"]], [Tr.t.filter_tags, ["artist", "album_artist", "album", "genre", "year", "codec"]], [Tr.t.filter_where, ["folder"]], [Tr.t.filter_listening, ["plays", "played_within", "not_played_within"]]];
        const options = [];
        for (const [heading, names] of groups) {
            options.push({
                heading: heading
            });
            for (const name of names)
                options.push({
                    value: name,
                    label: fields[name].label,
                    short: shortName(name)
                });
        }
        return options;
    }

    readonly property var matchOptions: ["all", "any"].map(match => ({
                value: match,
                label: Tr.t["match_" + match]
            }))

    readonly property var groupOptions: ["all", "any", "none"].map(group => ({
                value: group,
                label: Tr.t["group_" + group]
            }))

    readonly property var unitOptions: ["days", "weeks", "months", "years"].map(unit => ({
                value: unit,
                label: Tr.t["unit_" + unit]
            }))

    /// The fields tracks sort by; `kind` words the directions.
    readonly property var sortFields: ({
            title: {
                label: Tr.t.sort_key_title,
                kind: "text"
            },
            artist: {
                label: Tr.t.sort_key_artist,
                kind: "text"
            },
            album: {
                label: Tr.t.sort_key_album,
                name: Tr.t.sort_key_album_short,
                kind: "text"
            },
            year: {
                label: Tr.t.sort_key_year,
                kind: "date"
            },
            length: {
                label: Tr.t.sort_key_length,
                kind: "number"
            },
            plays: {
                label: Tr.t.sort_key_plays,
                kind: "number"
            },
            lastPlayed: {
                label: Tr.t.sort_key_last_played,
                kind: "date"
            },
            location: {
                label: Tr.t.sort_key_location,
                kind: "text"
            }
        })

    readonly property var sortOptions: Object.keys(sortFields).map(field => ({
                value: field,
                label: sortFields[field].label
            }))

    function shortName(field: string): string {
        const definition = fields[field];
        return definition.name ?? definition.label;
    }

    function empty(): var {
        return {
            match: "all",
            rules: []
        };
    }

    function newRule(field: string): var {
        return {
            field: field,
            op: fields[field].ops[0],
            value: "",
            from: "",
            to: "",
            amount: "30",
            unit: "days"
        };
    }

    function newGroup(): var {
        return {
            group: "any",
            rules: [newRule("artist")]
        };
    }

    /// Makes `rule` test `field` instead, starting its condition and value over.
    function retarget(rule: var, field: string) {
        const fresh = newRule(field);
        for (const key in fresh)
            rule[key] = fresh[key];
    }

    /// A filter from JSON; empty when there is none.
    function parse(json: string): var {
        try {
            const spec = JSON.parse(json);
            if (spec && Array.isArray(spec.rules))
                return spec;
        } catch (error) {}
        return empty();
    }

    /// The filter as JSON; empty without rules.
    function json(spec: var): string {
        return spec.rules.length > 0 ? JSON.stringify(spec) : "";
    }

    /// A copy of the filter without the rules missing a value, nor the groups left empty.
    function clean(spec: var): var {
        const rules = [];
        for (const rule of spec.rules) {
            if (rule.group === undefined) {
                if (describe(rule) !== null)
                    rules.push(rule);
                continue;
            }
            const inner = rule.rules.filter(entry => describe(entry) !== null);
            if (inner.length > 0)
                rules.push({
                    group: rule.group,
                    rules: inner
                });
        }
        return {
            match: spec.match,
            rules: rules
        };
    }

    /// A rule in words, as `{ name, text }` like `Genre` and `is J-Pop`; null while it misses
    /// a value.
    function describe(rule: var): var {
        if (rule.group !== undefined) {
            const complete = rule.rules.filter(entry => describe(entry) !== null).length;
            return complete === 0 ? null : {
                name: Tr.t["group_" + rule.group + "_of"],
                text: Format.count(complete, Tr.t.rules_one, Tr.t.rules_many)
            };
        }
        const field = fields[rule.field];
        if (!field)
            return null;
        const name = shortName(rule.field);
        switch (field.kind) {
        case "text":
        case "folder":
            {
                const value = String(rule.value).trim();
                return value === "" ? null : {
                    name: name,
                    text: Tr.t["op_" + rule.op] + " " + value
                };
            }
        case "range":
            {
                const plays = rule.field === "plays";
                const low = rule.op === "until" || rule.op === "at_most" ? "" : String(rule.from).trim();
                const high = rule.op === "from" || rule.op === "at_least" ? "" : String(rule.to).trim();
                if (low !== "" && high !== "")
                    return {
                        name: name,
                        text: low + " – " + high
                    };
                if (low !== "")
                    return {
                        name: name,
                        text: (plays ? Tr.t.op_at_least : Tr.t.op_from) + " " + low
                    };
                if (high !== "")
                    return {
                        name: name,
                        text: (plays ? Tr.t.op_at_most : Tr.t.op_until) + " " + high
                    };
                return null;
            }
        case "period":
            {
                const amount = parseInt(rule.amount);
                return !(amount > 0) ? null : {
                    name: name,
                    text: Tr.t.op_the_last + " " + Format.count(amount, Tr.t["unit_" + rule.unit + "_one"], Tr.t["unit_" + rule.unit + "_many"])
                };
            }
        }
        return null;
    }

    /// The complete rules of a filter in words, joined with ` · `.
    function summary(json: string): string {
        return parse(json).rules.map(describe).filter(part => part !== null).map(part => part.name + " " + part.text).join(" · ");
    }

    /// A sort as keys `{ field, descending }`, leaving out unknown fields.
    function parseSort(sort: string): var {
        return sort.split(",").map(key => key.trim()).filter(key => sortFields[key.replace(/^-/, "")] !== undefined).map(key => ({
                    field: key.replace(/^-/, ""),
                    descending: key.startsWith("-")
                }));
    }

    function sortString(keys: var): string {
        return keys.map(key => (key.descending ? "-" : "") + key.field).join(",");
    }

    /// A sort in short words, like `Artist ↑, Year ↓`.
    function sortLabel(sort: string): string {
        return parseSort(sort).map(key => {
            const field = sortFields[key.field];
            return (field.name ?? field.label) + (key.descending ? " ↓" : " ↑");
        }).join(", ");
    }

    /// How a field can sort, as choices `up` and `down`.
    function directionOptions(field: string): var {
        const kind = sortFields[field] ? sortFields[field].kind : "text";
        return [
            {
                value: "up",
                label: Tr.t["sort_" + kind + "_up"]
            },
            {
                value: "down",
                label: Tr.t["sort_" + kind + "_down"]
            }
        ];
    }
}
