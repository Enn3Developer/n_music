//! The folders of this computer, browsed to pick one.

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
        /// The folder shown, as the library writes local paths; empty until `open`.
        #[qproperty(QString, path)]
        /// The folder holding `path`; empty at the top of the file system.
        #[qproperty(QString, parent_path)]
        /// The deepest of `places` holding `path`: its path, or empty.
        #[qproperty(QString, place)]
        /// The way down to `path`, as `{ name, path, kind }`: from the place it is in, with that
        /// place's `kind` (see `places`), then each folder, with an empty one.
        #[qproperty(QVariant, crumbs)]
        /// The folders in `path` as `{ name, path, hidden }`, sorted by name; the hidden ones
        /// only with `show_hidden`.
        #[qproperty(QVariant, folders)]
        /// Hidden folders are listed too; changing it lists `path` again.
        #[qproperty(bool, show_hidden)]
        /// `path` was listed, or could not be: see `problem`.
        #[qproperty(bool, ready)]
        /// Why `path` could not be listed: `missing` when it is no folder, `unreadable` when it
        /// cannot be read; empty otherwise.
        #[qproperty(QString, problem)]
        /// Where browsing goes from, as `{ name, path, kind }`: `kind` is `home`, `music`,
        /// `root` (the whole file system) or `drive` (one mounted, which `name` names).
        #[qproperty(QVariant, places)]
        type FolderBrowser = super::FolderBrowserRust;

        /// Shows the folder `text` names: a path, from `path` when relative, `~` standing for
        /// the home folder, or a `file:` URL. False when that is no absolute path; one naming
        /// no folder shows as `missing`.
        #[qinvokable]
        fn open(self: Pin<&mut FolderBrowser>, text: &QString) -> bool;
        /// Looks for the drives again, as they come and go.
        #[qinvokable]
        fn find_places(self: Pin<&mut FolderBrowser>);
        /// Where browsing starts: the music folder, else the home folder.
        #[qinvokable]
        fn start(self: &FolderBrowser) -> QString;
    }

    impl cxx_qt::Threading for FolderBrowser {}
    impl cxx_qt::Initialize for FolderBrowser {}
}

use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QUrl, QVariant};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR};

pub struct FolderBrowserRust {
    path: QString,
    parent_path: QString,
    place: QString,
    crumbs: QVariant,
    folders: QVariant,
    show_hidden: bool,
    ready: bool,
    problem: QString,
    places: QVariant,
    /// What `places` lists.
    found: Vec<Place>,
    /// Counts the listings asked for: only the last one shows.
    listing: u64,
}

impl Default for FolderBrowserRust {
    fn default() -> Self {
        // Lists from the start, so QML can go through them before anything is listed.
        let empty = QVariant::from(&QList::<QVariant>::default());
        Self {
            path: QString::default(),
            parent_path: QString::default(),
            place: QString::default(),
            crumbs: empty.clone(),
            folders: empty.clone(),
            show_hidden: false,
            ready: false,
            problem: QString::default(),
            places: empty,
            found: vec![],
            listing: 0,
        }
    }
}

/// Somewhere browsing goes from, see `places`.
#[derive(Clone)]
struct Place {
    kind: &'static str,
    name: String,
    path: PathBuf,
}

impl Place {
    fn new(kind: &'static str, path: PathBuf) -> Self {
        Self {
            kind,
            name: String::new(),
            path,
        }
    }

    /// A drive mounted at `path`, named after where it is.
    fn drive(path: PathBuf) -> Self {
        let name = path.file_name().map_or_else(
            || text_of(&path).trim_end_matches(['/', '\\']).to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        Self {
            kind: "drive",
            name,
            path,
        }
    }
}

/// A folder in the one shown.
struct Folder {
    name: String,
    path: PathBuf,
    hidden: bool,
}

impl cxx_qt::Initialize for qobject::FolderBrowser {
    /// The drives come later, see `find_places`.
    fn initialize(mut self: Pin<&mut Self>) {
        self.as_mut()
            .on_show_hidden_changed(|browser| browser.relist())
            .release();
        self.show_places(standard_places());
    }
}

impl qobject::FolderBrowser {
    fn open(mut self: Pin<&mut Self>, text: &QString) -> bool {
        let Some(path) = resolve(Path::new(&self.path.to_string()), &text.to_string()) else {
            return false;
        };
        self.as_mut().show_way(&path);
        self.as_mut().set_parent_path(
            path.parent()
                .map_or_else(QString::default, |parent| QString::from(&text_of(parent))),
        );
        self.as_mut()
            .set_folders(QVariant::from(&QList::<QVariant>::default()));
        self.as_mut().set_problem(QString::default());
        self.as_mut().set_ready(false);
        self.as_mut().set_path(QString::from(&text_of(&path)));
        self.list_in_background(path);
        true
    }

    /// Lists `path` again, showing what it listed until then.
    fn relist(self: Pin<&mut Self>) {
        if !self.path.is_empty() {
            let path = PathBuf::from(self.path.to_string());
            self.list_in_background(path);
        }
    }

    /// Lists `path` away from the interface: a folder on a slow drive or across the network can
    /// take long to list.
    fn list_in_background(mut self: Pin<&mut Self>, path: PathBuf) {
        self.as_mut().rust_mut().listing += 1;
        let listing = self.listing;
        let show_hidden = self.show_hidden;
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let listed = list(&path, show_hidden);
            let _ = thread.queue(move |browser| browser.show_folders(listing, listed));
        });
    }

    fn find_places(self: Pin<&mut Self>) {
        let thread = self.qt_thread();
        // Telling whether a drive is there can wait on the network.
        std::thread::spawn(move || {
            let mut places = standard_places();
            places.extend(drives());
            let _ = thread.queue(move |browser| browser.show_places(places));
        });
    }

    fn start(&self) -> QString {
        let start = directories::UserDirs::new().map(|dirs| {
            dirs.audio_dir()
                .filter(|music| music.is_dir())
                .unwrap_or(dirs.home_dir())
                .to_path_buf()
        });
        QString::from(&text_of(&start.unwrap_or_else(root)))
    }

    fn show_places(mut self: Pin<&mut Self>, places: Vec<Place>) {
        let records = places.iter().map(|place| {
            [
                ("name", text(&place.name)),
                ("path", text(&text_of(&place.path))),
                ("kind", text(place.kind)),
            ]
        });
        self.as_mut().set_places(list_of(records));
        self.as_mut().rust_mut().found = places;
        // The way to the folder shown may now start from a drive.
        if !self.path.is_empty() {
            let path = PathBuf::from(self.path.to_string());
            self.show_way(&path);
        }
    }

    /// Shows the place `path` is in and the way from it.
    fn show_way(mut self: Pin<&mut Self>, path: &Path) {
        let place = self
            .found
            .iter()
            .filter(|place| path.starts_with(&place.path))
            .max_by_key(|place| place.path.components().count())
            .map_or_else(String::new, |place| text_of(&place.path));
        let records = crumbs(path, &self.found)
            .into_iter()
            .map(|(name, path, kind)| {
                [
                    ("name", text(&name)),
                    ("path", text(&text_of(&path))),
                    ("kind", text(kind)),
                ]
            });
        self.as_mut().set_crumbs(list_of(records));
        self.set_place(QString::from(&place));
    }

    fn show_folders(mut self: Pin<&mut Self>, listing: u64, listed: io::Result<Vec<Folder>>) {
        if listing != self.listing {
            return;
        }
        match listed {
            Ok(folders) => {
                let records = folders.iter().map(|folder| {
                    [
                        ("name", text(&folder.name)),
                        ("path", text(&text_of(&folder.path))),
                        ("hidden", QVariant::from(&folder.hidden)),
                    ]
                });
                self.as_mut().set_folders(list_of(records));
            }
            Err(error) => {
                let problem = match error.kind() {
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => "missing",
                    _ => "unreadable",
                };
                self.as_mut().set_problem(QString::from(problem));
            }
        }
        self.set_ready(true);
    }
}

/// The folder `text` names, see `open`; `None` when that is no absolute path.
fn resolve(current: &Path, text: &str) -> Option<PathBuf> {
    let text = text.trim();
    let path = if text.starts_with("file:") {
        PathBuf::from(QUrl::from(text).to_local_file()?.to_string())
    } else {
        match text.strip_prefix('~') {
            Some(rest) if rest.is_empty() || rest.starts_with(['/', MAIN_SEPARATOR]) => {
                let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
                home.join(rest.trim_start_matches(['/', MAIN_SEPARATOR]))
            }
            _ if text.is_empty() => return None,
            _ => current.join(text),
        }
    };
    // `..` leaves the folder before it, whether or not that was reached through a link.
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other),
        }
    }
    normal.has_root().then_some(normal)
}

/// The way down to `path` as `(name, path, kind)`: from the deepest place holding it but the
/// music folder, which is in the home folder, or the top of the file system.
fn crumbs(path: &Path, places: &[Place]) -> Vec<(String, PathBuf, &'static str)> {
    let start = places
        .iter()
        .filter(|place| place.kind != "music" && path.starts_with(&place.path))
        .max_by_key(|place| place.path.components().count())
        .cloned()
        .unwrap_or_else(|| {
            let top = path.ancestors().last().unwrap_or(path).to_path_buf();
            if top == root() {
                Place::new("root", top)
            } else {
                Place::drive(top)
            }
        });
    let mut crumbs = vec![(start.name, start.path.clone(), start.kind)];
    let mut way = start.path.clone();
    for component in path.strip_prefix(&start.path).into_iter().flatten() {
        way.push(component);
        crumbs.push((component.to_string_lossy().into_owned(), way.clone(), ""));
    }
    crumbs
}

/// The folders in `path`, sorted by name whatever its case, the hidden ones only with
/// `show_hidden`; a link to a folder counts as one.
fn list(path: &Path, show_hidden: bool) -> io::Result<Vec<Folder>> {
    let mut folders = vec![];
    for entry in fs::read_dir(path)?.flatten() {
        let folder = match entry.file_type() {
            Ok(kind) if kind.is_symlink() => entry.path().is_dir(),
            Ok(kind) => kind.is_dir(),
            Err(_) => false,
        };
        let hidden = folder && hidden(&entry);
        if folder && (show_hidden || !hidden) {
            folders.push(Folder {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path(),
                hidden,
            });
        }
    }
    folders.sort_by_cached_key(|folder| (folder.name.to_lowercase(), folder.name.clone()));
    Ok(folders)
}

#[cfg(windows)]
fn hidden(entry: &fs::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    entry
        .metadata()
        .is_ok_and(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
}

#[cfg(not(windows))]
fn hidden(entry: &fs::DirEntry) -> bool {
    entry.file_name().as_encoded_bytes().starts_with(b".")
}

/// The home and music folders, and the whole file system where it has one top.
fn standard_places() -> Vec<Place> {
    let mut places = vec![];
    if let Some(dirs) = directories::UserDirs::new() {
        let home = dirs.home_dir();
        places.push(Place::new("home", home.to_path_buf()));
        if let Some(music) = dirs
            .audio_dir()
            .filter(|music| *music != home && music.is_dir())
        {
            places.push(Place::new("music", music.to_path_buf()));
        }
    }
    if cfg!(unix) {
        places.push(Place::new("root", root()));
    }
    places
}

#[cfg(unix)]
fn root() -> PathBuf {
    PathBuf::from("/")
}

#[cfg(windows)]
fn root() -> PathBuf {
    PathBuf::from("C:\\")
}

/// The drives mounted where udisks and people mount them.
#[cfg(target_os = "linux")]
fn drives() -> Vec<Place> {
    let mounts = fs::read_to_string("/proc/self/mounts").unwrap_or_default();
    let mut drives: Vec<Place> = vec![];
    for line in mounts.lines() {
        let Some(point) = line.split(' ').nth(1).map(unescape) else {
            continue;
        };
        let path = PathBuf::from(&point);
        let mounted = ["/run/media/", "/media/", "/mnt/"]
            .iter()
            .any(|dir| point.starts_with(dir));
        if mounted && !drives.iter().any(|drive| drive.path == path) {
            drives.push(Place::drive(path));
        }
    }
    drives
}

/// A mount point as `/proc` writes it, with `\040` for a space and the like.
#[cfg(target_os = "linux")]
fn unescape(field: &str) -> String {
    let mut bytes = Vec::with_capacity(field.len());
    let mut rest = field.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        let octal = tail
            .get(..3)
            .filter(|digits| digits.iter().all(|digit| (b'0'..=b'7').contains(digit)));
        match octal {
            Some(digits) if byte == b'\\' => {
                let value = digits
                    .iter()
                    .fold(0u32, |value, digit| value * 8 + u32::from(digit - b'0'));
                bytes.push(u8::try_from(value).unwrap_or(b'?'));
                rest = &tail[3..];
            }
            _ => {
                bytes.push(byte);
                rest = tail;
            }
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The volumes but the startup disk, which shows there as a link to the root.
#[cfg(target_os = "macos")]
fn drives() -> Vec<Place> {
    let mut drives: Vec<Place> = fs::read_dir("/Volumes")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|volume| volume.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|volume| Place::drive(volume.path()))
        .collect();
    drives.sort_by(|a, b| a.name.cmp(&b.name));
    drives
}

#[cfg(windows)]
fn drives() -> Vec<Place> {
    (b'A'..=b'Z')
        .map(|letter| PathBuf::from(format!("{}:\\", char::from(letter))))
        .filter(|drive| drive.is_dir())
        .map(Place::drive)
        .collect()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn drives() -> Vec<Place> {
    vec![]
}

fn text_of(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Records, as QML reads `{ key: value }` objects.
fn list_of<const N: usize>(
    records: impl Iterator<Item = [(&'static str, QVariant); N]>,
) -> QVariant {
    let mut list = QList::<QVariant>::default();
    for record in records {
        let mut map = QMap::<QMapPair_QString_QVariant>::default();
        for (key, value) in record {
            map.insert(QString::from(key), value);
        }
        list.append(QVariant::from(&map));
    }
    QVariant::from(&list)
}

fn text(value: &str) -> QVariant {
    QVariant::from(&QString::from(value))
}
