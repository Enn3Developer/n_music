//! The playlists: the list of them, one's details, and changing them.

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
        /// The playlists in the library's order, as `{ id, name, smart }`.
        #[qproperty(QVariant, items)]
        type Playlists = super::PlaylistsRust;

        /// A playlist this interface asked for was created.
        #[qsignal]
        fn created(self: Pin<&mut Playlists>, id: i64);
        /// The library refused a playlist change, saying why.
        #[qsignal]
        fn rejected(self: Pin<&mut Playlists>, message: &QString);

        /// Creates a playlist of tracks added to it.
        #[qinvokable]
        fn create(self: &Playlists, name: &QString);
        /// Creates a smart playlist of the tracks matching `filter` (see `query::parse_filter`;
        /// every track without rules), in the order of `sort`.
        #[qinvokable]
        fn create_smart(self: &Playlists, name: &QString, filter: &QString, sort: &QString);
        #[qinvokable]
        fn rename(self: &Playlists, id: i64, name: &QString);
        #[qinvokable]
        fn remove(self: &Playlists, id: i64);
        /// Gives a smart playlist the rules of `filter`.
        #[qinvokable]
        fn set_rule(self: &Playlists, id: i64, filter: &QString);
        /// Saves the order a playlist shows in.
        #[qinvokable]
        fn set_sort(self: &Playlists, id: i64, sort: &QString);
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        /// The playlist to describe, by id.
        #[qproperty(i64, playlist_id)]
        /// The library has it.
        #[qproperty(bool, exists)]
        #[qproperty(QString, name)]
        #[qproperty(bool, smart)]
        /// A smart playlist's rules as the filter editor writes them; empty when it cannot show
        /// them.
        #[qproperty(QString, rule)]
        /// The order it shows in, see `query::parse_sort`; empty when never chosen.
        #[qproperty(QString, sort)]
        /// When it last changed, in Unix seconds.
        #[qproperty(f64, modified)]
        type Playlist = super::PlaylistRust;
    }

    impl cxx_qt::Threading for Playlists {}
    impl cxx_qt::Initialize for Playlists {}
    impl cxx_qt::Threading for Playlist {}
    impl cxx_qt::Initialize for Playlist {}
}

use crate::hub::{hub, Changed};
use crate::{bus, query};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QVariant};
use n_music_core::library::query::{Filter, PlaylistId};
use n_music_core::messages::{
    CreatePlaylist, DeletePlaylist, RenamePlaylist, SetPlaylistRule, SetPlaylistSort,
};
use std::collections::HashSet;

pub struct PlaylistsRust {
    items: QVariant,
    /// The playlists listed so far, to tell the new ones.
    known: HashSet<PlaylistId>,
}

impl Default for PlaylistsRust {
    fn default() -> Self {
        Self {
            // A list from the start, so QML can go through it before the playlists arrive.
            items: QVariant::from(&QList::<QVariant>::default()),
            known: HashSet::new(),
        }
    }
}

/// Asks the library for a playlist, so its arrival is reported as `created`.
pub fn create(playlist: CreatePlaylist) {
    hub().state().creating += 1;
    bus::emit(playlist);
}

impl cxx_qt::Initialize for qobject::Playlists {
    fn initialize(self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::PLAYLISTS | Changed::REJECTED,
            Self::changed,
        );
        let _ = self
            .qt_thread()
            .queue(|playlists| playlists.changed(Changed::PLAYLISTS));
    }
}

impl qobject::Playlists {
    fn changed(mut self: Pin<&mut Self>, changed: Changed) {
        if changed.contains(Changed::REJECTED) {
            let message = hub().state().rejected.clone();
            self.as_mut().rejected(&QString::from(&message));
        }
        if !changed.contains(Changed::PLAYLISTS) {
            return;
        }
        let playlists = hub().state().playlists.clone();
        let mut items = QList::<QVariant>::default();
        for playlist in playlists.iter() {
            let mut item = QMap::<QMapPair_QString_QVariant>::default();
            item.insert(QString::from("id"), QVariant::from(&playlist.id.0));
            item.insert(
                QString::from("name"),
                QVariant::from(&QString::from(&playlist.name)),
            );
            item.insert(QString::from("smart"), QVariant::from(&playlist.smart));
            items.append(QVariant::from(&item));
        }
        self.as_mut().set_items(QVariant::from(&items));

        let ids: HashSet<PlaylistId> = playlists.iter().map(|playlist| playlist.id).collect();
        let new: Vec<PlaylistId> = playlists
            .iter()
            .map(|playlist| playlist.id)
            .filter(|id| !self.known.contains(id))
            .collect();
        self.as_mut().rust_mut().known = ids;
        for id in new {
            let asked = {
                let mut state = hub().state();
                let asked = state.creating > 0;
                state.creating = state.creating.saturating_sub(1);
                asked
            };
            if !asked {
                break;
            }
            self.as_mut().created(id.0);
        }
    }

    fn create(&self, name: &QString) {
        create(CreatePlaylist {
            name: name.to_string(),
            rule: None,
            sort: vec![],
            tracks: vec![],
        });
    }

    fn create_smart(&self, name: &QString, filter: &QString, sort: &QString) {
        create(CreatePlaylist {
            name: name.to_string(),
            rule: Some(rule(filter)),
            sort: query::parse_sort(&sort.to_string(), None),
            tracks: vec![],
        });
    }

    fn rename(&self, id: i64, name: &QString) {
        bus::emit(RenamePlaylist {
            id: PlaylistId(id),
            name: name.to_string(),
        });
    }

    fn remove(&self, id: i64) {
        bus::emit(DeletePlaylist(PlaylistId(id)));
    }

    fn set_rule(&self, id: i64, filter: &QString) {
        bus::emit(SetPlaylistRule {
            id: PlaylistId(id),
            rule: Some(rule(filter)),
        });
    }

    fn set_sort(&self, id: i64, sort: &QString) {
        let id = PlaylistId(id);
        bus::emit(SetPlaylistSort {
            id,
            sort: query::parse_sort(&sort.to_string(), Some(id)),
        });
    }
}

/// A smart playlist's rule from the filter editor's; without rules it holds every track.
fn rule(filter: &QString) -> Filter {
    query::parse_filter(&filter.to_string()).unwrap_or(Filter::All(vec![]))
}

#[derive(Default)]
pub struct PlaylistRust {
    playlist_id: i64,
    exists: bool,
    name: QString,
    smart: bool,
    rule: QString,
    sort: QString,
    modified: f64,
}

impl cxx_qt::Initialize for qobject::Playlist {
    fn initialize(mut self: Pin<&mut Self>) {
        hub().watch(self.qt_thread(), Changed::PLAYLISTS, |playlist, _| {
            playlist.load()
        });
        self.as_mut()
            .on_playlist_id_changed(|playlist| playlist.load())
            .release();
    }
}

impl qobject::Playlist {
    /// Reads the playlist from the library again.
    fn load(mut self: Pin<&mut Self>) {
        let id = PlaylistId(self.playlist_id);
        let stored = hub().library().read().playlist(id).map(|playlist| {
            (
                playlist.name.clone(),
                playlist.rule.is_some(),
                playlist.rule.as_ref().and_then(query::filter_spec),
                query::sort_string(&playlist.sort),
                playlist.modified,
            )
        });
        let Some((name, smart, rule, sort, modified)) = stored else {
            self.as_mut().set_exists(false);
            return;
        };
        self.as_mut().set_name(QString::from(&name));
        self.as_mut().set_smart(smart);
        self.as_mut()
            .set_rule(QString::from(&rule.unwrap_or_default()));
        self.as_mut().set_sort(QString::from(&sort));
        self.as_mut().set_modified(modified as f64);
        self.set_exists(true);
    }
}
