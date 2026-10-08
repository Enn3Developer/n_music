//! A remote file, read while a thread downloads it ahead of reading: the decoder shares its
//! thread with the playback controls, so it must not wait on the network. A [`Remote`] brings
//! the bytes, a part at a time: a file on the web, a file on Telegram.

use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom};
use std::mem;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use symphonia::core::io::MediaSource;

/// How far ahead of reading the download goes, at least and at most: in between, as much as
/// reading read since it last jumped. Reading tags then fetches little, and playing keeps
/// seconds of audio at hand.
pub(super) const MIN_AHEAD: u64 = 256 * 1024;
const MAX_AHEAD: u64 = 8 * 1024 * 1024;
/// How far ahead the download goes right after reading jumps, then twice what reading read
/// until that reaches `MIN_AHEAD`: seeking jumps around, reading a few KiB at each stop before
/// it lands.
const JUMP_AHEAD: u64 = 64 * 1024;
/// Reading this soon after opening is taken for reading tags, however far it goes: the download
/// keeps only `MIN_AHEAD` ahead.
const SETTLING: Duration = Duration::from_secs(1);
/// Kept behind reading, for the short seeks back of probing a format.
const BEHIND: u64 = 256 * 1024;
/// Read from the network at a time.
const CHUNK: usize = 64 * 1024;
/// When reading jumps away from a part with this much left at most, the rest still comes, for
/// the window reading left: an HTTP connection is used again only once its response was read to
/// the end, and a new one costs round trips of handshakes.
const DRAIN: u64 = 64 * 1024;
/// Reading gives up when the download did not move for this long.
const STALL: Duration = Duration::from_secs(20);
/// The waits before trying again after the download failed, one per attempt.
pub(super) const RETRIES: [Duration; 3] = [
    Duration::from_millis(250),
    Duration::from_secs(1),
    Duration::from_secs(3),
];

/// Where a [`RemoteFile`] comes from.
pub(super) trait Remote: Send + 'static {
    /// Asks for the part of the file from `from`, up to `to` (excluded) when not to its end.
    /// Without ranges, it is always asked from the start.
    fn part(&self, from: u64, to: Option<u64>) -> io::Result<Part>;

    /// Names the file in messages.
    fn name(&self) -> &str;
}

/// Part of a file, as a [`Remote`] brings it.
pub(super) struct Part {
    pub(super) body: Box<dyn Read + Send>,
    /// How many bytes it brings, when told.
    pub(super) len: Option<u64>,
}

/// A remote file as a [`MediaSource`]. With ranges, a jump costs a part; without, the file
/// comes in one piece, and jumping back downloads it again.
pub(super) struct RemoteFile {
    shared: Arc<Shared>,
    /// Where reading is.
    position: u64,
    /// The file's size, when known.
    len: Option<u64>,
    /// The remote brings parts from anywhere in the file.
    ranges: bool,
}

struct Shared {
    state: Mutex<State>,
    /// Notified on new data, on reading moving, and on closing.
    changed: Condvar,
}

/// Downloaded bytes, from `start`.
#[derive(Default)]
struct Window {
    data: VecDeque<u8>,
    start: u64,
}

struct State {
    opened: Instant,
    /// Where the download goes on.
    window: Window,
    /// The window reading left when it last jumped, for jumping back: probing a format reads
    /// the start, looks at the end, then reads on from the start.
    parked: Window,
    /// Where reading is.
    position: u64,
    /// How much reading read since it last jumped.
    read: u64,
    /// Counts the jumps of reading to where the download cannot get by going on.
    jumps: u64,
    /// Without ranges, the download reached the end of the file.
    complete: bool,
    /// Why the download stopped, for reading to tell.
    error: Option<(io::ErrorKind, String)>,
    /// The file was dropped.
    closed: bool,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }
}

impl Window {
    fn at(start: u64) -> Self {
        Self {
            data: VecDeque::new(),
            start,
        }
    }

    fn end(&self) -> u64 {
        self.start + self.data.len() as u64
    }

    /// Reading at `position` can use it, now or once it goes on.
    fn reaches(&self, position: u64) -> bool {
        self.start <= position && position <= self.end()
    }
}

impl State {
    fn new() -> Self {
        Self {
            opened: Instant::now(),
            window: Window::default(),
            parked: Window::default(),
            position: 0,
            read: 0,
            jumps: 0,
            complete: false,
            error: None,
            closed: false,
        }
    }

    /// How far ahead of reading the download goes.
    fn ahead(&self) -> u64 {
        if self.jumps > 0 && self.read < MIN_AHEAD / 2 {
            return (2 * self.read).max(JUMP_AHEAD);
        }
        if self.opened.elapsed() < SETTLING {
            return MIN_AHEAD;
        }
        self.read.clamp(MIN_AHEAD, MAX_AHEAD)
    }

    /// Drops what is too far behind reading.
    fn trim(&mut self) {
        let keep = self.position.saturating_sub(BEHIND);
        let window = &mut self.window;
        if keep > window.start {
            let dropped = (keep - window.start).min(window.data.len() as u64);
            window.data.drain(..dropped as usize);
            window.start += dropped;
        }
    }

    /// Reading jumped to `to`, where the download cannot get by going on. With ranges, the
    /// download goes on from the window parked there or starts there, parking the one left;
    /// without, it starts over from the start of the file.
    fn jump(&mut self, to: u64, ranges: bool) {
        self.jumps += 1;
        self.read = 0;
        if !ranges {
            self.restart();
            return;
        }
        self.error = None;
        let window = if self.parked.reaches(to) {
            mem::take(&mut self.parked)
        } else {
            Window::at(to)
        };
        let left = mem::replace(&mut self.window, window);
        if !left.data.is_empty() {
            self.parked = left;
        }
    }

    /// Without ranges: empties the download, to get the file again from the start.
    fn restart(&mut self) {
        self.window = Window::default();
        self.complete = false;
        self.error = None;
    }
}

impl RemoteFile {
    /// Reads the file `remote` brings, `first` being the part asked for to open it: from the
    /// start, up to `MIN_AHEAD` with `ranges`, else the whole file. `len` is the file's size,
    /// when known. A thread called `thread` downloads it; this returns once the start came.
    pub(super) fn open(
        remote: impl Remote,
        first: Part,
        ranges: bool,
        len: Option<u64>,
        thread: &str,
    ) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::new()),
            changed: Condvar::new(),
        });
        let download = Download {
            shared: shared.clone(),
            remote,
            len,
            ranges,
        };
        let expected = if ranges {
            len.map(|len| len.min(MIN_AHEAD))
        } else {
            len
        };
        std::thread::Builder::new()
            .name(thread.to_string())
            .spawn(move || download.run(first, expected))?;
        let file = Self {
            shared,
            position: 0,
            len,
            ranges,
        };
        // Probing a format may look at the end first: the start is then parked, not dropped.
        let first = expected.map_or(MIN_AHEAD, |expected| expected.min(MIN_AHEAD));
        if first > 0 {
            drop(wait_for(&file.shared, first - 1)?);
        }
        Ok(file)
    }
}

impl Read for RemoteFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.len.is_some_and(|len| self.position >= len) {
            return Ok(0);
        }
        let mut state = wait_for(&self.shared, self.position)?;
        let window = &state.window;
        if self.position >= window.end() {
            return Ok(0);
        }
        let count = copy(&window.data, (self.position - window.start) as usize, buf);
        self.position += count as u64;
        state.position = self.position;
        state.read += count as u64;
        state.trim();
        self.shared.changed.notify_all();
        Ok(count)
    }
}

/// Waits until the download has the byte at `position`, or reached the end of the file before
/// it; fails when the download did, or stalled.
fn wait_for(shared: &Shared, position: u64) -> io::Result<MutexGuard<'_, State>> {
    let mut state = shared.lock();
    let mut moved = (state.window.end(), Instant::now());
    loop {
        let window = &state.window;
        if window.start <= position && position < window.end() {
            return Ok(state);
        }
        if state.complete && position >= window.end() {
            return Ok(state);
        }
        if let Some((kind, message)) = &state.error {
            return Err(io::Error::new(*kind, message.clone()));
        }
        if window.end() != moved.0 {
            moved = (window.end(), Instant::now());
        }
        let left = STALL.saturating_sub(moved.1.elapsed());
        if left.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("The download stalled for {}s", STALL.as_secs()),
            ));
        }
        state = shared.changed.wait_timeout(state, left).unwrap().0;
    }
}

impl Seek for RemoteFile {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let target = match to {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::Current(offset) => self.position.checked_add_signed(offset),
            SeekFrom::End(offset) => {
                let len = self.len.ok_or_else(|| {
                    io::Error::new(io::ErrorKind::Unsupported, "The file's size is unknown")
                })?;
                len.checked_add_signed(offset)
            }
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Seek before the start"))?;
        if target != self.position {
            let mut state = self.shared.lock();
            // A little past the download, waiting is quicker than asking again; without ranges,
            // the download only goes forward.
            let window = &state.window;
            let near =
                window.start <= target && (!self.ranges || target <= window.end() + CHUNK as u64);
            if !near {
                state.jump(target, self.ranges);
            }
            self.position = target;
            state.position = target;
            self.shared.changed.notify_all();
        }
        Ok(target)
    }
}

impl MediaSource for RemoteFile {
    fn is_seekable(&self) -> bool {
        self.ranges
    }

    fn byte_len(&self) -> Option<u64> {
        self.len
    }
}

impl Drop for RemoteFile {
    fn drop(&mut self) {
        self.shared.lock().closed = true;
        self.shared.changed.notify_all();
    }
}

/// Fills a [`RemoteFile`] ahead of where it is read, until it is dropped.
struct Download<R> {
    shared: Arc<Shared>,
    remote: R,
    len: Option<u64>,
    ranges: bool,
}

impl<R: Remote> Download<R> {
    /// Reads `first`, the part asked for to open the file, which brings it up to `expected`,
    /// then asks for more as reading goes.
    fn run(self, first: Part, expected: Option<u64>) {
        let mut pending = Some((first, 0, expected));
        let mut jumps = 0;
        let mut failures = 0;
        loop {
            let (part, from, expected) = match pending.take() {
                Some(pending) => pending,
                None => {
                    let before = jumps;
                    let Some((from, to)) = self.wanted(&mut jumps) else {
                        return;
                    };
                    if jumps != before {
                        failures = 0;
                    }
                    match self.remote.part(from, to) {
                        Ok(part) => (part, from, to.or(self.len)),
                        Err(error) => {
                            if !self.retry(error, &mut failures, jumps) {
                                return;
                            }
                            continue;
                        }
                    }
                }
            };
            match self.receive(part, from, expected, jumps) {
                Ok(()) => failures = 0,
                Err(error) => {
                    let error = io::Error::new(
                        error.kind(),
                        format!("Could not download {}: {error}", self.remote.name()),
                    );
                    if !self.retry(error, &mut failures, jumps) {
                        return;
                    }
                }
            }
        }
    }

    /// Waits until more of the file is wanted, and tells which part: from where, and up to
    /// where when not to its end. `None` once the file is dropped.
    fn wanted(&self, jumps: &mut u64) -> Option<(u64, Option<u64>)> {
        let mut state = self.shared.lock();
        loop {
            if state.closed {
                return None;
            }
            *jumps = state.jumps;
            if state.error.is_none() {
                if !self.ranges {
                    // Asked once the last part is over, or when starting over.
                    if !state.complete {
                        return Some((0, None));
                    }
                } else {
                    let len = self.len.unwrap_or(u64::MAX);
                    let end = state.window.end();
                    let ahead = state.ahead();
                    if end < len && end < state.position + ahead / 2 {
                        let to = (state.position + ahead).max(end + CHUNK as u64);
                        // Less than a part left at the end comes too, not on a round trip of
                        // its own.
                        let to = if to.saturating_add(CHUNK as u64) >= len {
                            len
                        } else {
                            to
                        };
                        return Some((end, Some(to)));
                    }
                }
            }
            state = self.shared.changed.wait(state).unwrap();
        }
    }

    /// Adds what `part`, which brings the file from `at` up to `expected`, brings to the
    /// download; without ranges, as fast as reading goes. Stops early when the file is dropped,
    /// or when reading jumps with more than `DRAIN` of the part left.
    fn receive(
        &self,
        part: Part,
        mut at: u64,
        expected: Option<u64>,
        jumps: u64,
    ) -> io::Result<()> {
        let part_end = part.len.map(|len| at + len);
        let mut body = part.body;
        let mut chunk = vec![0; CHUNK];
        loop {
            let read = match body.read(&mut chunk) {
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            let mut state = self.shared.lock();
            if state.closed {
                return Ok(());
            }
            let data = &chunk[..read];
            if state.jumps != jumps {
                // Reading jumped away: what little is left still comes, for the window it left.
                let parked = &mut state.parked;
                if parked.end() == at && !parked.data.is_empty() {
                    parked.data.extend(data);
                }
                at += read as u64;
                let left = part_end.map_or(u64::MAX, |end| end.saturating_sub(at));
                if read == 0 || left > DRAIN {
                    return Ok(());
                }
                continue;
            }
            if read == 0 {
                if expected.is_some_and(|expected| state.window.end() < expected) {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the part ended early",
                    ));
                }
                if !self.ranges {
                    state.complete = true;
                    self.shared.changed.notify_all();
                }
                return Ok(());
            }
            state.window.data.extend(data);
            at += read as u64;
            state.trim();
            self.shared.changed.notify_all();
            if !self.ranges {
                while !state.closed
                    && state.jumps == jumps
                    && state.window.end() >= state.position + state.ahead()
                {
                    state = self.shared.changed.wait(state).unwrap();
                }
            }
        }
    }

    /// After `error`, waits to try again, or gives up and leaves the error for reading, until
    /// it jumps. Returns `false` once the file is dropped.
    fn retry(&self, error: io::Error, failures: &mut usize, jumps: u64) -> bool {
        let mut state = self.shared.lock();
        if state.closed || state.jumps != jumps {
            return !state.closed;
        }
        let retriable = !matches!(
            error.kind(),
            io::ErrorKind::NotFound
                | io::ErrorKind::PermissionDenied
                | io::ErrorKind::InvalidInput
                | io::ErrorKind::InvalidData
        );
        let Some(&wait) = RETRIES.get(*failures).filter(|_| retriable) else {
            log::debug!("Giving up downloading {}: {error}", self.remote.name());
            state.error = Some((error.kind(), error.to_string()));
            self.shared.changed.notify_all();
            return true;
        };
        *failures += 1;
        log::debug!(
            "Downloading {} again in {wait:?}: {error}",
            self.remote.name()
        );
        let until = Instant::now() + wait;
        while !state.closed && state.jumps == jumps {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            state = self.shared.changed.wait_timeout(state, left).unwrap().0;
        }
        // Without ranges, it can only start over.
        if !self.ranges && state.jumps == jumps {
            state.restart();
        }
        !state.closed
    }
}

/// Copies `data` from `offset` into `buf`, as much as fits; returns how much.
fn copy(data: &VecDeque<u8>, offset: usize, buf: &mut [u8]) -> usize {
    let (front, back) = data.as_slices();
    let mut copied = 0;
    let mut skip = offset;
    for part in [front, back] {
        if skip >= part.len() {
            skip -= part.len();
            continue;
        }
        let part = &part[skip..];
        skip = 0;
        let count = part.len().min(buf.len() - copied);
        buf[copied..copied + count].copy_from_slice(&part[..count]);
        copied += count;
        if copied == buf.len() {
            break;
        }
    }
    copied
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A file in memory, served in parts like a server would.
    struct Memory {
        data: Arc<Vec<u8>>,
        ranges: bool,
        /// Parts asked for, the opening one aside.
        asked: Arc<AtomicUsize>,
        /// The next parts asked for fail, this many of them.
        failing: Arc<AtomicUsize>,
    }

    impl Remote for Memory {
        fn part(&self, from: u64, to: Option<u64>) -> io::Result<Part> {
            self.asked.fetch_add(1, Ordering::SeqCst);
            if self
                .failing
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                    left.checked_sub(1)
                })
                .is_ok()
            {
                return Err(io::Error::new(io::ErrorKind::ConnectionReset, "reset"));
            }
            let from = if self.ranges { from } else { 0 };
            Ok(memory_part(&self.data, from, to.filter(|_| self.ranges)))
        }

        fn name(&self) -> &str {
            "memory"
        }
    }

    fn memory_part(data: &Arc<Vec<u8>>, from: u64, to: Option<u64>) -> Part {
        let end = to.map_or(data.len(), |to| (to as usize).min(data.len()));
        let bytes = data[from as usize..end].to_vec();
        Part {
            len: Some(bytes.len() as u64),
            body: Box::new(io::Cursor::new(bytes)),
        }
    }

    fn file(
        size: usize,
        ranges: bool,
    ) -> (RemoteFile, Arc<Vec<u8>>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let data: Arc<Vec<u8>> = Arc::new((0..size).map(|index| (index % 251) as u8).collect());
        let asked = Arc::new(AtomicUsize::new(0));
        let failing = Arc::new(AtomicUsize::new(0));
        let first = if ranges {
            memory_part(&data, 0, Some(MIN_AHEAD))
        } else {
            memory_part(&data, 0, None)
        };
        let remote = Memory {
            data: data.clone(),
            ranges,
            asked: asked.clone(),
            failing: failing.clone(),
        };
        let file =
            RemoteFile::open(remote, first, ranges, Some(size as u64), "test download").unwrap();
        (file, data, asked, failing)
    }

    fn read_all(file: &mut RemoteFile) -> Vec<u8> {
        let mut read = vec![];
        file.read_to_end(&mut read).unwrap();
        read
    }

    #[test]
    fn reads_the_whole_file_in_parts() {
        for ranges in [true, false] {
            let (mut file, data, _, _) = file(3 * 1024 * 1024 + 17, ranges);
            assert_eq!(read_all(&mut file), *data, "ranges: {ranges}");
        }
    }

    #[test]
    fn seeks_anywhere() {
        for ranges in [true, false] {
            let (mut file, data, _, _) = file(2 * 1024 * 1024, ranges);
            for offset in [1_500_000_u64, 10, 900_000, 2 * 1024 * 1024 - 5] {
                assert_eq!(file.seek(SeekFrom::Start(offset)).unwrap(), offset);
                let mut buf = [0; 5];
                file.read_exact(&mut buf).unwrap();
                let offset = offset as usize;
                assert_eq!(buf, data[offset..offset + 5], "ranges: {ranges}");
            }
            assert_eq!(file.seek(SeekFrom::End(-3)).unwrap(), 2 * 1024 * 1024 - 3);
            assert_eq!(read_all(&mut file), data[data.len() - 3..]);
        }
    }

    #[test]
    fn jumping_back_to_the_start_uses_what_it_parked() {
        let (mut file, data, asked, _) = file(4 * 1024 * 1024, true);
        let mut start = [0; 1024];
        file.read_exact(&mut start).unwrap();
        file.seek(SeekFrom::End(-1024)).unwrap();
        let mut end = [0; 1024];
        file.read_exact(&mut end).unwrap();
        let before = asked.load(Ordering::SeqCst);
        file.seek(SeekFrom::Start(1024)).unwrap();
        let mut next = [0; 1024];
        file.read_exact(&mut next).unwrap();
        assert_eq!(next, data[1024..2048]);
        assert_eq!(end, data[data.len() - 1024..]);
        // The parked start had those bytes already.
        assert_eq!(asked.load(Ordering::SeqCst), before);
    }

    #[test]
    fn tries_again_after_a_failure() {
        let (mut file, data, _, failing) = file(1024 * 1024, true);
        failing.store(1, Ordering::SeqCst);
        assert_eq!(read_all(&mut file), *data);
    }

    #[test]
    fn tells_why_it_gave_up() {
        let (mut file, _, _, failing) = file(1024 * 1024, true);
        failing.store(usize::MAX, Ordering::SeqCst);
        file.seek(SeekFrom::Start(900_000)).unwrap();
        let mut buf = [0; 16];
        let error = file.read(&mut buf).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::ConnectionReset);
    }
}
