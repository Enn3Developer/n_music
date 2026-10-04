//! The library queries of the views, from how QML describes them.

use n_music_core::library::query::{Filter, PlaylistId, Query, SortField, SortKey};
use serde_json::Value;

/// The tracks matching `search` (all when blank) and `filter` (see [`parse_filter`]), in the
/// order of `sort` (see [`parse_sort`]).
pub fn tracks(search: &str, filter: &str, sort: &str) -> Query {
    let mut filters = vec![];
    let search = search.trim();
    if !search.is_empty() {
        filters.push(Filter::Search(search.to_string()));
    }
    filters.extend(parse_filter(filter));
    Query {
        filter: Filter::All(filters),
        sort: parse_sort(sort),
    }
}

/// Reads a sort like `artist,album` or `-year`: field names in order of precedence, each
/// descending after a `-`. Unknown names are skipped.
pub fn parse_sort(sort: &str) -> Vec<SortKey> {
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
                _ => return None,
            };
            Some(SortKey { field, descending })
        })
        .collect()
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

/// A whole number, typed in a text field or not.
fn number(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}
