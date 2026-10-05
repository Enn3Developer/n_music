//! New versions for copies installed with the n_music installer: finding, downloading and
//! restarting into them.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// `unsupported` (not installed with the installer), `idle`, `checking`, `current`,
        /// `available`, `downloading`, `ready` or `failed`.
        #[qproperty(QString, status)]
        /// The version found, while one is available, downloading or ready.
        #[qproperty(QString, latest)]
        /// How much of it downloaded, in percent.
        #[qproperty(i32, progress)]
        /// When the last check finished, in Unix seconds; 0 before one since launch.
        #[qproperty(f64, checked)]
        type Updates = super::UpdatesRust;

        /// Looks for a newer version.
        #[qinvokable]
        fn check(self: Pin<&mut Updates>);
        /// Downloads the version found.
        #[qinvokable]
        fn download(self: Pin<&mut Updates>);
        /// Has the downloaded version installed once the app quits, then started; false when
        /// that failed. The app must quit next.
        #[qinvokable]
        fn apply(self: Pin<&mut Updates>) -> bool;
    }

    impl cxx_qt::Threading for Updates {}
    impl cxx_qt::Initialize for Updates {}
}

use crate::settings;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::sync::mpsc;
use std::time::SystemTime;
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager};

const REPOSITORY: &str = "https://github.com/Enn3Developer/n_music";

#[derive(Default)]
pub struct UpdatesRust {
    status: QString,
    latest: QString,
    progress: i32,
    checked: f64,
    /// The version the last check found.
    found: Option<UpdateInfo>,
}

/// The updater of this copy; `None` unless the installer installed it.
fn manager() -> Option<UpdateManager> {
    // Pre-releases too: every release so far is one.
    UpdateManager::new(GithubSource::new(REPOSITORY, None, true), None, None)
        .inspect_err(|error| log::debug!("No updates for this copy: {error}"))
        .ok()
}

impl cxx_qt::Initialize for qobject::Updates {
    fn initialize(mut self: Pin<&mut Self>) {
        if manager().is_none() {
            self.set_status(QString::from("unsupported"));
            return;
        }
        self.as_mut().set_status(QString::from("idle"));
        // Once a launch, when the settings first show, unless turned off.
        if settings::ui().get().check_updates {
            self.check();
        }
    }
}

impl qobject::Updates {
    fn is(&self, statuses: &[&str]) -> bool {
        statuses.contains(&self.status.to_string().as_str())
    }

    fn check(mut self: Pin<&mut Self>) {
        if self.is(&["unsupported", "checking", "downloading", "ready"]) {
            return;
        }
        self.as_mut().set_status(QString::from("checking"));
        let thread = self.qt_thread();
        let spawned = std::thread::Builder::new()
            .name(String::from("update check"))
            .spawn(move || {
                let found = match manager().map(|manager| manager.check_for_updates()) {
                    Some(Ok(UpdateCheck::UpdateAvailable(info))) => Ok(Some(*info)),
                    Some(Ok(UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty)) => {
                        Ok(None)
                    }
                    Some(Err(error)) => Err(error.to_string()),
                    None => Err(String::from("no updater")),
                };
                let _ = thread.queue(move |updates| updates.show_check(found));
            });
        if let Err(error) = spawned {
            log::error!("Could not check for updates: {error}");
            self.set_status(QString::from("failed"));
        }
    }

    fn show_check(mut self: Pin<&mut Self>, found: Result<Option<UpdateInfo>, String>) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0.0, |since| since.as_secs_f64());
        self.as_mut().set_checked(now);
        let status = match found {
            Ok(Some(info)) => {
                self.as_mut()
                    .set_latest(QString::from(&info.TargetFullRelease.Version));
                self.as_mut().rust_mut().found = Some(info);
                "available"
            }
            Ok(None) => "current",
            Err(error) => {
                log::warn!("Could not check for updates: {error}");
                "failed"
            }
        };
        self.set_status(QString::from(status));
    }

    fn download(mut self: Pin<&mut Self>) {
        let Some(info) = self.found.clone().filter(|_| self.is(&["available"])) else {
            return;
        };
        self.as_mut().set_progress(0);
        self.as_mut().set_status(QString::from("downloading"));
        let thread = self.qt_thread();
        let spawned = std::thread::Builder::new()
            .name(String::from("update download"))
            .spawn(move || {
                let (progress, percents) = mpsc::channel::<i16>();
                let reporter = thread.clone();
                // Reports each percent until the download drops its end of the channel.
                let reporting = std::thread::spawn(move || {
                    for percent in percents {
                        let _ =
                            reporter.queue(move |updates| updates.set_progress(i32::from(percent)));
                    }
                });
                let done = match manager() {
                    Some(manager) => manager
                        .download_updates(&info, Some(progress))
                        .map_err(|error| error.to_string()),
                    None => Err(String::from("no updater")),
                };
                let _ = reporting.join();
                let _ = thread.queue(move |updates| {
                    let status = match done {
                        Ok(()) => "ready",
                        Err(error) => {
                            log::warn!("Could not download the update: {error}");
                            "failed"
                        }
                    };
                    updates.set_status(QString::from(status));
                });
            });
        if let Err(error) = spawned {
            log::error!("Could not download the update: {error}");
            self.set_status(QString::from("failed"));
        }
    }

    fn apply(self: Pin<&mut Self>) -> bool {
        let Some(info) = self.found.clone().filter(|_| self.is(&["ready"])) else {
            return false;
        };
        let applied = manager().map(|manager| {
            manager.wait_exit_then_apply_updates(&info, false, true, Vec::<String>::new())
        });
        match applied {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                log::error!("Could not apply the update: {error}");
                self.set_status(QString::from("failed"));
                false
            }
            None => false,
        }
    }
}
