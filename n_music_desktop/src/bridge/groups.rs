//! The library's tracks grouped by album, artist or genre, for a grid of cards.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
    }

    unsafe extern "C++" {
        include!(<QtCore/QAbstractListModel>);
        type QAbstractListModel = crate::bridge::base::qobject::QAbstractListModel;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QAbstractListModel]
        /// What tracks are grouped by: `album`, `artist`, `genre` or `source`.
        #[qproperty(QString, kind)]
        /// Text a group's name, an album's artist or a source's location contains.
        #[qproperty(QString, search)]
        /// `name`, `artist` (albums), `year` or `-year` (albums), or `-tracks`.
        #[qproperty(QString, sort)]
        /// How many groups are listed.
        #[qproperty(i32, count)]
        /// The first list arrived.
        #[qproperty(bool, ready)]
        type GroupList = super::GroupListRust;

        #[inherit]
        fn begin_reset_model(self: Pin<&mut GroupList>);
        #[inherit]
        fn end_reset_model(self: Pin<&mut GroupList>);

        #[cxx_override]
        fn row_count(self: &GroupList, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &GroupList, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        fn role_names(self: &GroupList) -> QHash_i32_QByteArray;

        /// What opens the group of `row`: `{ name, artist, location }`, the artist being an
        /// album's and the location what a source's tracks' locations start with. The group of
        /// the tracks without an album, an artist or a genre has an empty name.
        #[qinvokable]
        fn key(self: &GroupList, row: i32) -> QVariant;
    }

    impl cxx_qt::Threading for GroupList {}
    impl cxx_qt::Initialize for GroupList {}
}

use crate::bridge::sources;
use crate::hub::{hub, Changed};
use crate::worker;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QMap, QMapPair_QString_QVariant, QModelIndex,
    QString, QVariant,
};
use n_music_core::library::catalog::Catalog;
use n_music_core::source::Locator;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

const NAME: i32 = 0x0100;
const ARTIST: i32 = NAME + 1;
const YEAR: i32 = NAME + 2;
const TRACKS: i32 = NAME + 3;
const COVER: i32 = NAME + 4;
const KNOWN: i32 = NAME + 5;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Album,
    Artist,
    Genre,
    Source,
}

/// Tracks sharing an album, an artist, a genre or a source; `name` is `None` for those lacking
/// one.
#[derive(Clone, PartialEq)]
struct Group {
    name: Option<String>,
    /// An album's artist, or where a source is.
    artist: Option<String>,
    /// What the locations of a source's tracks start with.
    location: Option<String>,
    year: Option<i32>,
    tracks: usize,
    cover: Option<PathBuf>,
}

pub struct GroupListRust {
    kind: QString,
    search: QString,
    sort: QString,
    count: i32,
    ready: bool,
    groups: Vec<Group>,
    slot: u64,
    generation: u64,
}

impl Default for GroupListRust {
    fn default() -> Self {
        Self {
            kind: QString::default(),
            search: QString::default(),
            sort: QString::default(),
            count: 0,
            ready: false,
            groups: vec![],
            slot: worker::slot(),
            generation: 0,
        }
    }
}

impl cxx_qt::Initialize for qobject::GroupList {
    fn initialize(mut self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::TRACKS | Changed::METADATA | Changed::ROOTS,
            |list, changed| {
                let delay = if changed.intersects(Changed::TRACKS | Changed::ROOTS) {
                    Duration::ZERO
                } else {
                    worker::STREAMING
                };
                list.refresh(delay);
            },
        );
        self.as_mut()
            .on_kind_changed(|list| list.refresh(Duration::ZERO))
            .release();
        self.as_mut()
            .on_search_changed(|list| list.refresh(Duration::ZERO))
            .release();
        self.as_mut()
            .on_sort_changed(|list| list.refresh(Duration::ZERO))
            .release();
        // After QML has set the properties.
        let _ = self.qt_thread().queue(|list| list.refresh(Duration::ZERO));
    }
}

impl qobject::GroupList {
    fn kind_value(&self) -> Kind {
        match self.kind.to_string().as_str() {
            "artist" => Kind::Artist,
            "genre" => Kind::Genre,
            "source" => Kind::Source,
            _ => Kind::Album,
        }
    }

    /// Groups the tracks again after `delay`.
    fn refresh(mut self: Pin<&mut Self>, delay: Duration) {
        let kind = self.kind_value();
        let search = self.search.to_string().trim().to_lowercase();
        let sort = self.sort.to_string();
        let roots = hub().state().roots.clone().unwrap_or_default();
        let generation = {
            let mut list = self.as_mut().rust_mut();
            list.generation += 1;
            list.generation
        };
        let thread = self.qt_thread();
        worker::submit(
            self.slot,
            delay,
            Box::new(move |catalog| {
                let groups = group(catalog, kind, &roots, &search, &sort);
                let _ = thread.queue(move |list| list.show(generation, groups));
            }),
        );
    }

    fn show(mut self: Pin<&mut Self>, generation: u64, groups: Vec<Group>) {
        if generation != self.generation || groups == self.groups {
            self.set_ready(true);
            return;
        }
        let count = groups.len();
        self.as_mut().begin_reset_model();
        self.as_mut().rust_mut().groups = groups;
        self.as_mut().end_reset_model();
        self.as_mut()
            .set_count(i32::try_from(count).unwrap_or(i32::MAX));
        self.set_ready(true);
    }

    fn group_at(&self, row: i32) -> Option<&Group> {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.groups.get(row))
    }

    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        i32::try_from(self.groups.len()).unwrap_or(i32::MAX)
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(group) = self.group_at(index.row()) else {
            return QVariant::default();
        };
        let text = |value: &Option<String>| {
            QVariant::from(&QString::from(value.as_deref().unwrap_or_default()))
        };
        match role {
            NAME => text(&group.name),
            ARTIST => text(&group.artist),
            YEAR => QVariant::from(&group.year.unwrap_or(0)),
            TRACKS => QVariant::from(&i32::try_from(group.tracks).unwrap_or(i32::MAX)),
            COVER => QVariant::from(&QString::from(
                &*group
                    .cover
                    .as_deref()
                    .map(|path| path.to_string_lossy())
                    .unwrap_or_default(),
            )),
            KNOWN => QVariant::from(&group.name.is_some()),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for (role, name) in [
            (NAME, "name"),
            (ARTIST, "artist"),
            (YEAR, "year"),
            (TRACKS, "tracks"),
            (COVER, "cover"),
            (KNOWN, "known"),
        ] {
            roles.insert(role, QByteArray::from(name));
        }
        roles
    }

    fn key(&self, row: i32) -> QVariant {
        let Some(group) = self.group_at(row) else {
            return QVariant::default();
        };
        let mut key = QMap::<QMapPair_QString_QVariant>::default();
        for (field, value) in [
            ("name", &group.name),
            ("artist", &group.artist),
            ("location", &group.location),
        ] {
            key.insert(
                QString::from(field),
                QVariant::from(&QString::from(value.as_deref().unwrap_or_default())),
            );
        }
        QVariant::from(&key)
    }
}

/// The groups of `kind`, the sources being `roots`, whose name or artist contains `search`, in
/// the order of `sort`.
fn group(catalog: &Catalog, kind: Kind, roots: &[Locator], search: &str, sort: &str) -> Vec<Group> {
    let mut groups = match kind {
        Kind::Source => roots.iter().map(|root| source(catalog, root)).collect(),
        _ => tagged(catalog, kind),
    };
    if !search.is_empty() {
        let contains = |value: &Option<String>| {
            value
                .as_deref()
                .is_some_and(|value| value.to_lowercase().contains(search))
        };
        groups.retain(|group| contains(&group.name) || contains(&group.artist));
    }
    let text = |value: &Option<String>| value.as_deref().unwrap_or_default().to_lowercase();
    match sort {
        "artist" => groups.sort_by_cached_key(|group| (text(&group.artist), text(&group.name))),
        "year" => groups.sort_by_key(|group| (group.year.is_none(), group.year)),
        "-year" => {
            groups.sort_by_key(|group| (group.year.is_none(), std::cmp::Reverse(group.year)))
        }
        "-tracks" => groups.sort_by_key(|group| std::cmp::Reverse(group.tracks)),
        _ => groups.sort_by_cached_key(|group| text(&group.name)),
    }
    // The tracks lacking a name come last, whatever the order.
    groups.sort_by_key(|group| group.name.is_none());
    groups
}

/// The tracks of the source `root`.
fn source(catalog: &Catalog, root: &Locator) -> Group {
    let location = root.to_string();
    let holds = sources::holds(catalog, root);
    let mut group = Group {
        name: Some(sources::name(&location)),
        artist: Some(location.clone()),
        // What its page goes by: the folder its tracks are in, or the playlist.
        location: Some(match root {
            Locator::Local(_) => sources::prefix(&location),
            _ => location,
        }),
        year: None,
        tracks: 0,
        cover: None,
    };
    for track in catalog.tracks() {
        if holds(&track.locator) {
            group.tracks += 1;
            if group.cover.is_none() {
                group.cover = track.cover.clone();
            }
        }
    }
    group
}

/// The tracks grouped by album, artist or genre.
fn tagged(catalog: &Catalog, kind: Kind) -> Vec<Group> {
    // Keyed by lower case, so `J-Pop` and `j-pop` group together.
    let mut groups: Vec<Group> = vec![];
    let mut index: HashMap<(String, String), usize> = HashMap::new();
    let mut add = |name: Option<&str>, artist: Option<&str>, track: &n_music_core::Track| {
        let key = (
            name.map(str::to_lowercase).unwrap_or_default(),
            artist.map(str::to_lowercase).unwrap_or_default(),
        );
        let position = *index.entry(key).or_insert_with(|| {
            groups.push(Group {
                name: name.map(String::from),
                artist: artist.map(String::from),
                location: None,
                year: None,
                tracks: 0,
                cover: None,
            });
            groups.len() - 1
        });
        let group = &mut groups[position];
        group.tracks += 1;
        group.year = group.year.or(track.year);
        if group.cover.is_none() {
            group.cover = track.cover.clone();
        }
    };
    for track in catalog.tracks() {
        match kind {
            Kind::Album => {
                let artist = track
                    .album_artist
                    .as_deref()
                    .or(track.artists.first().map(String::as_str));
                match track.album.as_deref() {
                    Some(album) => add(Some(album), artist, track),
                    // One group for every track without an album.
                    None => add(None, None, track),
                }
            }
            Kind::Artist if track.artists.is_empty() => add(None, None, track),
            Kind::Artist => {
                for artist in &track.artists {
                    add(Some(artist), None, track);
                }
            }
            Kind::Genre if track.genres.is_empty() => add(None, None, track),
            Kind::Genre => {
                for genre in &track.genres {
                    add(Some(genre), None, track);
                }
            }
            // Listed by `source`.
            Kind::Source => {}
        }
    }
    groups
}
