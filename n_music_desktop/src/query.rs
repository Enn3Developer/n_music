//! The library queries of the views, from how QML describes them.

use n_music_core::library::query::{Filter, Query, SortField, SortKey};

/// The tracks matching `search` (all when blank), in the order of `sort`.
pub fn tracks(search: &str, sort: &str) -> Query {
    let mut filters = vec![];
    let search = search.trim();
    if !search.is_empty() {
        filters.push(Filter::Search(search.to_string()));
    }
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
