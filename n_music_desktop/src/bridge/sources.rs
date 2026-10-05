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
        /// `{ name, kind, location, tracks, available, updating }`: `kind` is `folder` or `web`,
        /// `available` false when the folder is gone or the playlist could not be reached, and
        /// `updating` true while it is scanned or waits for a scan.
        #[qproperty(QVariant, items)]
        /// The library reported its sources: `items` lists them.
        #[qproperty(bool, loaded)]
        /// No sources were ever chosen, so the app asks for them before anything else.
        #[qproperty(bool, first_run)]
        type Sources = super::SourcesRust;

        /// Adds the local folder at `path`, as `Catalog.folder` writes it, and scans it.
        #[qinvokable]
        fn add_folder(self: &Sources, path: &QString);
        /// Adds the playlist at `address`, as `webAddress` writes it, and scans it.
        #[qinvokable]
        fn add_web(self: &Sources, address: &QString);
        /// Adds a source of `kind`: `spotify`, `youtube` or `deezer`.
        #[qinvokable]
        fn add(self: &Sources, kind: &QString);
        /// The playlist address typed in `text` as the library writes it, `https://` when it
        /// names no scheme; empty when it is not an `http` or `https` address.
        #[qinvokable]
        fn web_address(self: &Sources, text: &QString) -> QString;
        /// What the source at `location` is called, see `items`.
        #[qinvokable]
        fn name(self: &Sources, location: &QString) -> QString;
        /// Takes the source at `index` and its tracks out of the library.
        #[qinvokable]
        fn remove(self: &Sources, index: i32);
        /// Picks up tracks added, changed or removed in the source at `index`; unchanged ones
        /// come from the cache.
        #[qinvokable]
        fn refresh(self: &Sources, index: i32);
        /// Reads the tags and cover of every track of the source at `index` again, ignoring
        /// the cache.
        #[qinvokable]
        fn reload(self: &Sources, index: i32);
        /// Makes the sources at `locations`, folders and web playlists as `items` writes them,
        /// the library's and scans them, ending the first run.
        #[qinvokable]
        fn set_sources(self: Pin<&mut Sources>, locations: &QStringList);
    }

    impl cxx_qt::Threading for Sources {}
    impl cxx_qt::Initialize for Sources {}
}

use crate::hub::{hub, Changed};
use crate::{bus, settings, worker};
use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QStringList, QVariant};
use n_music_core::library::catalog::Catalog;
use n_music_core::messages::{ScanRequested, SetLibraryRoots};
use n_music_core::source::Locator;
use std::path::{Path, MAIN_SEPARATOR};
use std::time::Duration;

pub struct SourcesRust {
    items: QVariant,
    loaded: bool,
    first_run: bool,
    slot: u64,
}

impl Default for SourcesRust {
    fn default() -> Self {
        Self {
            // A list from the start, so QML can go through it before the sources arrive.
            items: QVariant::from(&QList::<QVariant>::default()),
            loaded: false,
            first_run: settings::first_run(),
            slot: worker::slot(),
        }
    }
}

/// A source as the list shows it.
struct Source {
    name: String,
    kind: &'static str,
    location: String,
    tracks: usize,
    available: bool,
    updating: bool,
}

impl cxx_qt::Initialize for qobject::Sources {
    fn initialize(self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::ROOTS | Changed::TRACKS | Changed::SCAN,
            |sources, _| sources.recount(),
        );
        self.recount();
    }
}

impl qobject::Sources {
    /// Counts the tracks of every source again.
    fn recount(self: Pin<&mut Self>) {
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
            Duration::ZERO,
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
        let mut items = QList::<QVariant>::default();
        for source in sources {
            let mut item = QMap::<QMapPair_QString_QVariant>::default();
            let text = |value: &str| QVariant::from(&QString::from(value));
            item.insert(QString::from("name"), text(&source.name));
            item.insert(QString::from("kind"), text(source.kind));
            item.insert(QString::from("location"), text(&source.location));
            item.insert(
                QString::from("tracks"),
                QVariant::from(&i32::try_from(source.tracks).unwrap_or(i32::MAX)),
            );
            item.insert(
                QString::from("available"),
                QVariant::from(&source.available),
            );
            item.insert(QString::from("updating"), QVariant::from(&source.updating));
            items.append(QVariant::from(&item));
        }
        self.as_mut().set_items(QVariant::from(&items));
        self.set_loaded(true);
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

    fn add(&self, kind: &QString) {
        let service = match kind.to_string().as_str() {
            "spotify" => "Spotify",
            "youtube" => "YouTube",
            "deezer" => "Deezer",
            _ => return,
        };
        unimplemented!(
            "n_music_core cannot use {service} as a source: a Locator is a local path, an Android \
             document or a web address, and Providers has no backend for streaming services"
        );
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

    fn remove(&self, index: i32) {
        let Some(roots) = hub().state().roots.clone() else {
            return;
        };
        let Some(index) = usize::try_from(index)
            .ok()
            .filter(|&index| index < roots.len())
        else {
            return;
        };
        let mut roots = roots.to_vec();
        roots.remove(index);
        bus::emit(SetLibraryRoots(roots));
    }

    fn refresh(&self, index: i32) {
        scan(index, true);
    }

    fn reload(&self, index: i32) {
        scan(index, false);
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

/// Scans the source at `index`; `check_cache` false reads every track again.
fn scan(index: i32, check_cache: bool) {
    let Some(roots) = hub().state().roots.clone() else {
        return;
    };
    if let Some(root) = usize::try_from(index)
        .ok()
        .and_then(|index| roots.get(index))
    {
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
pub fn name(location: &str) -> String {
    match locator(location) {
        root @ Locator::Web(_) => root.display_name(),
        _ => Path::new(location).file_name().map_or_else(
            || location.to_string(),
            |name| name.to_string_lossy().into_owned(),
        ),
    }
}

/// What the paths of the tracks in the folder `root` start with, as the scan writes them.
pub fn prefix(root: &str) -> String {
    if root.ends_with(MAIN_SEPARATOR) {
        root.to_string()
    } else {
        format!("{root}{MAIN_SEPARATOR}")
    }
}

/// Whether the track at a location comes from the source `root`: inside its folder, or listed
/// by its playlist.
pub fn holds<'a>(catalog: &'a Catalog, root: &'a Locator) -> impl Fn(&Locator) -> bool + 'a {
    let prefix = match root {
        Locator::Local(root) => prefix(root),
        _ => String::new(),
    };
    let listed = catalog.listed(root);
    move |track| match (root, track) {
        (Locator::Local(_), Locator::Local(path)) => path.starts_with(&prefix),
        (Locator::Web(_), _) => listed.is_some_and(|listed| listed.tracks.contains(track)),
        _ => false,
    }
}

/// `root` with how many tracks of the library come from it.
fn describe(catalog: &Catalog, root: &Locator, updating: bool) -> Source {
    let location = root.to_string();
    let holds = holds(catalog, root);
    let tracks = catalog
        .tracks()
        .iter()
        .filter(|track| holds(&track.locator))
        .count();
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
        available,
        updating,
    }
}
