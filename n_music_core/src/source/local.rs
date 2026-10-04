use super::{version_stamp, Locator, OpenedStream, StreamProvider, TrackEntry};
use std::fs::File;
use std::io;
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

    fn list_tracks(&self, root: &Locator) -> io::Result<Vec<TrackEntry>> {
        let Locator::Local(root) = root else {
            return Err(unsupported(root));
        };
        let mut tracks = vec![];
        for entry in std::fs::read_dir(root)?.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let path = entry.path();
            let Ok(Some(mime)) = infer::get_from_path(&path) else {
                continue;
            };
            if !mime.mime_type().contains("audio") {
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
        Ok(tracks)
    }
}

fn unsupported(locator: &Locator) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{locator} is not a local path"),
    )
}
