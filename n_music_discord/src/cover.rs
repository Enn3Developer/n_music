//! Cover art Discord can show: it takes only URLs, so covers are looked up on iTunes by tags.

use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

const SEARCH: &str = "https://itunes.apple.com/search";
/// iTunes serves its artwork in any size, named in the URL.
const SIZE: &str = "512x512bb";

pub(crate) struct Covers {
    agent: ureq::Agent,
    /// Found URLs by search, `None` for searches that found nothing.
    found: HashMap<String, Option<String>>,
}

impl Default for Covers {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(5)))
            .build()
            .into();
        Self {
            agent,
            found: HashMap::new(),
        }
    }
}

impl Covers {
    /// The URL of the cover of `album`, or of the track `title` when its album is unknown.
    pub(crate) fn find(
        &mut self,
        artist: &str,
        album: Option<&str>,
        title: &str,
    ) -> Option<String> {
        if artist.is_empty() {
            return None;
        }
        let (entity, name) = match album {
            Some(album) => ("album", album),
            None => ("song", title),
        };
        let term = format!("{artist} {name}");
        if let Some(url) = self.found.get(&term) {
            return url.clone();
        }
        let url = self
            .search(&term, entity)
            .inspect_err(|error| log::debug!("Could not look up the cover of {term:?}: {error}"))
            .ok()
            .flatten();
        self.found.insert(term, url.clone());
        url
    }

    fn search(
        &self,
        term: &str,
        entity: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let body = self
            .agent
            .get(SEARCH)
            .query("term", term)
            .query("entity", entity)
            .query("limit", "1")
            .call()?
            .body_mut()
            .read_to_string()?;
        let results: Value = serde_json::from_str(&body)?;
        Ok(results["results"][0]["artworkUrl100"]
            .as_str()
            .map(|url| url.replace("100x100bb", SIZE)))
    }
}
