use super::{version_stamp, Locator, OpenedStream, StreamProvider, TrackEntry};
use std::collections::HashSet;
use std::fs::{DirEntry, File};
use std::io;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

/// Reads tracks straight from the local file system.
#[derive(Default)]
pub struct LocalProvider;

impl StreamProvider for LocalProvider {
    fn open(&self, locator: &Locator) -> io::Result<OpenedStream> {
        let Locator::Local(path) = locator else {
            return Err(unsupported(locator));
        };
        let file = File::open(path).map_err(|error| {
            io::Error::new(error.kind(), format!("Could not open {path:?}: {error}"))
        })?;
        Ok(OpenedStream {
            source: Box::new(file),
            extension: locator.extension(),
        })
    }

    /// Lists the folder and every folder inside it but hidden ones, following links. What
    /// several paths lead to is listed once, by the first path found: a folder's own files
    /// first, then its folders, in name order.
    fn list_tracks(&self, root: &Locator) -> io::Result<Vec<TrackEntry>> {
        let Locator::Local(root) = root else {
            return Err(unsupported(root));
        };
        let root = PathBuf::from(root);
        let mut tracks = vec![];
        // Where the files and folders found are, links resolved.
        let mut listed = HashSet::new();
        let mut first =
            |path: &PathBuf| listed.insert(path.canonicalize().unwrap_or_else(|_| path.clone()));
        let mut folders = vec![root.clone()];
        while let Some(folder) = folders.pop() {
            // A link back up the tree would otherwise be followed forever.
            if !first(&folder) {
                continue;
            }
            let entries = match std::fs::read_dir(&folder) {
                Ok(entries) => entries,
                // The library itself has to be there; a folder inside that cannot be read is
                // left out.
                Err(error) if folder == root => return Err(error),
                Err(error) => {
                    log::debug!("Could not list {}: {error}", folder.display());
                    continue;
                }
            };
            let mut entries: Vec<DirEntry> = entries.flatten().collect();
            entries.sort_by_key(DirEntry::file_name);
            let mut inside = vec![];
            for entry in entries {
                let path = entry.path();
                let Ok(metadata) = std::fs::metadata(&path) else {
                    continue;
                };
                if metadata.is_dir() {
                    if !entry.file_name().to_string_lossy().starts_with('.') {
                        inside.push(path);
                    }
                    continue;
                }
                if !metadata.is_file() {
                    continue;
                }
                let Ok(Some(mime)) = infer::get_from_path(&path) else {
                    continue;
                };
                if !mime.mime_type().contains("audio") || !first(&path) {
                    continue;
                }
                let version = metadata
                    .modified()
                    .ok()
                    .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                    .map(|modified| version_stamp(metadata.len(), modified.as_millis() as u64));
                tracks.push(TrackEntry {
                    locator: Locator::Local(path.to_string_lossy().into_owned()),
                    version,
                });
            }
            // Popped in name order.
            folders.extend(inside.into_iter().rev());
        }
        Ok(tracks)
    }
}

fn unsupported(locator: &Locator) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{locator} is not a local path"),
    )
}
