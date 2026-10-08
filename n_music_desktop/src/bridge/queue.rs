//! The play session for the queue page: what played, then what is still to play, queued items
//! among the context's where they stand. The current item is listed when asked for, first among
//! those still to play.

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
        /// Lists the items played before the current one too, above the others.
        #[qproperty(bool, show_history)]
        /// Lists the current item too, before those still to play.
        #[qproperty(bool, show_current)]
        /// Rows are sliding into their places: session updates wait until they stop, as one
        /// landing meanwhile would leave them where they stopped.
        #[qproperty(bool, moving)]
        #[qproperty(i32, history_count)]
        /// Queued items still to play.
        #[qproperty(i32, up_next_count)]
        /// Items still to play, the last rows.
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
        fn begin_move_rows(
            self: Pin<&mut QueueList>,
            source_parent: &QModelIndex,
            source_first: i32,
            source_last: i32,
            destination_parent: &QModelIndex,
            destination_child: i32,
        ) -> bool;
        #[inherit]
        fn end_move_rows(self: Pin<&mut QueueList>);
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

        /// Plays the item at `row`; the current one from its start.
        #[qinvokable]
        fn play(self: &QueueList, row: i32);
        /// Takes the queued item at `row` out of the session.
        #[qinvokable]
        fn remove(self: &QueueList, row: i32);
        /// Takes every queued item still to play out of the session.
        #[qinvokable]
        fn clear_up_next(self: &QueueList);
        /// Starts dragging the item at `row`, the current one or one still to play, to another
        /// place. Session updates wait until the drag ends, so no row moves under the pointer.
        #[qinvokable]
        fn start_drag(self: Pin<&mut QueueList>, row: i32);
        /// Moves the dragged item to `row` in this list alone: anywhere, one still to play
        /// counting as played above the current one.
        #[qinvokable]
        fn drag_to(self: Pin<&mut QueueList>, row: i32);
        /// Ends the drag: the dragged item plays where it was dropped when `keep`, or goes back
        /// where it was.
        #[qinvokable]
        fn end_drag(self: Pin<&mut QueueList>, keep: bool);
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
use n_music_core::messages::{ClearQueued, MoveCurrent, MoveUpcoming, Play, RemoveQueued, Seek};
use n_music_core::queue::ItemId;
use n_music_core::{Track, TrackInfo};
use std::sync::Arc;
use std::time::Duration;

const TITLE: i32 = 0x0100;
const ARTIST: i32 = TITLE + 1;
const LENGTH: i32 = TITLE + 2;
const COVER: i32 = TITLE + 3;
const SECTION: i32 = TITLE + 4;
const QUEUED: i32 = TITLE + 5;
const CURRENT: i32 = TITLE + 6;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    /// Played before the current item.
    History,
    /// The current item, then those still to play.
    Next,
}

impl Section {
    fn name(self) -> &'static str {
        match self {
            Section::History => "history",
            Section::Next => "next",
        }
    }
}

#[derive(Clone)]
struct Row {
    item: ItemId,
    track: Track,
    section: Section,
    /// Queued with `Enqueue` rather than taken from the context.
    queued: bool,
    current: bool,
}

impl Row {
    fn same(&self, other: &Row) -> bool {
        Arc::ptr_eq(&self.track, &other.track)
            && self.section == other.section
            && self.queued == other.queued
            && self.current == other.current
    }
}

#[derive(Default)]
struct Counts {
    history: usize,
    /// Queued items still to play.
    next: usize,
    /// Items still to play.
    left: usize,
}

/// An item being dragged to another place.
struct Drag {
    item: ItemId,
    /// Its row when picked up.
    from: usize,
    current: bool,
    /// Queued with `Enqueue`: among those played, it joins the context.
    queued: bool,
    /// The rows played before the current item when picked up, the first ones.
    played: usize,
}

pub struct QueueListRust {
    show_history: bool,
    show_current: bool,
    moving: bool,
    history_count: i32,
    up_next_count: i32,
    left_count: i32,
    context_label: QString,
    context_detail: QString,
    context_page: QString,
    rows: Vec<Row>,
    drag: Option<Drag>,
    /// The session changed while rows could not move.
    missed: bool,
    slot: u64,
    generation: u64,
}

impl Default for QueueListRust {
    fn default() -> Self {
        Self {
            show_history: false,
            show_current: false,
            moving: false,
            history_count: 0,
            up_next_count: 0,
            left_count: 0,
            context_label: QString::default(),
            context_detail: QString::default(),
            context_page: QString::from("tracks"),
            rows: vec![],
            drag: None,
            missed: false,
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
        self.as_mut()
            .on_show_current_changed(|list| list.refresh(Duration::ZERO))
            .release();
        self.as_mut()
            .on_moving_changed(|list| {
                if !list.moving && list.drag.is_none() && list.missed {
                    list.refresh(Duration::ZERO);
                }
            })
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
        let (show_history, show_current) = (self.show_history, self.show_current);
        let generation = {
            let mut list = self.as_mut().rust_mut();
            list.generation += 1;
            list.missed = false;
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
                    let current = position == Some(index);
                    let section = match position {
                        Some(position) if index < position => Section::History,
                        _ => Section::Next,
                    };
                    if current && !show_current {
                        continue;
                    }
                    if section == Section::History {
                        counts.history += 1;
                        if !show_history {
                            continue;
                        }
                    } else if !current {
                        counts.left += 1;
                        counts.next += usize::from(entry.queued);
                    }
                    let track = catalog
                        .track(&entry.locator)
                        .cloned()
                        .unwrap_or_else(|| Arc::new(TrackInfo::placeholder(entry.locator.clone())));
                    rows.push(Row {
                        item: entry.item,
                        track,
                        section,
                        queued: entry.queued,
                        current,
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
        // No row moves under the pointer or while rows slide: the drag's end or the rows'
        // stop catches up.
        if self.drag.is_some() || self.moving {
            self.as_mut().rust_mut().missed = true;
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
            QUEUED => QVariant::from(&row.queued),
            CURRENT => QVariant::from(&row.current),
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
            (QUEUED, "queued"),
            (CURRENT, "current"),
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

    fn start_drag(mut self: Pin<&mut Self>, row: i32) {
        if self.drag.is_some() {
            self.as_mut().end_drag(false);
        }
        let Some(found) = self.row(row).filter(|found| found.section == Section::Next) else {
            return;
        };
        let (item, from, current, queued) = (found.item, row as usize, found.current, found.queued);
        let played = self
            .rows
            .iter()
            .take_while(|row| row.section == Section::History)
            .count();
        self.as_mut().rust_mut().drag = Some(Drag {
            item,
            from,
            current,
            queued,
            played,
        });
    }

    fn drag_to(mut self: Pin<&mut Self>, row: i32) {
        let Some(&Drag {
            item,
            current,
            queued,
            played,
            ..
        }) = self.drag.as_ref()
        else {
            return;
        };
        let Some(from) = self.rows.iter().position(|row| row.item == item) else {
            return;
        };
        // Those still to play go among those played too, above the current item.
        let first = if current || self.rows.iter().any(|row| row.current) {
            0
        } else {
            self.rows
                .iter()
                .position(|row| row.section == Section::Next && !row.current)
                .unwrap_or(from)
        };
        let to = usize::try_from(row)
            .unwrap_or(0)
            .min(self.rows.len() - 1)
            .max(first);
        self.as_mut().move_row(from, to);
        if current {
            self.split_at_current(from, to, played);
        } else {
            self.section_dragged(to, queued);
        }
    }

    fn end_drag(mut self: Pin<&mut Self>, keep: bool) {
        let Some(drag) = self.as_mut().rust_mut().drag.take() else {
            return;
        };
        let at = self.rows.iter().position(|row| row.item == drag.item);
        match at {
            Some(at) if keep && at != drag.from => {
                let (item, before) = (drag.item, self.rows.get(at + 1).map(|row| row.item));
                if drag.current {
                    bus::emit(MoveCurrent { item, before });
                } else {
                    bus::emit(MoveUpcoming { item, before });
                }
                // Refreshes under way read the session before the move; this one waits for the
                // session to make it.
                self.refresh(MOVE_SETTLES);
            }
            Some(at) => {
                if !keep {
                    self.as_mut().move_row(at, drag.from);
                    if drag.current {
                        self.as_mut().split_at_current(at, drag.from, drag.played);
                    } else {
                        self.as_mut().section_dragged(drag.from, drag.queued);
                    }
                }
                if self.missed {
                    self.refresh(Duration::ZERO);
                }
            }
            None => self.refresh(Duration::ZERO),
        }
    }

    /// Moves the row at `from` to `to` in this list alone.
    fn move_row(mut self: Pin<&mut Self>, from: usize, to: usize) {
        if from == to || to >= self.rows.len() {
            return;
        }
        let root = QModelIndex::default();
        // Qt places the row before `destination` as the list stands before the move.
        let destination = if to > from { to + 1 } else { to };
        self.as_mut()
            .begin_move_rows(&root, clamp(from), clamp(from), &root, clamp(destination));
        {
            let mut list = self.as_mut().rust_mut();
            let row = list.rows.remove(from);
            list.rows.insert(to, row);
        }
        self.as_mut().end_move_rows();
    }

    /// Sections the rows from `from` to `to` again, the current item dragged from one to the
    /// other: above it the first `played` rows played, the others play after it.
    fn split_at_current(mut self: Pin<&mut Self>, from: usize, to: usize, played: usize) {
        let mut changed: Option<(usize, usize)> = None;
        {
            let mut list = self.as_mut().rust_mut();
            for index in from.min(to)..=from.max(to) {
                let section = if index < to.min(played) {
                    Section::History
                } else {
                    Section::Next
                };
                if list.rows[index].section != section {
                    list.rows[index].section = section;
                    changed = Some((changed.map_or(index, |(first, _)| first), index));
                }
            }
        }
        if let Some((first, last)) = changed {
            self.rows_changed(first, last, &[SECTION]);
        }
    }

    /// Sections the dragged item at `at`, one still to play, by where it stands: above the
    /// current item it counts as played, and one `queued` joins the context.
    fn section_dragged(mut self: Pin<&mut Self>, at: usize, queued: bool) {
        let section = match self.rows.iter().position(|row| row.current) {
            Some(current) if at < current => Section::History,
            _ => Section::Next,
        };
        let queued = queued && section == Section::Next;
        if self.rows[at].section != section || self.rows[at].queued != queued {
            {
                let mut list = self.as_mut().rust_mut();
                list.rows[at].section = section;
                list.rows[at].queued = queued;
            }
            self.rows_changed(at, at, &[SECTION, QUEUED]);
        }
    }

    /// Tells the view `roles` of the rows from `first` to `last` changed.
    fn rows_changed(mut self: Pin<&mut Self>, first: usize, last: usize, roles: &[i32]) {
        let root = QModelIndex::default();
        let top_left = self.index(clamp(first), 0, &root);
        let bottom_right = self.index(clamp(last), 0, &root);
        let mut changed = QList::default();
        for &role in roles {
            changed.append(role);
        }
        self.as_mut()
            .data_changed(&top_left, &bottom_right, &changed);
    }
}

/// How long the session may take to make a dropped item's move.
const MOVE_SETTLES: Duration = Duration::from_millis(250);

/// Qt counts rows in `i32`.
fn clamp(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
