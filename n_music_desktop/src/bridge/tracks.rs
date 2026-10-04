//! Tracks of the library for a list view: a search over it, in a chosen order.

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
        include!("cxx-qt-lib/qlist.h");
        type QList_i32 = cxx_qt_lib::QList<i32>;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
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
        /// Text the title, an artist or the album contains.
        #[qproperty(QString, search)]
        /// Rules over the tracks, as JSON: see `query::parse_filter`.
        #[qproperty(QString, filter)]
        /// A playlist to list the tracks of, by id; 0 for the whole library.
        #[qproperty(i64, playlist)]
        /// Field names in order of precedence, each descending after a `-`: `artist,album`.
        /// `added` sorts by when tracks were added to the playlist.
        #[qproperty(QString, sort)]
        /// What the queue calls a session played from this list, like `Tracks`.
        #[qproperty(QString, label)]
        /// How the list is narrowed down in words, for the queue: the search and the rules.
        #[qproperty(QString, detail)]
        /// The page showing this list, for the queue to open.
        #[qproperty(QString, origin)]
        /// How many tracks are listed.
        #[qproperty(i32, count)]
        /// How many tracks the library has.
        #[qproperty(i32, total)]
        /// The length of the listed tracks, in seconds.
        #[qproperty(f64, duration)]
        /// The row of the current track; -1 when it is not listed.
        #[qproperty(i32, current_row)]
        /// The first list arrived.
        #[qproperty(bool, ready)]
        /// Up to four covers of the listed tracks, each once, for a mosaic.
        #[qproperty(QStringList, covers)]
        type TrackList = super::TrackListRust;

        #[inherit]
        fn begin_reset_model(self: Pin<&mut TrackList>);
        #[inherit]
        fn end_reset_model(self: Pin<&mut TrackList>);
        #[inherit]
        fn index(self: &TrackList, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;
        #[inherit]
        #[qsignal]
        fn data_changed(
            self: Pin<&mut TrackList>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QList_i32,
        );

        #[cxx_override]
        fn row_count(self: &TrackList, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &TrackList, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        fn role_names(self: &TrackList) -> QHash_i32_QByteArray;

        /// Plays the listed tracks from `row`.
        #[qinvokable]
        fn play(self: &TrackList, row: i32);
        /// Plays the listed tracks from the first, in order or shuffled.
        #[qinvokable]
        fn play_all(self: &TrackList, shuffle: bool);
        /// Queues the track of `row` after the current one; `next` before what is queued.
        #[qinvokable]
        fn enqueue(self: &TrackList, row: i32, next: bool);
        /// Adds the track of `row` to playlist `id`.
        #[qinvokable]
        fn add_to_playlist(self: &TrackList, row: i32, id: i64);
        /// Creates a playlist named `name` holding the track of `row`.
        #[qinvokable]
        fn add_to_new_playlist(self: &TrackList, row: i32, name: &QString);
        /// Removes the track of `row` from the playlist listed.
        #[qinvokable]
        fn remove_from_playlist(self: &TrackList, row: i32);
        /// The ids of the playlists holding the track of `row` as an added track.
        #[qinvokable]
        fn playlists_with(self: &TrackList, row: i32) -> QVariant;
        /// What the track of `row` belongs to, as `{ album, albumArtist, artist }`; the album's
        /// artist falls back to the first artist, and each is empty when unknown.
        #[qinvokable]
        fn about(self: &TrackList, row: i32) -> QVariant;
    }

    impl cxx_qt::Threading for TrackList {}
    impl cxx_qt::Initialize for TrackList {}
}

use crate::hub::{hub, Changed, Context};
use crate::{bus, format, query, worker};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QList, QMap, QMapPair_QString_QVariant,
    QModelIndex, QString, QStringList, QVariant,
};
use n_music_core::library::query::{PlaylistId, Query};
use n_music_core::messages::{
    AddToPlaylist, CreatePlaylist, Enqueue, PlayFrom, RemoveFromPlaylist, SetShuffle,
};
use n_music_core::source::Locator;
use n_music_core::Track;
use std::sync::Arc;
use std::time::Duration;

const TITLE: i32 = 0x0100;
const ARTIST: i32 = TITLE + 1;
const ALBUM: i32 = TITLE + 2;
const YEAR: i32 = TITLE + 3;
const PLAYS: i32 = TITLE + 4;
const LENGTH: i32 = TITLE + 5;
const COVER: i32 = TITLE + 6;
const CURRENT: i32 = TITLE + 7;
const LAST_PLAYED: i32 = TITLE + 8;
const ADDED: i32 = TITLE + 9;

struct Row {
    track: Track,
    plays: u32,
    /// Unix seconds; 0 for never.
    last_played: i64,
    /// When the track was added to the playlist, in Unix seconds; 0 when not added.
    added: i64,
}

impl Row {
    fn same(&self, other: &Row) -> bool {
        Arc::ptr_eq(&self.track, &other.track)
            && self.plays == other.plays
            && self.last_played == other.last_played
            && self.added == other.added
    }
}

pub struct TrackListRust {
    search: QString,
    filter: QString,
    playlist: i64,
    sort: QString,
    label: QString,
    detail: QString,
    origin: QString,
    count: i32,
    total: i32,
    duration: f64,
    current_row: i32,
    ready: bool,
    covers: QStringList,
    rows: Vec<Row>,
    /// The current track, highlighted where listed.
    current: Option<Locator>,
    /// The query thread's slot for this list.
    slot: u64,
    /// Counts the queries sent; only the latest one's tracks are shown.
    generation: u64,
}

impl Default for TrackListRust {
    fn default() -> Self {
        Self {
            search: QString::default(),
            filter: QString::default(),
            playlist: 0,
            sort: QString::default(),
            label: QString::default(),
            detail: QString::default(),
            origin: QString::default(),
            count: 0,
            total: 0,
            duration: 0.0,
            current_row: -1,
            ready: false,
            covers: QStringList::default(),
            rows: vec![],
            current: None,
            slot: worker::slot(),
            generation: 0,
        }
    }
}

impl cxx_qt::Initialize for qobject::TrackList {
    fn initialize(mut self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::TRACKS
                | Changed::METADATA
                | Changed::STATS
                | Changed::CURRENT
                | Changed::PLAYLISTS,
            Self::changed,
        );
        self.as_mut()
            .on_search_changed(|list| list.refresh(Duration::ZERO))
            .release();
        self.as_mut()
            .on_filter_changed(|list| list.refresh(Duration::ZERO))
            .release();
        self.as_mut()
            .on_playlist_changed(|list| list.refresh(Duration::ZERO))
            .release();
        self.as_mut()
            .on_sort_changed(|list| list.refresh(Duration::ZERO))
            .release();
        // After QML has set the properties.
        let _ = self.qt_thread().queue(|mut list| {
            list.as_mut().refresh(Duration::ZERO);
            list.update_current();
        });
    }
}

impl qobject::TrackList {
    fn changed(mut self: Pin<&mut Self>, changed: Changed) {
        if changed.intersects(Changed::TRACKS | Changed::STATS | Changed::PLAYLISTS) {
            self.as_mut().refresh(Duration::ZERO);
        } else if changed.contains(Changed::METADATA) {
            self.as_mut().refresh(worker::STREAMING);
        }
        if changed.contains(Changed::CURRENT) {
            self.update_current();
        }
    }

    fn playlist_id(&self) -> Option<PlaylistId> {
        (self.playlist > 0).then_some(PlaylistId(self.playlist))
    }

    fn query(&self) -> Query {
        query::tracks(
            &self.search.to_string(),
            &self.filter.to_string(),
            &self.sort.to_string(),
            self.playlist_id(),
        )
    }

    /// Runs the query again after `delay`.
    fn refresh(mut self: Pin<&mut Self>, delay: Duration) {
        let query = self.query();
        let playlist = self.playlist_id();
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
                let items = playlist
                    .and_then(|id| catalog.playlist(id))
                    .map(|playlist| &playlist.items);
                let rows: Vec<Row> = catalog
                    .select(&query)
                    .into_iter()
                    .map(|track| {
                        let stats = catalog.stats(&track.locator).copied().unwrap_or_default();
                        Row {
                            plays: stats.plays,
                            last_played: stats.last_played,
                            added: items
                                .and_then(|items| items.get(&track.locator))
                                .map_or(0, |item| item.added),
                            track,
                        }
                    })
                    .collect();
                let total = catalog.tracks().len();
                let _ = thread.queue(move |list| list.show(generation, rows, total));
            }),
        );
    }

    /// Shows the tracks of query `generation` unless a newer one was sent.
    fn show(mut self: Pin<&mut Self>, generation: u64, rows: Vec<Row>, total: usize) {
        if generation != self.generation {
            return;
        }
        let duration = rows.iter().map(|row| row.track.length).sum();
        let count = rows.len();
        if count == self.rows.len() {
            // Same length: update in place, so the view keeps its position.
            let first = self.rows.iter().zip(&rows).position(|(a, b)| !a.same(b));
            let last = self.rows.iter().zip(&rows).rposition(|(a, b)| !a.same(b));
            self.as_mut().rust_mut().rows = rows;
            if let (Some(first), Some(last)) = (first, last) {
                self.as_mut().rows_changed(first, last, &[]);
            }
        } else {
            self.as_mut().begin_reset_model();
            self.as_mut().rust_mut().rows = rows;
            self.as_mut().end_reset_model();
        }
        let current_row = self.row_of_current();
        self.as_mut().set_current_row(current_row);
        let covers = self.first_covers();
        self.as_mut().set_covers(covers);
        self.as_mut().set_count(clamp(count));
        self.as_mut().set_total(clamp(total));
        self.as_mut().set_duration(duration);
        self.set_ready(true);
    }

    /// Highlights the track that is current now.
    fn update_current(mut self: Pin<&mut Self>) {
        let current = hub()
            .state()
            .current
            .as_ref()
            .map(|track| track.locator.clone());
        if current == self.current {
            return;
        }
        let previous = self.current_row;
        self.as_mut().rust_mut().current = current;
        let row = self.row_of_current();
        self.as_mut().set_current_row(row);
        for row in [previous, row] {
            if let Ok(row) = usize::try_from(row) {
                self.as_mut().rows_changed(row, row, &[CURRENT]);
            }
        }
    }

    fn first_covers(&self) -> QStringList {
        let mut covers = Vec::<&std::path::Path>::new();
        for cover in self
            .rows
            .iter()
            .filter_map(|row| row.track.cover.as_deref())
        {
            if !covers.contains(&cover) {
                covers.push(cover);
                if covers.len() == 4 {
                    break;
                }
            }
        }
        let mut list = QList::<QString>::default();
        for cover in covers {
            list.append(QString::from(&*cover.to_string_lossy()));
        }
        QStringList::from(&list)
    }

    fn row_of_current(&self) -> i32 {
        self.current
            .as_ref()
            .and_then(|current| {
                self.rows
                    .iter()
                    .position(|row| row.track.locator == *current)
            })
            .map_or(-1, clamp)
    }

    /// Tells the view that `roles` of rows `first..=last` changed; all roles when empty.
    fn rows_changed(self: Pin<&mut Self>, first: usize, last: usize, roles: &[i32]) {
        let top_left = self.index(clamp(first), 0, &QModelIndex::default());
        let bottom_right = self.index(clamp(last), 0, &QModelIndex::default());
        let mut list = QList::<i32>::default();
        for &role in roles {
            list.append(role);
        }
        self.data_changed(&top_left, &bottom_right, &list);
    }

    fn row(&self, row: i32) -> Option<&Row> {
        usize::try_from(row).ok().and_then(|row| self.rows.get(row))
    }

    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        clamp(self.rows.len())
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(row) = self.row(index.row()) else {
            return QVariant::default();
        };
        let track = &row.track;
        match role {
            TITLE => QVariant::from(&QString::from(&track.title)),
            ARTIST => QVariant::from(&QString::from(&track.artist())),
            ALBUM => QVariant::from(&QString::from(track.album.as_deref().unwrap_or_default())),
            YEAR => QVariant::from(
                &track
                    .year
                    .map_or_else(QString::default, |year| QString::from(&year.to_string())),
            ),
            PLAYS => QVariant::from(&clamp(row.plays as usize)),
            LENGTH => QVariant::from(&QString::from(&format::clock(track.length))),
            COVER => QVariant::from(
                &track
                    .cover
                    .as_deref()
                    .map_or_else(QString::default, |path| {
                        QString::from(&*path.to_string_lossy())
                    }),
            ),
            CURRENT => QVariant::from(&(self.current.as_ref() == Some(&track.locator))),
            LAST_PLAYED => QVariant::from(&(row.last_played as f64)),
            ADDED => QVariant::from(&(row.added as f64)),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for (role, name) in [
            (TITLE, "title"),
            (ARTIST, "artist"),
            (ALBUM, "album"),
            (YEAR, "year"),
            (PLAYS, "plays"),
            (LENGTH, "length"),
            (COVER, "cover"),
            (CURRENT, "current"),
            (LAST_PLAYED, "lastPlayed"),
            (ADDED, "added"),
        ] {
            roles.insert(role, QByteArray::from(name));
        }
        roles
    }

    fn play(&self, row: i32) {
        let Some(row) = self.row(row) else {
            return;
        };
        self.play_from(Some(row.track.locator.clone()));
    }

    fn play_all(&self, shuffle: bool) {
        bus::emit(SetShuffle(shuffle));
        self.play_from(None);
    }

    fn locator(&self, row: i32) -> Option<Locator> {
        self.row(row).map(|row| row.track.locator.clone())
    }

    fn enqueue(&self, row: i32, next: bool) {
        if let Some(locator) = self.locator(row) {
            bus::emit(Enqueue {
                tracks: vec![locator],
                next,
            });
        }
    }

    fn add_to_playlist(&self, row: i32, id: i64) {
        if let Some(locator) = self.locator(row) {
            bus::emit(AddToPlaylist {
                id: PlaylistId(id),
                tracks: vec![locator],
            });
        }
    }

    fn add_to_new_playlist(&self, row: i32, name: &QString) {
        if let Some(locator) = self.locator(row) {
            crate::bridge::playlists::create(CreatePlaylist {
                name: name.to_string(),
                rule: None,
                sort: vec![],
                tracks: vec![locator],
            });
        }
    }

    fn remove_from_playlist(&self, row: i32) {
        if let (Some(id), Some(locator)) = (self.playlist_id(), self.locator(row)) {
            bus::emit(RemoveFromPlaylist {
                id,
                tracks: vec![locator],
            });
        }
    }

    fn about(&self, row: i32) -> QVariant {
        let mut about = QMap::<QMapPair_QString_QVariant>::default();
        if let Some(row) = self.row(row) {
            let track = &row.track;
            let artist = track.artists.first().map(String::as_str);
            for (key, value) in [
                ("album", track.album.as_deref()),
                ("albumArtist", track.album_artist.as_deref().or(artist)),
                ("artist", artist),
            ] {
                about.insert(
                    QString::from(key),
                    QVariant::from(&QString::from(value.unwrap_or_default())),
                );
            }
        }
        QVariant::from(&about)
    }

    fn playlists_with(&self, row: i32) -> QVariant {
        let mut ids = QList::<QVariant>::default();
        if let Some(locator) = self.locator(row) {
            let catalog = hub().library().read();
            for playlist in catalog.playlists() {
                if playlist.items.contains_key(&locator) {
                    ids.append(QVariant::from(&playlist.id.0));
                }
            }
        }
        QVariant::from(&ids)
    }

    /// Plays this list from `start`, telling the queue where the session comes from.
    fn play_from(&self, start: Option<Locator>) {
        let detail = self.detail.to_string();
        let context = Context {
            label: self.label.to_string(),
            detail: (!detail.is_empty()).then_some(detail).into_iter().collect(),
            page: self.origin.to_string(),
        };
        hub().update(Changed::CONTEXT, |state| state.context = Some(context));
        bus::emit(PlayFrom {
            query: self.query(),
            start,
        });
    }
}

/// Qt counts rows in `i32`.
fn clamp(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
