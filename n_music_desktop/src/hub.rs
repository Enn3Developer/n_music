//! What the core reported, kept for the views, and how views learn that it changed.
//!
//! The bus thread writes the [`State`] and wakes the views that care; each view then reads
//! what it shows on the Qt thread. Wakes are coalesced: a burst of changes reaches a view as
//! one call with every change of the burst.

use bitflags::bitflags;
use core::pin::Pin;
use cxx_qt::{CxxQtThread, Threading};
use n_music_core::library::catalog::Library;
use n_music_core::queue::LoopStatus;
use n_music_core::{Track, TrackTime};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

bitflags! {
    /// What a view may have to refresh.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Changed: u32 {
        /// The library's tracks were listed again.
        const TRACKS = 1 << 0;
        /// Metadata of some tracks loaded; they come in bursts during a scan.
        const METADATA = 1 << 1;
        /// Play counts.
        const STATS = 1 << 2;
        /// The current item or its track.
        const CURRENT = 1 << 3;
        const PLAYBACK = 1 << 4;
        /// A scan started or finished.
        const SCAN = 1 << 5;
        /// Where playback is in the current track.
        const POSITION = 1 << 6;
        const VOLUME = 1 << 7;
        /// Shuffle or repeat.
        const MODES = 1 << 8;
    }
}

#[derive(Default)]
pub struct State {
    /// The track of the current item.
    pub current: Option<Track>,
    pub playing: bool,
    pub scanning: bool,
    pub time: TrackTime,
    /// The last tracked seek that applied.
    pub seek: u64,
    pub volume: f64,
    pub shuffle: bool,
    pub loop_status: LoopStatus,
}

pub struct Hub {
    library: Library,
    state: Mutex<State>,
    views: Mutex<Vec<Box<dyn View>>>,
}

static HUB: OnceLock<Hub> = OnceLock::new();

/// Keeps the core's `library` for the views; the bus starts it before the interface loads.
pub fn init(library: Library) {
    let _ = HUB.set(Hub {
        library,
        // The engine starts with a scan.
        state: Mutex::new(State {
            scanning: true,
            ..State::default()
        }),
        views: Mutex::default(),
    });
}

pub fn hub() -> &'static Hub {
    HUB.get().expect("the bus starts before the interface")
}

impl Hub {
    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }

    /// Calls `apply` on the Qt thread of `thread`'s object when something in `interest`
    /// changed, until the object is destroyed.
    pub fn watch<T>(
        &self,
        thread: CxxQtThread<T>,
        interest: Changed,
        apply: fn(Pin<&mut T>, Changed),
    ) where
        T: Threading + 'static,
    {
        self.views.lock().unwrap().push(Box::new(Watch {
            thread,
            interest,
            pending: Arc::default(),
            apply,
        }));
    }

    /// Applies `change` to the state, then wakes the views that care about `changed`.
    pub fn update(&self, changed: Changed, change: impl FnOnce(&mut State)) {
        change(&mut self.state());
        self.notify(changed);
    }

    /// Wakes the views that care about `changed`.
    pub fn notify(&self, changed: Changed) {
        self.views.lock().unwrap().retain(|view| view.wake(changed));
    }
}

trait View: Send {
    /// Schedules a refresh for `changed`; false once the view is gone.
    fn wake(&self, changed: Changed) -> bool;
}

struct Watch<T: Threading> {
    thread: CxxQtThread<T>,
    interest: Changed,
    /// Changes not delivered yet; a wake is queued while it is not empty.
    pending: Arc<AtomicU32>,
    apply: fn(Pin<&mut T>, Changed),
}

impl<T: Threading + 'static> View for Watch<T> {
    fn wake(&self, changed: Changed) -> bool {
        let changed = changed & self.interest;
        if changed.is_empty() || self.pending.fetch_or(changed.bits(), Ordering::AcqRel) != 0 {
            return true;
        }
        let pending = self.pending.clone();
        let apply = self.apply;
        self.thread
            .queue(move |view| {
                let changed = Changed::from_bits_retain(pending.swap(0, Ordering::AcqRel));
                apply(view, changed);
            })
            .is_ok()
    }
}
