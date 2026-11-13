//! What the desktop app's bridge offers its pages beyond the queries Android uses: every
//! column of a track, list headers, the sources as the Sources page shows them and their
//! actions, where scanning is, output devices to choose from, and the interface's preferences.

use crate::library::{self, ListSummary, SourceInfo, ValueField};
use crate::rows::TrackEntry;
use crate::state::{OutputChoice, ScanStatus, StreamCacheState, TelegramSignInState};
use crate::ui::{self, DesktopUi};
use crate::Core;
use n_music_core::library::query::{PlaylistId, Query};
use n_music_core::messages::{RenameLibrary, SetLibraryRoots, SetOutputDevice};
use n_music_core::settings::{OutputDevice, SettingsStorage};
use n_music_core::source::telegram::TelegramStatus;
use n_music_core::source::Locator;
use serde_json::Value;
use std::sync::atomic::Ordering;

#[uniffi::export]
impl Core {
    /// The tracks `query` selects with every column the track table shows: `limit` of them from
    /// `offset`. `playlist` is the playlist listed, for when its tracks were added.
    pub fn track_entries(
        &self,
        query: Query,
        playlist: Option<PlaylistId>,
        offset: u32,
        limit: u32,
    ) -> Vec<TrackEntry> {
        let Some(library) = self.library.get() else {
            return vec![];
        };
        let catalog = library.read();
        catalog
            .select(&query)
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|track| TrackEntry::new(track, &catalog, playlist))
            .collect()
    }

    /// The track at `locator` with every column, when the library has it: for the player, its
    /// format and ReplayGain.
    pub fn track_entry(
        &self,
        locator: Locator,
        playlist: Option<PlaylistId>,
    ) -> Option<TrackEntry> {
        let catalog = self.library.get()?.read();
        catalog
            .track(&locator)
            .map(|track| TrackEntry::new(track, &catalog, playlist))
    }

    /// How many tracks `query` selects of how many the library has, how long they play, and
    /// the first covers among them: a track list's header.
    pub fn list_summary(&self, query: Query) -> ListSummary {
        let Some(library) = self.library.get() else {
            return library::list_summary(&[], 0);
        };
        let catalog = library.read();
        library::list_summary(&catalog.select(&query), catalog.tracks().len())
    }

    /// The row of `locator` among the tracks `query` selects, like the current track's;
    /// `None` when it is not listed.
    pub fn index_of(&self, query: Query, locator: Locator) -> Option<u32> {
        let catalog = self.library.get()?.read();
        let index = catalog
            .select(&query)
            .iter()
            .position(|track| track.locator == locator)?;
        u32::try_from(index).ok()
    }

    /// The playlists `locator` was added to: a track menu checks them.
    pub fn playlists_with(&self, locator: Locator) -> Vec<PlaylistId> {
        let Some(library) = self.library.get() else {
            return vec![];
        };
        library::playlists_with(&library.read(), &locator)
    }

    /// The library's values of `field`, each once whatever its case, sorted: suggestions for
    /// a rule's value.
    pub fn field_values(&self, field: ValueField) -> Vec<String> {
        let Some(library) = self.library.get() else {
            return vec![];
        };
        library::field_values(&library.read(), field)
    }

    /// The library's sources as it last reported them, in the order added; `None` before it
    /// did, as `LibraryRootsChanged` comes at startup.
    pub fn library_roots(&self) -> Option<Vec<Locator>> {
        self.state.lock().roots.clone()
    }

    /// The library's sources, in the order added, with what each holds and its state.
    pub fn source_list(&self) -> Vec<SourceInfo> {
        let (roots, updating, signed_in) = {
            let state = self.state.lock();
            (
                state.roots.clone().unwrap_or_default(),
                state.updating.clone(),
                matches!(
                    state.telegram.as_ref().map(|telegram| &telegram.status),
                    Some(TelegramStatus::SignedIn { .. })
                ),
            )
        };
        let Some(library) = self.library.get() else {
            return vec![];
        };
        let catalog = library.read();
        roots
            .iter()
            .map(|root| library::source_info(&catalog, root, updating.contains(root), signed_in))
            .collect()
    }

    /// What `root` is called: the name it was given, else its default name.
    pub fn source_name(&self, root: Locator) -> String {
        self.library
            .get()
            .and_then(|library| library.read().library_name(&root).map(String::from))
            .unwrap_or_else(|| library::default_name(&root))
    }

    /// Adds `root` to the library's sources, called `name` unless that is blank, and scans it.
    /// False when it is one already, or the sources were not reported yet.
    pub fn add_source(&self, root: Locator, name: String) -> bool {
        let roots = {
            let mut state = self.state.lock();
            let Some(roots) = state.roots.as_mut() else {
                return false;
            };
            if roots.contains(&root) {
                return false;
            }
            roots.push(root.clone());
            roots.clone()
        };
        self.writer.emit(SetLibraryRoots(roots));
        // Named once it is a library.
        if let Some(name) = named(&name) {
            self.writer.emit(RenameLibrary {
                library: root,
                name: Some(name),
            });
        }
        true
    }

    /// Takes `root` and its tracks out of the library. False when it is not one of its
    /// sources.
    pub fn remove_source(&self, root: Locator) -> bool {
        let roots = {
            let mut state = self.state.lock();
            let Some(roots) = state.roots.as_mut() else {
                return false;
            };
            let Some(index) = roots.iter().position(|known| *known == root) else {
                return false;
            };
            roots.remove(index);
            roots.clone()
        };
        self.writer.emit(SetLibraryRoots(roots));
        true
    }

    /// Calls the source `root` `name`, or by its default name when that is blank.
    pub fn rename_source(&self, root: Locator, name: String) {
        let known = self
            .state
            .lock()
            .roots
            .as_ref()
            .is_some_and(|roots| roots.contains(&root));
        if known {
            self.writer.emit(RenameLibrary {
                library: root,
                name: named(&name),
            });
        }
    }

    /// Makes `roots` the library's sources and scans them, ending the first run. Each is called
    /// the name at the same place in `names`, unless that is blank; duplicates are dropped.
    pub fn set_sources(&self, roots: Vec<Locator>, names: Vec<String>) {
        let mut chosen: Vec<Locator> = vec![];
        let mut renames = vec![];
        for (index, root) in roots.into_iter().enumerate() {
            if chosen.contains(&root) {
                continue;
            }
            renames.push(RenameLibrary {
                library: root.clone(),
                name: names.get(index).and_then(|name| named(name)),
            });
            chosen.push(root);
        }
        self.state.lock().roots = Some(chosen.clone());
        self.writer.emit(SetLibraryRoots(chosen));
        for rename in renames {
            self.writer.emit(rename);
        }
        self.first_run.store(false, Ordering::Release);
    }

    /// No sources were ever chosen when the core started, and `set_sources` was not called
    /// since: the app asks for them before anything else.
    pub fn first_run(&self) -> bool {
        self.first_run.load(Ordering::Acquire)
    }

    /// Where scanning is. The core starts with a scan.
    pub fn scan_status(&self) -> ScanStatus {
        self.state.lock().scan_status()
    }

    /// The output device chosen to play on; `None` plays on the system default.
    pub fn output_device(&self) -> Option<OutputDevice> {
        self.state.lock().chosen.clone()
    }

    /// The output devices to choose from, as the core last listed them (`ListOutputDevices`
    /// lists them again), after the chosen one while it is not there.
    pub fn output_choices(&self) -> Vec<OutputChoice> {
        self.state.lock().output_choices()
    }

    /// Plays on the output device with `id`, one of `output_choices`; `None` on the system
    /// default.
    pub fn choose_output_device(&self, id: Option<String>) {
        let device = {
            let mut state = self.state.lock();
            let device = id.map(|id| state.device(id));
            state.chosen = device.clone();
            device
        };
        self.writer.emit(SetOutputDevice(device));
    }

    /// Where signing in to Telegram is; `None` until the core tells, as in a build without
    /// Telegram: there is no Telegram then.
    pub fn telegram_state(&self) -> Option<TelegramSignInState> {
        self.state.lock().telegram.clone()
    }

    /// The stream cache as the core last reported it; `None` before it did.
    pub fn stream_cache(&self) -> Option<StreamCacheState> {
        self.state.lock().cache
    }

    /// The desktop interface's preferences, kept under `desktop.ui` as the Qt app keeps them.
    pub fn desktop_ui(&self) -> DesktopUi {
        DesktopUi::read(self.storage.load(ui::KEY).as_ref())
    }

    /// Saves the desktop interface's preferences under `desktop.ui`, keeping what else the
    /// section holds.
    pub fn set_desktop_ui(&self, preferences: DesktopUi) {
        let section = preferences.write(self.storage.load(ui::KEY));
        self.storage.store(ui::KEY, section);
    }

    /// The value of `field` in the settings section `key` as JSON; `None` when there is none.
    pub fn setting_field(&self, key: String, field: String) -> Option<String> {
        self.storage
            .load(&key)?
            .get(&field)
            .map(|value| value.to_string())
    }

    /// Stores `json` as `field` of the settings section `key`, keeping its other fields; text
    /// that is not JSON is dropped.
    pub fn set_setting_field(&self, key: String, field: String, json: String) {
        let value: Value = match serde_json::from_str(&json) {
            Ok(value) => value,
            Err(error) => {
                log::error!("Not storing settings field {key}.{field}: {error}");
                return;
            }
        };
        let mut section = match self.storage.load(&key) {
            Some(Value::Object(section)) => section,
            _ => serde_json::Map::new(),
        };
        section.insert(field, value);
        self.storage.store(&key, Value::Object(section));
    }

    /// The folder the logs are written to, beside the settings.
    pub fn logs_dir(&self) -> String {
        self.data_dir.to_string_lossy().into_owned()
    }
}

/// The version of the core, like `1.2.0`.
#[uniffi::export]
pub fn core_version() -> String {
    String::from(env!("CARGO_PKG_VERSION"))
}

/// `name` as typed: `None` when blank.
fn named(name: &str) -> Option<String> {
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_string())
}
