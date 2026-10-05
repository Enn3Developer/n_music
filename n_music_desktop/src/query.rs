//! The library queries of the views, from how QML describes them.

use n_music_core::library::query::{Filter, PlaylistId, Query, SortField, SortKey};
use serde_json::{json, Value};

/// The tracks of `playlist` (all without one) matching `search` (all when blank) and `filter`
/// (see [`parse_filter`]), in the order of `sort` (see [`parse_sort`]).
pub fn tracks(search: &str, filter: &str, sort: &str, playlist: Option<PlaylistId>) -> Query {
    let mut filters: Vec<Filter> = playlist.map(Filter::Playlist).into_iter().collect();
    let search = search.trim();
    if !search.is_empty() {
        filters.push(Filter::Search(search.to_string()));
    }
    filters.extend(parse_filter(filter));
    Query {
        filter: Filter::All(filters),
        sort: parse_sort(sort, playlist),
    }
}

/// Reads a sort like `artist,album` or `-year`: field names in order of precedence, each
/// descending after a `-`. `added` sorts by when tracks were added to `playlist`. Unknown
/// names are skipped.
pub fn parse_sort(sort: &str, playlist: Option<PlaylistId>) -> Vec<SortKey> {
    sort.split(',')
        .filter_map(|key| {
            let key = key.trim();
            let (descending, name) = match key.strip_prefix('-') {
                Some(name) => (true, name),
                None => (false, key),
            };
            let field = match name {
                "title" => SortField::Title,
                "artist" => SortField::Artist,
                "album" => SortField::Album,
                "year" => SortField::Year,
                "length" => SortField::Length,
                "plays" => SortField::Plays,
                "lastPlayed" => SortField::LastPlayed,
                "location" => SortField::Location,
                "added" => SortField::Added(playlist?),
                "genre" => unimplemented!(
                    "n_music_core cannot sort by genre: SortField has no genre, so the Genre \
                     column cannot order the tracks"
                ),
                "format" => unimplemented!(
                    "n_music_core cannot sort by format: SortField has no codec, so the Format \
                     column cannot order the tracks"
                ),
                _ => return None,
            };
            Some(SortKey { field, descending })
        })
        .collect()
}

/// A sort as [`parse_sort`] reads it.
pub fn sort_string(keys: &[SortKey]) -> String {
    let names: Vec<String> = keys
        .iter()
        .map(|key| {
            let name = match key.field {
                SortField::Title => "title",
                SortField::Artist => "artist",
                SortField::Album => "album",
                SortField::Year => "year",
                SortField::Length => "length",
                SortField::Plays => "plays",
                SortField::LastPlayed => "lastPlayed",
                SortField::Added(_) => "added",
                SortField::Location => "location",
            };
            format!("{}{name}", if key.descending { "-" } else { "" })
        })
        .collect();
    names.join(",")
}

/// Reads a filter as the filter editor writes it:
///
/// ```json
/// {"match": "all", "rules": [
///     {"field": "genre", "op": "is", "value": "J-Pop"},
///     {"group": "any", "rules": [{"field": "artist", "op": "is", "value": "Neru"}]}
/// ]}
/// ```
///
/// `match` is `all` or `any`, a group's `all`, `any` or `none`. Rules missing a value are left
/// out, as the editor shows them while they are being written; `None` when nothing is left.
pub fn parse_filter(json: &str) -> Option<Filter> {
    let spec: Value = serde_json::from_str(json).ok()?;
    let rules: Vec<Filter> = spec["rules"].as_array()?.iter().filter_map(entry).collect();
    if rules.is_empty() {
        return None;
    }
    Some(match spec["match"].as_str() {
        Some("any") => Filter::Any(rules),
        _ => Filter::All(rules),
    })
}

/// A rule or a group of rules.
fn entry(entry: &Value) -> Option<Filter> {
    let Some(group) = entry["group"].as_str() else {
        return rule(entry);
    };
    let rules: Vec<Filter> = entry["rules"].as_array()?.iter().filter_map(rule).collect();
    if rules.is_empty() {
        return None;
    }
    Some(match group {
        "any" => Filter::Any(rules),
        "none" => Filter::Not(Box::new(Filter::Any(rules))),
        _ => Filter::All(rules),
    })
}

fn rule(rule: &Value) -> Option<Filter> {
    let op = rule["op"].as_str().unwrap_or_default();
    let text = || {
        rule["value"]
            .as_str()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(String::from)
    };
    let not = |filter: Filter, negated: bool| {
        if negated {
            Filter::Not(Box::new(filter))
        } else {
            filter
        }
    };
    let filter = match rule["field"].as_str()? {
        "search" => not(Filter::Search(text()?), op == "not_contains"),
        "artist" => not(Filter::Artist(text()?), op == "is_not"),
        "album_artist" => not(Filter::AlbumArtist(text()?), op == "is_not"),
        "album" => not(Filter::Album(text()?), op == "is_not"),
        "genre" => not(Filter::Genre(text()?), op == "is_not"),
        "codec" => not(Filter::Codec(text()?), op == "is_not"),
        "folder" => not(Filter::Folder(text()?), op == "not_in"),
        "playlist" => not(
            Filter::Playlist(PlaylistId(number(&rule["value"])?)),
            op == "not_in",
        ),
        "year" => {
            let (from, to) = range(rule, op)?;
            Filter::Year {
                from: from.and_then(|year| i32::try_from(year).ok()),
                to: to.and_then(|year| i32::try_from(year).ok()),
            }
        }
        "plays" => {
            let (min, max) = range(rule, op)?;
            Filter::Plays {
                min: min.and_then(|plays| u32::try_from(plays).ok()),
                max: max.and_then(|plays| u32::try_from(plays).ok()),
            }
        }
        "played_within" => Filter::PlayedWithin {
            seconds: period(rule)?,
        },
        "not_played_within" => Filter::NotPlayedWithin {
            seconds: period(rule)?,
        },
        _ => return None,
    };
    Some(filter)
}

/// The bounds a range rule's `op` uses: `between` both, `from`/`at_least` the lower one,
/// `until`/`at_most` the upper one. `None` without any.
fn range(rule: &Value, op: &str) -> Option<(Option<i64>, Option<i64>)> {
    let (from, to) = (number(&rule["from"]), number(&rule["to"]));
    let bounds = match op {
        "from" | "at_least" => (from, None),
        "until" | "at_most" => (None, to),
        _ => (from, to),
    };
    (bounds.0.is_some() || bounds.1.is_some()).then_some(bounds)
}

/// `amount` of `unit`s (days, weeks, months or years) in seconds.
fn period(rule: &Value) -> Option<u64> {
    let amount = u64::try_from(number(&rule["amount"])?).ok()?;
    let unit = match rule["unit"].as_str()? {
        "days" => 1,
        "weeks" => 7,
        "months" => 30,
        "years" => 365,
        _ => return None,
    };
    amount.checked_mul(unit * 24 * 60 * 60)
}

/// A filter as the filter editor writes it (see [`parse_filter`]); `None` when it has a shape
/// the editor cannot show, like groups inside groups.
pub fn filter_spec(filter: &Filter) -> Option<String> {
    let (all, entries) = match filter {
        Filter::All(entries) => (true, entries.as_slice()),
        Filter::Any(entries) => (false, entries.as_slice()),
        other => (true, std::slice::from_ref(other)),
    };
    let rules = entries
        .iter()
        .map(|entry| match entry {
            Filter::All(rules) => group("all", rules),
            Filter::Any(rules) => group("any", rules),
            Filter::Not(inner) => match inner.as_ref() {
                Filter::Any(rules) => group("none", rules),
                _ => rule_spec(entry),
            },
            _ => rule_spec(entry),
        })
        .collect::<Option<Vec<Value>>>()?;
    let spec = json!({"match": if all { "all" } else { "any" }, "rules": rules});
    Some(spec.to_string())
}

fn group(kind: &str, rules: &[Filter]) -> Option<Value> {
    let rules = rules
        .iter()
        .map(rule_spec)
        .collect::<Option<Vec<Value>>>()?;
    Some(json!({"group": kind, "rules": rules}))
}

/// A single rule; `None` for groups and for what the editor has no field for.
fn rule_spec(filter: &Filter) -> Option<Value> {
    let (negated, filter) = match filter {
        Filter::Not(inner) => (true, inner.as_ref()),
        other => (false, other),
    };
    let text = |field: &str, ops: [&str; 2], value: &str| json!({"field": field, "op": ops[usize::from(negated)], "value": value});
    let bound = |value: Option<i64>| value.map_or_else(String::new, |value| value.to_string());
    let spec = match filter {
        Filter::Search(value) => text("search", ["contains", "not_contains"], value),
        Filter::Artist(value) => text("artist", ["is", "is_not"], value),
        Filter::AlbumArtist(value) => text("album_artist", ["is", "is_not"], value),
        Filter::Album(value) => text("album", ["is", "is_not"], value),
        Filter::Genre(value) => text("genre", ["is", "is_not"], value),
        Filter::Codec(value) => text("codec", ["is", "is_not"], value),
        Filter::Folder(value) => text("folder", ["in", "not_in"], value),
        Filter::Playlist(id) => text("playlist", ["in", "not_in"], &id.0.to_string()),
        _ if negated => return None,
        Filter::Year { from, to } => {
            let op = match (from, to) {
                (Some(_), Some(_)) => "between",
                (Some(_), None) => "from",
                (None, Some(_)) => "until",
                (None, None) => return None,
            };
            json!({"field": "year", "op": op, "from": bound(from.map(i64::from)), "to": bound(to.map(i64::from))})
        }
        Filter::Plays { min, max } => {
            let op = match (min, max) {
                (Some(_), Some(_)) => "between",
                (Some(_), None) => "at_least",
                (None, Some(_)) => "at_most",
                (None, None) => return None,
            };
            json!({"field": "plays", "op": op, "from": bound(min.map(i64::from)), "to": bound(max.map(i64::from))})
        }
        Filter::PlayedWithin { seconds } => period_spec("played_within", *seconds),
        Filter::NotPlayedWithin { seconds } => period_spec("not_played_within", *seconds),
        _ => return None,
    };
    Some(spec)
}

/// A period in the largest unit [`period`] reads it back exactly in.
fn period_spec(field: &str, seconds: u64) -> Value {
    let days = seconds / (24 * 60 * 60);
    let (amount, unit) = [(365, "years"), (30, "months"), (7, "weeks")]
        .into_iter()
        .find(|(length, _)| days > 0 && days.is_multiple_of(*length))
        .map_or((days, "days"), |(length, unit)| (days / length, unit));
    json!({"field": field, "op": "the_last", "amount": amount.to_string(), "unit": unit})
}

/// A whole number, typed in a text field or not.
fn number(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}
