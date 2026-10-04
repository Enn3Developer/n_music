//! Whether the library folders are being scanned.

#[cxx_qt::bridge]
pub mod qobject {
    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// A scan of the library folders is running.
        #[qproperty(bool, running)]
        type Scan = super::ScanRust;
    }

    impl cxx_qt::Threading for Scan {}
    impl cxx_qt::Initialize for Scan {}
}

use crate::hub::{hub, Changed};
use core::pin::Pin;
use cxx_qt::Threading;

#[derive(Default)]
pub struct ScanRust {
    running: bool,
}

impl cxx_qt::Initialize for qobject::Scan {
    fn initialize(self: Pin<&mut Self>) {
        hub().watch(self.qt_thread(), Changed::SCAN, |scan, _| scan.load());
        self.load();
    }
}

impl qobject::Scan {
    fn load(self: Pin<&mut Self>) {
        let running = hub().state().scanning;
        self.set_running(running);
    }
}
