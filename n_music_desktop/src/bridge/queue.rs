//! The play session for the queue page: what played, what is queued next, and the rest of
//! what the session plays from. The current item is not listed.

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
        /// Lists the items played before the current one too.
        #[qproperty(bool, show_history)]
        #[qproperty(i32, history_count)]
        #[qproperty(i32, up_next_count)]
        /// Items of the context after the current one.
        #[qproperty(i32, left_count)]
        /// What the session plays from, like `Tracks`; empty for the whole library.
        #[qproperty(QString, context_label)]
        /// How that was narrowed down, like a search, joined with ` · `.
        #[qproperty(QString, context_detail)]
        /// The page that shows what the session plays from.
        #[qproperty(QString, context_page)]
        type QueueList = super::QueueListRust;

        #[inherit]
        fn begin_insert_rows(
            self: Pin<&mut QueueList>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        fn end_insert_rows(self: Pin<&mut QueueList>);
        #[inherit]
        fn begin_remove_rows(
            self: Pin<&mut QueueList>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        fn end_remove_rows(self: Pin<&mut QueueList>);
        #[inherit]
        fn index(self: &QueueList, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;
        #[inherit]
        #[qsignal]
        fn data_changed(
            self: Pin<&mut QueueList>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QList_i32,
        );

        #[cxx_override]
        fn row_count(self: &QueueList, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &QueueList, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        fn role_names(self: &QueueList) -> QHash_i32_QByteArray;

        /// Plays the item at `row`.
        #[qinvokable]
        fn play(self: &QueueList, row: i32);
        /// Takes the item at `row` out of up next.
        #[qinvokable]
        fn remove(self: &QueueList, row: i32);
        /// Empties up next.
        #[qinvokable]
        fn clear_up_next(self: &QueueList);
    }

    impl cxx_qt::Threading for QueueList {}
    impl cxx_qt::Initialize for QueueList {}
}

use crate::hub::{hub, Changed};
use crate::{bus, format, worker};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QList, QModelIndex, QString, QVariant,
};
use n_music_core::messages::{ClearQueued, Play, RemoveQueued, Seek};
use n_music_core::queue::ItemId;
use n_music_core::{Track, TrackInfo};
use std::sync::Arc;
use std::time::Duration;

const TITLE: i32 = 0x0100;
const ARTIST: i32 = TITLE + 1;
const LENGTH: i32 = TITLE + 2;
const COVER: i32 = TITLE + 3;
const SECTION: i32 = TITLE + 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    /// Played before the current item.
    History,
    /// Queued to play next.
    Next,
    /// The rest of the context.
    Context,
}

impl Section {
    fn name(self) -> &'static str {
        match self {
            Section::History => "history",
            Section::Next => "next",
            Section::Context => "context",
        }
    }
}

#[derive(Clone)]
struct Row {
    item: ItemId,
    track: Track,
    section: Section,
}

impl Row {
    fn same(&self, other: &Row) -> bool {
        Arc::ptr_eq(&self.track, &other.track) && self.section == other.section
    }
}

#[derive(Default)]
struct Counts {
    history: usize,
    next: usize,
    left: usize,
}

pub struct QueueListRust {
    show_history: bool,
    history_count: i32,
    up_next_count: i32,
    left_count: i32,
    context_label: QString,
    context_detail: QString,
    context_page: QString,
    rows: Vec<Row>,
    slot: u64,
    generation: u64,
}

impl Default for QueueListRust {
    fn default() -> Self {
        Self {
            show_history: false,
            history_count: 0,
            up_next_count: 0,
            left_count: 0,
            context_label: QString::default(),
            context_detail: QString::default(),
            context_page: QString::from("tracks"),
            rows: vec![],
            slot: worker::slot(),
            generation: 0,
        }
    }
}

impl cxx_qt::Initialize for qobject::QueueList {
    fn initialize(mut self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::QUEUE
                | Changed::CURRENT
                | Changed::TRACKS
                | Changed::METADATA
                | Changed::CONTEXT,
            Self::changed,
        );
        self.as_mut()
            .on_show_history_changed(|list| list.refresh(Duration::ZERO))
            .release();
        self.as_mut().refresh(Duration::ZERO);
        self.load_context();
    }
}

impl qobject::QueueList {
    fn changed(mut self: Pin<&mut Self>, changed: Changed) {
        if changed.intersects(Changed::QUEUE | Changed::CURRENT | Changed::TRACKS) {
            self.as_mut().refresh(Duration::ZERO);
        } else if changed.contains(Changed::METADATA) {
            self.as_mut().refresh(worker::STREAMING);
        }
        if changed.contains(Changed::CONTEXT) {
            self.load_context();
        }
    }

    fn load_context(mut self: Pin<&mut Self>) {
        let context = hub().state().context.clone();
        let (label, detail, page) = match context {
            Some(context) => (context.label, context.detail.join(" · "), context.page),
            None => (String::new(), String::new(), String::from("tracks")),
        };
        self.as_mut().set_context_label(QString::from(&label));
        self.as_mut().set_context_detail(QString::from(&detail));
        self.set_context_page(QString::from(&page));
    }

    /// Splits the session into sections again, after `delay`.
    fn refresh(mut self: Pin<&mut Self>, delay: Duration) {
        let show_history = self.show_history;
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
                let (entries, current) = {
                    let state = hub().state();
                    (state.queue.clone(), state.current_item)
                };
                let position = current
                    .and_then(|current| entries.iter().position(|entry| entry.item == current));
                let mut counts = Counts::default();
                let mut rows = Vec::with_capacity(entries.len());
                for (index, entry) in entries.iter().enumerate() {
                    let section = match position {
                        Some(position) if index == position => continue,
                        Some(position) if index < position => Section::History,
                        _ if entry.queued => Section::Next,
                        _ => Section::Context,
                    };
                    match section {
                        Section::History => counts.history += 1,
                        Section::Next => counts.next += 1,
                        Section::Context => counts.left += 1,
                    }
                    if section == Section::History && !show_history {
                        continue;
                    }
                    let track = catalog
                        .track(&entry.locator)
                        .cloned()
                        .unwrap_or_else(|| Arc::new(TrackInfo::placeholder(entry.locator.clone())));
                    rows.push(Row {
                        item: entry.item,
                        track,
                        section,
                    });
                }
                let _ = thread.queue(move |list| list.show(generation, rows, counts));
            }),
        );
    }

    /// Shows `rows` as removals and insertions between the rows both lists keep at their
    /// ends, so the view keeps its place when the session moves on.
    fn show(mut self: Pin<&mut Self>, generation: u64, rows: Vec<Row>, counts: Counts) {
        if generation != self.generation {
            return;
        }
        let (old_len, new_len) = (self.rows.len(), rows.len());
        let prefix = self
            .rows
            .iter()
            .zip(&rows)
            .take_while(|(a, b)| a.item == b.item)
            .count();
        let suffix = self
            .rows
            .iter()
            .rev()
            .zip(rows.iter().rev())
            .take(old_len.min(new_len) - prefix)
            .take_while(|(a, b)| a.item == b.item)
            .count();
        let changed: Vec<usize> = (0..prefix)
            .filter(|&row| !self.rows[row].same(&rows[row]))
            .chain((1..=suffix).filter_map(|back| {
                let (old, new) = (old_len - back, new_len - back);
                (!self.rows[old].same(&rows[new])).then_some(new)
            }))
            .collect();

        let root = QModelIndex::default();
        if old_len - suffix > prefix {
            self.as_mut()
                .begin_remove_rows(&root, clamp(prefix), clamp(old_len - suffix - 1));
            self.as_mut()
                .rust_mut()
                .rows
                .drain(prefix..old_len - suffix);
            self.as_mut().end_remove_rows();
        }
        if new_len - suffix > prefix {
            self.as_mut()
                .begin_insert_rows(&root, clamp(prefix), clamp(new_len - suffix - 1));
            let inserted = rows[prefix..new_len - suffix].to_vec();
            self.as_mut()
                .rust_mut()
                .rows
                .splice(prefix..prefix, inserted);
            self.as_mut().end_insert_rows();
        }
        self.as_mut().rust_mut().rows = rows;
        if let (Some(&first), Some(&last)) = (changed.iter().min(), changed.iter().max()) {
            let top_left = self.index(clamp(first), 0, &root);
            let bottom_right = self.index(clamp(last), 0, &root);
            self.as_mut()
                .data_changed(&top_left, &bottom_right, &QList::default());
        }
        self.as_mut().set_history_count(clamp(counts.history));
        self.as_mut().set_up_next_count(clamp(counts.next));
        self.set_left_count(clamp(counts.left));
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
            LENGTH => QVariant::from(&QString::from(&format::clock(track.length))),
            COVER => QVariant::from(
                &track
                    .cover
                    .as_deref()
                    .map_or_else(QString::default, |path| {
                        QString::from(&*path.to_string_lossy())
                    }),
            ),
            SECTION => QVariant::from(&QString::from(row.section.name())),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for (role, name) in [
            (TITLE, "title"),
            (ARTIST, "artist"),
            (LENGTH, "length"),
            (COVER, "cover"),
            (SECTION, "section"),
        ] {
            roles.insert(role, QByteArray::from(name));
        }
        roles
    }

    fn play(&self, row: i32) {
        if let Some(row) = self.row(row) {
            // A jump keeps playback paused when it was; picking a track means playing it.
            bus::emit(Seek::ToItem {
                item: row.item,
                position: 0.0,
            });
            bus::emit(Play);
        }
    }

    fn remove(&self, row: i32) {
        if let Some(row) = self.row(row) {
            bus::emit(RemoveQueued(row.item));
        }
    }

    fn clear_up_next(&self) {
        bus::emit(ClearQueued);
    }
}

/// Qt counts rows in `i32`.
fn clamp(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
