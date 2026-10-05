//! Scans of the library's sources: whether one runs, how far it got, and starting one.

#[cxx_qt::bridge]
pub mod qobject {
    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// A scan of some sources is running or waiting.
        #[qproperty(bool, running)]
        /// Tracks the running scan found; 0 until it listed its sources.
        #[qproperty(i32, found)]
        /// Tracks found that were read, from their files or the cache.
        #[qproperty(i32, read)]
        /// When the last complete scan of every source finished, in Unix seconds; 0 before one
        /// since launch.
        #[qproperty(f64, updated)]
        type Scan = super::ScanRust;

        /// Picks up tracks added, changed or removed in every source; unchanged ones come from
        /// the cache.
        #[qinvokable]
        fn refresh(self: &Scan);
        /// Reads the tags and cover of every track again, ignoring the cache.
        #[qinvokable]
        fn reload(self: &Scan);
    }

    impl cxx_qt::Threading for Scan {}
    impl cxx_qt::Initialize for Scan {}
}

use crate::bus;
use crate::hub::{hub, Changed};
use core::pin::Pin;
use cxx_qt::Threading;
use n_music_core::messages::ScanRequested;

#[derive(Default)]
pub struct ScanRust {
    running: bool,
    found: i32,
    read: i32,
    updated: f64,
}

impl cxx_qt::Initialize for qobject::Scan {
    fn initialize(self: Pin<&mut Self>) {
        hub().watch(
            self.qt_thread(),
            Changed::SCAN | Changed::PROGRESS,
            |scan, _| scan.load(),
        );
        self.load();
    }
}

impl qobject::Scan {
    fn load(mut self: Pin<&mut Self>) {
        let (running, found, unread, updated) = {
            let state = hub().state();
            (state.scanning, state.found, state.unread, state.updated)
        };
        let count = |value: usize| i32::try_from(value).unwrap_or(i32::MAX);
        self.as_mut().set_found(count(found));
        self.as_mut().set_read(count(found.saturating_sub(unread)));
        self.as_mut().set_updated(updated.unwrap_or(0.0));
        self.set_running(running);
    }

    fn refresh(&self) {
        bus::emit(ScanRequested {
            library: None,
            check_cache: true,
        });
    }

    fn reload(&self) {
        bus::emit(ScanRequested {
            library: None,
            check_cache: false,
        });
    }
}
