use n_event_bus::{EventWriter, Job, JobToken};
use n_music_core::messages::SetLibraryRoots;
use n_music_core::source::Locator;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn open_link_desktop(link: String) {
    if let Err(error) = open::that(&link) {
        log::error!("Could not open link {link:?}: {error}");
    }
}

fn internal_dir_desktop() -> PathBuf {
    let base_dirs =
        directories::BaseDirs::new().expect("No desktop application data directory is available");
    let local_data_dir = base_dirs.data_local_dir();
    let app_dir = local_data_dir.join("n_music");
    if let Err(error) = std::fs::create_dir_all(&app_dir) {
        log::error!(
            "Could not create application data directory {}: {error}",
            app_dir.display()
        );
    }
    app_dir
}

fn cache_dir_desktop() -> PathBuf {
    let base_dirs =
        directories::BaseDirs::new().expect("No desktop application data directory is available");
    let cache_dir = base_dirs.cache_dir().join("n_music");
    if let Err(error) = std::fs::create_dir_all(&cache_dir) {
        log::error!(
            "Could not create cache directory {}: {error}",
            cache_dir.display()
        );
    }
    cache_dir
}

pub type NativePlatform = DesktopPlatform;

pub struct DesktopPlatform;

impl DesktopPlatform {
    pub fn new() -> Self {
        Self
    }

    pub fn open_link(&self, link: String) {
        open_link_desktop(link)
    }

    /// Where the app keeps its data.
    pub fn internal_dir(&self) -> PathBuf {
        internal_dir_desktop()
    }

    /// Where the app keeps what can be rebuilt (covers).
    pub fn cache_dir(&self) -> PathBuf {
        cache_dir_desktop()
    }

    /// Asks for a music folder; `None` when the dialog is cancelled or one is open already.
    fn pick_music_folder(&self) -> Option<Locator> {
        if PICKING.swap(true, Ordering::SeqCst) {
            return None;
        }
        let folder = rfd::FileDialog::new().pick_folder();
        PICKING.store(false, Ordering::SeqCst);
        folder.map(|folder| Locator::Local(folder.to_string_lossy().into_owned()))
    }
}

/// A folder dialog is open: a second request is ignored.
static PICKING: AtomicBool = AtomicBool::new(false);

/// Asks for a music folder and makes it the library.
pub struct PickFolderJob(pub Arc<NativePlatform>);
impl Job for PickFolderJob {
    fn run(self, _: u64, writer: EventWriter, _: Option<JobToken>) {
        if let Some(root) = self.0.pick_music_folder() {
            // Picking a folder replaces the libraries until there is UI to manage several.
            writer.emit(SetLibraryRoots(vec![root]));
        }
    }
}

pub struct OpenLinkJob(pub Arc<NativePlatform>, pub String);
impl Job for OpenLinkJob {
    fn run(self, _: u64, _: EventWriter, _: Option<JobToken>) {
        self.0.open_link(self.1);
    }
}
