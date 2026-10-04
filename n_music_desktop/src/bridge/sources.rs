//! Where the library comes from: its sources with their tracks, adding and removing them.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// The sources in the order added, as `{ name, kind, location, tracks, available }`:
        /// `kind` is `folder`, and `available` false when the folder is gone.
        #[qproperty(QVariant, items)]
        type Sources = super::SourcesRust;

        /// Adds the local folder at `path`, as `Catalog.folder` writes it, and scans it.
        #[qinvokable]
        fn add_folder(self: &Sources, path: &QString);
        /// Adds a source of `kind`: `web`, `spotify`, `youtube` or `deezer`.
        #[qinvokable]
        fn add(self: &Sources, kind: &QString);
        /// Takes the source at `index` and its tracks out of the library.
        #[qinvokable]
        fn remove(self: &Sources, index: i32);
    }

    impl cxx_qt::Threading for Sources {}
    impl cxx_qt::Initialize for Sources {}
}

use crate::hub::{hub, Changed};
use crate::{bus, worker};
use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QVariant};
use n_music_core::library::catalog::Catalog;
use n_music_core::messages::SetLibraryRoots;
use n_music_core::source::Locator;
use std::path::{Path, MAIN_SEPARATOR};
use std::time::Duration;

pub struct SourcesRust {
    items: QVariant,
    slot: u64,
}

impl Default for SourcesRust {
    fn default() -> Self {
        Self {
            // A list from the start, so QML can go through it before the sources arrive.
            items: QVariant::from(&QList::<QVariant>::default()),
            slot: worker::slot(),
        }
    }
}

/// A source as the list shows it.
struct Source {
    name: String,
    location: String,
    tracks: usize,
    available: bool,
}

impl cxx_qt::Initialize for qobject::Sources {
    fn initialize(self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::ROOTS | Changed::TRACKS,
            |sources, _| sources.refresh(),
        );
        self.refresh();
    }
}

impl qobject::Sources {
    /// Counts the tracks of every source again.
    fn refresh(self: Pin<&mut Self>) {
        let roots = hub().state().roots.clone();
        let thread = self.qt_thread();
        worker::submit(
            self.slot,
            Duration::ZERO,
            Box::new(move |catalog| {
                let sources: Vec<Source> =
                    roots.iter().map(|root| describe(catalog, root)).collect();
                let _ = thread.queue(move |list| list.show(sources));
            }),
        );
    }

    fn show(self: Pin<&mut Self>, sources: Vec<Source>) {
        let mut items = QList::<QVariant>::default();
        for source in sources {
            let mut item = QMap::<QMapPair_QString_QVariant>::default();
            let text = |value: &str| QVariant::from(&QString::from(value));
            item.insert(QString::from("name"), text(&source.name));
            item.insert(QString::from("kind"), text("folder"));
            item.insert(QString::from("location"), text(&source.location));
            item.insert(
                QString::from("tracks"),
                QVariant::from(&i32::try_from(source.tracks).unwrap_or(i32::MAX)),
            );
            item.insert(
                QString::from("available"),
                QVariant::from(&source.available),
            );
            items.append(QVariant::from(&item));
        }
        self.set_items(QVariant::from(&items));
    }

    fn add_folder(&self, path: &QString) {
        let path = path.to_string();
        if path.is_empty() {
            return;
        }
        let root = Locator::Local(path);
        let mut roots = hub().state().roots.to_vec();
        if !roots.contains(&root) {
            roots.push(root);
            bus::emit(SetLibraryRoots(roots));
        }
    }

    fn add(&self, kind: &QString) {
        let service = match kind.to_string().as_str() {
            "web" => unimplemented!(
                "n_music_core cannot use a web playlist as a source: a Locator is a local path or \
                 an Android document, and Providers has no backend reading M3U or PLS playlists \
                 over HTTPS"
            ),
            "spotify" => "Spotify",
            "youtube" => "YouTube",
            "deezer" => "Deezer",
            _ => return,
        };
        unimplemented!(
            "n_music_core cannot use {service} as a source: a Locator is a local path or an \
             Android document, and Providers has no backend for streaming services"
        );
    }

    fn remove(&self, index: i32) {
        let mut roots = hub().state().roots.to_vec();
        let Some(index) = usize::try_from(index)
            .ok()
            .filter(|&index| index < roots.len())
        else {
            return;
        };
        roots.remove(index);
        bus::emit(SetLibraryRoots(roots));
    }
}

/// `root` with how many tracks of the library come from it.
fn describe(catalog: &Catalog, root: &Locator) -> Source {
    let location = root.to_string();
    let name = Path::new(&location).file_name().map_or_else(
        || location.clone(),
        |name| name.to_string_lossy().into_owned(),
    );
    let (tracks, available) = match root {
        Locator::Local(root) => (
            catalog
                .tracks()
                .iter()
                .filter(
                    |track| matches!(&track.locator, Locator::Local(path) if inside(path, root)),
                )
                .count(),
            Path::new(root).is_dir(),
        ),
        // Android's documents do not come up on the desktop.
        Locator::DocumentTree(_) | Locator::Document { .. } => (0, true),
    };
    Source {
        name,
        location,
        tracks,
        available,
    }
}

/// `path` is in the folder `root`, both written the way the scan writes them.
fn inside(path: &str, root: &str) -> bool {
    path.strip_prefix(root)
        .is_some_and(|rest| rest.starts_with(MAIN_SEPARATOR) || root.ends_with(MAIN_SEPARATOR))
}
