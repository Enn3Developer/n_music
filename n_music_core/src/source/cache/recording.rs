//! A track's bytes written to a file as they are read, wherever reading goes.

use super::StreamCache;
use crate::source::{Locator, StreamProvider};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use symphonia::core::io::MediaSource;

/// A track being written to a file: what playback reads, then what a download of the rest
/// brings.
pub(super) struct Recording {
    pub(super) key: u64,
    pub(super) locator: Locator,
    /// Opens the track again to download what playback did not read.
    pub(super) provider: Arc<dyn StreamProvider>,
    pub(super) path: PathBuf,
    written: Mutex<Written>,
}

struct Written {
    /// `None` once the recording is closed or writing failed.
    file: Option<File>,
    /// What was written, in order and apart.
    parts: Vec<Range<u64>>,
    /// The track's size, as its stream told or where reading met the end.
    len: Option<u64>,
    /// Writing failed, or the cache was turned off: it cannot become a copy.
    failed: bool,
    /// It became a copy.
    kept: bool,
}

impl Written {
    fn complete(&self) -> bool {
        !self.failed
            && self
                .len
                .is_some_and(|len| self.parts.first() == Some(&(0..len)))
    }
}

impl Recording {
    /// Starts recording `locator` into a new file at `path`.
    pub(super) fn create(
        key: u64,
        locator: Locator,
        provider: Arc<dyn StreamProvider>,
        path: PathBuf,
        len: Option<u64>,
    ) -> io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self {
            key,
            locator,
            provider,
            path,
            written: Mutex::new(Written {
                file: Some(file),
                parts: vec![],
                len,
                failed: false,
                kept: false,
            }),
        })
    }

    /// Writes `data`, read at `position`. Returns whether that completed the recording.
    pub(super) fn write(&self, position: u64, data: &[u8]) -> bool {
        let mut written = self.lock();
        let Some(file) = &mut written.file else {
            return false;
        };
        let result = file
            .seek(SeekFrom::Start(position))
            .and_then(|_| file.write_all(data));
        match result {
            Ok(()) => {
                let complete = written.complete();
                add(&mut written.parts, position..position + data.len() as u64);
                !complete && written.complete()
            }
            Err(error) => {
                log::warn!("Could not record {}: {error}", self.locator);
                written.file = None;
                written.failed = true;
                false
            }
        }
    }

    /// Reading met the end of the track at `position`. Returns whether that completed the
    /// recording.
    pub(super) fn end(&self, position: u64) -> bool {
        let mut written = self.lock();
        let complete = written.complete();
        // A stream that ends before the size it told leaves a part missing.
        written.len.get_or_insert(position);
        !complete && written.complete()
    }

    /// What is still to write, as ranges whose end is `None` up to the end of a track of
    /// unknown size; `None` when writing failed.
    pub(super) fn missing(&self) -> Option<Vec<(u64, Option<u64>)>> {
        let written = self.lock();
        if written.failed {
            return None;
        }
        let mut missing = vec![];
        let mut at = 0;
        for part in &written.parts {
            if part.start > at {
                missing.push((at, Some(part.start)));
            }
            at = part.end;
        }
        match written.len {
            Some(len) if at < len => missing.push((at, Some(len))),
            Some(_) => {}
            None => missing.push((at, None)),
        }
        Some(missing)
    }

    /// The track's size, once known.
    pub(super) fn len(&self) -> Option<u64> {
        self.lock().len
    }

    /// Everything is written.
    pub(super) fn is_complete(&self) -> bool {
        self.lock().complete()
    }

    /// It became a copy.
    pub(super) fn is_kept(&self) -> bool {
        self.lock().kept
    }

    /// Closes the file, which became a copy: what is read on is not written any more.
    pub(super) fn keep(&self) {
        let mut written = self.lock();
        written.file = None;
        written.kept = true;
    }

    /// Stops writing: the recording will not become a copy.
    pub(super) fn abandon(&self) {
        let mut written = self.lock();
        written.file = None;
        written.failed = true;
    }

    /// Closes the file, to rename or delete it.
    pub(super) fn close(&self) {
        self.lock().file = None;
    }

    fn lock(&self) -> MutexGuard<'_, Written> {
        self.written.lock().unwrap()
    }
}

/// Adds `new` to `parts`, keeping them in order and apart: parts it overlaps or touches merge
/// with it.
fn add(parts: &mut Vec<Range<u64>>, new: Range<u64>) {
    if new.is_empty() {
        return;
    }
    let first = parts.partition_point(|part| part.end < new.start);
    let last = parts.partition_point(|part| part.start <= new.end);
    let merged = if first < last {
        parts[first].start.min(new.start)..parts[last - 1].end.max(new.end)
    } else {
        new
    };
    parts.splice(first..last, [merged]);
}

/// A stream that records what is read from it.
pub(super) struct Recorder {
    source: Box<dyn MediaSource>,
    recording: Arc<Recording>,
    cache: Arc<StreamCache>,
    position: u64,
}

impl Recorder {
    /// Records `source`, positioned at its first byte, into `recording`.
    pub(super) fn new(
        source: Box<dyn MediaSource>,
        recording: Arc<Recording>,
        cache: Arc<StreamCache>,
    ) -> Self {
        Self {
            source,
            recording,
            cache,
            position: 0,
        }
    }
}

impl Read for Recorder {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.source.read(buf)?;
        let completed = if count > 0 {
            self.recording.write(self.position, &buf[..count])
        } else {
            !buf.is_empty() && self.recording.end(self.position)
        };
        self.position += count as u64;
        if completed {
            // A track on repeat is read again before it is closed: it is kept now.
            self.cache.try_keep(&self.recording);
        }
        Ok(count)
    }
}

impl Seek for Recorder {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.position = self.source.seek(to)?;
        Ok(self.position)
    }
}

impl MediaSource for Recorder {
    fn is_seekable(&self) -> bool {
        self.source.is_seekable()
    }

    fn byte_len(&self) -> Option<u64> {
        self.source.byte_len()
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.cache.leave(&self.recording);
    }
}
