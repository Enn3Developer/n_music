//! Where the library comes from: its sources with their tracks, adding and removing them.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// The sources in the order added, as
        /// `{ name, kind, location, tracks, cover, available, updating }`: `kind` is `folder` or
        /// `web`, `tracks` how many tracks of the library it listed, `cover` the cover of one of
        /// them or empty, `available` false when the folder is gone or the playlist could not be
        /// reached, and `updating` true while it is scanned or waits for a scan.
        #[qproperty(QVariant, items)]
        /// The library reported its sources: `items` lists them.
        #[qproperty(bool, loaded)]
        /// No sources were ever chosen, so the app asks for them before anything else.
        #[qproperty(bool, first_run)]
        /// Tracks streamed from web playlists are cached on disk as they play, see `changeCache`.
        #[qproperty(bool, cache_enabled)]
        /// The most the cached tracks may take, in bytes.
        #[qproperty(f64, cache_limit)]
        /// What the cached tracks take, in bytes.
        #[qproperty(f64, cache_used)]
        type Sources = super::SourcesRust;

        /// Adds the local folder at `path`, as `Catalog.folder` writes it, and scans it.
        #[qinvokable]
        fn add_folder(self: &Sources, path: &QString);
        /// Adds the playlist at `address`, as `webAddress` writes it, and scans it.
        #[qinvokable]
        fn add_web(self: &Sources, address: &QString);
        /// The playlist address typed in `text` as the library writes it, `https://` when it
        /// names no scheme; empty when it is not an `http` or `https` address.
        #[qinvokable]
        fn web_address(self: &Sources, text: &QString) -> QString;
        /// What the source at `location` is called, see `items`.
        #[qinvokable]
        fn name(self: &Sources, location: &QString) -> QString;
        /// The `file:` URL of the folder at `location`, to open it elsewhere.
        #[qinvokable]
        fn folder_url(self: &Sources, location: &QString) -> QString;
        /// Takes the source at `location`, as `items` writes it, and its tracks out of the
        /// library.
        #[qinvokable]
        fn remove(self: &Sources, location: &QString);
        /// Picks up tracks added, changed or removed in the source at `location`; unchanged
        /// ones come from the cache.
        #[qinvokable]
        fn refresh(self: &Sources, location: &QString);
        /// Reads the tags and cover of every track of the source at `location` again, ignoring
        /// the cache.
        #[qinvokable]
        fn reload(self: &Sources, location: &QString);
        /// Makes the sources at `locations`, folders and web playlists as `items` writes them,
        /// the library's and scans them, ending the first run.
        #[qinvokable]
        fn set_sources(self: Pin<&mut Sources>, locations: &QStringList);
        /// Caches streamed tracks on disk as they play, or (`enabled` false) deletes them, and
        /// lets them take at most `limit` bytes: the most played stay.
        #[qinvokable]
        fn change_cache(self: &Sources, enabled: bool, limit: f64);
    }

    impl cxx_qt::Threading for Sources {}
    impl cxx_qt::Initialize for Sources {}
}

use crate::hub::{hub, Changed};
use crate::{bus, settings, worker};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QStringList, QUrl, QVariant};
use n_music_core::library::catalog::Catalog;
use n_music_core::messages::{ScanRequested, SetLibraryRoots, SetStreamCache};
use n_music_core::settings::StreamCacheSettings;
use n_music_core::source::Locator;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct SourcesRust {
    items: QVariant,
    loaded: bool,
    first_run: bool,
    cache_enabled: bool,
    cache_limit: f64,
    cache_used: f64,
    slot: u64,
    /// What `items` lists.
    shown: Vec<Source>,
}

impl Default for SourcesRust {
    fn default() -> Self {
        let cache = StreamCacheSettings::default();
        Self {
            // A list from the start, so QML can go through it before the sources arrive.
            items: QVariant::from(&QList::<QVariant>::default()),
            loaded: false,
            first_run: settings::first_run(),
            cache_enabled: cache.enabled,
            cache_limit: cache.limit as f64,
            cache_used: 0.0,
            slot: worker::slot(),
            shown: vec![],
        }
    }
}

/// A source as the list shows it.
#[derive(PartialEq)]
struct Source {
    name: String,
    kind: &'static str,
    location: String,
    tracks: usize,
    cover: Option<PathBuf>,
    available: bool,
    updating: bool,
}

impl cxx_qt::Initialize for qobject::Sources {
    fn initialize(mut self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::ROOTS | Changed::TRACKS | Changed::SCAN | Changed::METADATA,
            |sources, changed| {
                // Covers come in with the metadata a scan streams.
                let delay = if changed == Changed::METADATA {
                    worker::STREAMING
                } else {
                    Duration::ZERO
                };
                sources.recount(delay);
            },
        );
        hub().watch(self.qt_thread(), Changed::CACHE, |sources, _| {
            sources.show_cache()
        });
        self.as_mut().show_cache();
        self.recount(Duration::ZERO);
    }
}

impl qobject::Sources {
    /// Counts the tracks of every source again after `delay`.
    fn recount(self: Pin<&mut Self>, delay: Duration) {
        let (roots, updating) = {
            let state = hub().state();
            let Some(roots) = state.roots.clone() else {
                return;
            };
            (roots, state.updating.clone())
        };
        let thread = self.qt_thread();
        worker::submit(
            self.slot,
            delay,
            Box::new(move |catalog| {
                let sources: Vec<Source> = roots
                    .iter()
                    .map(|root| describe(catalog, root, updating.contains(root)))
                    .collect();
                let _ = thread.queue(move |list| list.show(sources));
            }),
        );
    }

    fn show(mut self: Pin<&mut Self>, sources: Vec<Source>) {
        // Unchanged, the views keep what they show, covers and all.
        if self.loaded && sources == self.shown {
            return;
        }
        let mut items = QList::<QVariant>::default();
        for source in &sources {
            let mut item = QMap::<QMapPair_QString_QVariant>::default();
            let text = |value: &str| QVariant::from(&QString::from(value));
            item.insert(QString::from("name"), text(&source.name));
            item.insert(QString::from("kind"), text(source.kind));
            item.insert(QString::from("location"), text(&source.location));
            item.insert(
                QString::from("tracks"),
                QVariant::from(&i32::try_from(source.tracks).unwrap_or(i32::MAX)),
            );
            let cover = source.cover.as_deref().map(Path::to_string_lossy);
            item.insert(
                QString::from("cover"),
                text(cover.as_deref().unwrap_or_default()),
            );
            item.insert(
                QString::from("available"),
                QVariant::from(&source.available),
            );
            item.insert(QString::from("updating"), QVariant::from(&source.updating));
            items.append(QVariant::from(&item));
        }
        self.as_mut().rust_mut().shown = sources;
        self.as_mut().set_items(QVariant::from(&items));
        self.set_loaded(true);
    }

    /// Shows the stream cache as the core last reported it.
    fn show_cache(mut self: Pin<&mut Self>) {
        let Some(cache) = hub().state().cache else {
            return;
        };
        self.as_mut().set_cache_enabled(cache.enabled);
        self.as_mut().set_cache_limit(cache.limit as f64);
        self.set_cache_used(cache.used as f64);
    }

    fn add_folder(&self, path: &QString) {
        if !path.is_empty() {
            add(Locator::Local(path.to_string()));
        }
    }

    fn add_web(&self, address: &QString) {
        if let Some(root) = Locator::web(&address.to_string()) {
            add(root);
        }
    }

    fn web_address(&self, text: &QString) -> QString {
        let text = text.to_string();
        let text = text.trim();
        let address = if text.contains("://") {
            Locator::web(text)
        } else {
            Locator::web(&format!("https://{text}"))
        };
        address.map_or_else(QString::default, |address| {
            QString::from(&address.to_string())
        })
    }

    fn name(&self, location: &QString) -> QString {
        QString::from(&name(&location.to_string()))
    }

    fn folder_url(&self, location: &QString) -> QString {
        QUrl::from_local_file(location).to_qstring()
    }

    fn remove(&self, location: &QString) {
        let Some(roots) = hub().state().roots.clone() else {
            return;
        };
        let location = location.to_string();
        if roots.iter().any(|root| root.to_string() == location) {
            let mut roots = roots.to_vec();
            roots.retain(|root| root.to_string() != location);
            bus::emit(SetLibraryRoots(roots));
        }
    }

    fn refresh(&self, location: &QString) {
        scan(&location.to_string(), true);
    }

    fn reload(&self, location: &QString) {
        scan(&location.to_string(), false);
    }

    fn set_sources(self: Pin<&mut Self>, locations: &QStringList) {
        let mut roots: Vec<Locator> = vec![];
        for location in QList::<QString>::from(locations).iter() {
            let root = locator(&location.to_string());
            if !location.is_empty() && !roots.contains(&root) {
                roots.push(root);
            }
        }
        bus::emit(SetLibraryRoots(roots));
        self.set_first_run(false);
    }

    fn change_cache(&self, enabled: bool, limit: f64) {
        bus::emit(SetStreamCache {
            enabled,
            limit: limit.max(0.0) as u64,
        });
    }
}

/// Adds `root` to the sources, unless it is one.
fn add(root: Locator) {
    let Some(roots) = hub().state().roots.clone() else {
        return;
    };
    if !roots.contains(&root) {
        let mut roots = roots.to_vec();
        roots.push(root);
        bus::emit(SetLibraryRoots(roots));
    }
}

/// Scans the source at `location`; `check_cache` false reads every track again.
fn scan(location: &str, check_cache: bool) {
    let Some(roots) = hub().state().roots.clone() else {
        return;
    };
    if let Some(root) = roots.iter().find(|root| root.to_string() == location) {
        bus::emit(ScanRequested {
            library: Some(root.clone()),
            check_cache,
        });
    }
}

/// The source at `location`, as `Sources.items` writes it: a web playlist or a folder.
pub fn locator(location: &str) -> Locator {
    match Locator::web(location) {
        Some(_) => Locator::Web(location.to_string()),
        None => Locator::Local(location.to_string()),
    }
}

/// What the source at `location` is called: its folder's name, or its playlist's.
fn name(location: &str) -> String {
    match locator(location) {
        root @ Locator::Web(_) => root.display_name(),
        _ => Path::new(location).file_name().map_or_else(
            || location.to_string(),
            |name| name.to_string_lossy().into_owned(),
        ),
    }
}

/// `root` with how many tracks of the library it listed, as its page's `source` rule finds
/// them, and the cover of the first with one.
fn describe(catalog: &Catalog, root: &Locator, updating: bool) -> Source {
    let location = root.to_string();
    let listed = catalog.listed(root);
    let mut tracks = 0;
    let mut cover = None;
    for track in catalog.tracks() {
        if listed.is_some_and(|listed| listed.tracks.contains(&track.locator)) {
            tracks += 1;
            if cover.is_none() {
                cover = track.cover.clone();
            }
        }
    }
    let (kind, available) = match root {
        Locator::Local(root) => ("folder", Path::new(root).is_dir()),
        // Unknown before its first listing.
        Locator::Web(_) => (
            "web",
            catalog.listed(root).is_none_or(|listed| listed.reachable),
        ),
        // Android's documents do not come up on the desktop.
        Locator::DocumentTree(_) | Locator::Document { .. } => ("folder", true),
    };
    Source {
        name: name(&location),
        kind,
        location,
        tracks,
        cover,
        available,
        updating,
    }
}
