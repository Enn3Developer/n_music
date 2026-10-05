//! A file on the web, read while a thread downloads it ahead of reading: the decoder shares its
//! thread with the playback controls, so it must not wait on the network.

use super::failed;
use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom};
use std::mem;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use symphonia::core::io::MediaSource;
use ureq::http::{Response, StatusCode};
use ureq::{Agent, Body};

/// How far ahead of reading the download goes, at least and at most: in between, as far as
/// reading got since it last jumped. Reading tags then fetches little, and playing keeps
/// seconds of audio at hand.
const MIN_AHEAD: u64 = 256 * 1024;
const MAX_AHEAD: u64 = 8 * 1024 * 1024;
/// Reading this soon after opening is taken for reading tags, however far it goes: the download
/// keeps only `MIN_AHEAD` ahead.
const SETTLING: Duration = Duration::from_secs(1);
/// Kept behind reading, for the short seeks back of probing a format.
const BEHIND: u64 = 256 * 1024;
/// Read from the network at a time.
const CHUNK: usize = 64 * 1024;
/// Reading gives up when the download did not move for this long.
const STALL: Duration = Duration::from_secs(20);
/// A request for part of the file has this long to bring it.
const PART_TIMEOUT: Duration = Duration::from_secs(120);
/// The waits before trying again after the download failed, one per attempt.
const RETRIES: [Duration; 3] = [
    Duration::from_millis(250),
    Duration::from_secs(1),
    Duration::from_secs(3),
];

/// A file on the web as a [`MediaSource`]. Where the server answers range requests, a jump
/// costs a request; elsewhere the file comes in one piece, and jumping back downloads it again.
pub(super) struct WebFile {
    shared: Arc<Shared>,
    /// Where reading is.
    position: u64,
    /// The file's size, when the server tells.
    len: Option<u64>,
    /// The server answers range requests.
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
    /// Where reading went on from when it last jumped.
    since: u64,
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
            since: 0,
            jumps: 0,
            complete: false,
            error: None,
            closed: false,
        }
    }

    /// How far ahead of reading the download goes.
    fn ahead(&self) -> u64 {
        if self.opened.elapsed() < SETTLING {
            return MIN_AHEAD;
        }
        self.position
            .saturating_sub(self.since)
            .clamp(MIN_AHEAD, MAX_AHEAD)
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
        self.since = to;
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

impl WebFile {
    /// Opens the file at `address`, once the server answered.
    pub(super) fn open(agent: Agent, address: &str) -> io::Result<Self> {
        let response = first_response(&agent, address)?;
        let (ranges, len) = if response.status() == StatusCode::PARTIAL_CONTENT {
            let (_, len) = content_range(&response)
                .filter(|&(start, _)| start == 0)
                .ok_or_else(|| other_part(address))?;
            (true, Some(len))
        } else {
            (false, response.body().content_length())
        };
        let shared = Arc::new(Shared {
            state: Mutex::new(State::new()),
            changed: Condvar::new(),
        });
        let download = Download {
            shared: shared.clone(),
            agent,
            address: address.to_string(),
            len,
            ranges,
        };
        let expected = if ranges {
            len.map(|len| len.min(MIN_AHEAD))
        } else {
            len
        };
        std::thread::Builder::new()
            .name("web download".to_string())
            .spawn(move || download.run(response, expected))?;
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

impl Read for WebFile {
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

impl Seek for WebFile {
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

impl MediaSource for WebFile {
    fn is_seekable(&self) -> bool {
        self.ranges
    }

    fn byte_len(&self) -> Option<u64> {
        self.len
    }
}

impl Drop for WebFile {
    fn drop(&mut self) {
        self.shared.lock().closed = true;
        self.shared.changed.notify_all();
    }
}

/// Fills a [`WebFile`] ahead of where it is read, until it is dropped.
struct Download {
    shared: Arc<Shared>,
    agent: Agent,
    address: String,
    len: Option<u64>,
    ranges: bool,
}

impl Download {
    /// Reads `first`, the response to the opening request, which brings the file up to
    /// `expected`, then asks for more as reading goes.
    fn run(self, first: Response<Body>, expected: Option<u64>) {
        let mut pending = Some((first, expected));
        let mut jumps = 0;
        let mut failures = 0;
        loop {
            let (response, expected) = match pending.take() {
                Some(pending) => pending,
                None => {
                    let before = jumps;
                    let Some((from, to)) = self.wanted(&mut jumps) else {
                        return;
                    };
                    if jumps != before {
                        failures = 0;
                    }
                    match self.request(from, to) {
                        Ok(response) => (response, to.or(self.len)),
                        Err(error) => {
                            if !self.retry(error, &mut failures, jumps) {
                                return;
                            }
                            continue;
                        }
                    }
                }
            };
            match self.receive(response, expected, jumps) {
                Ok(()) => failures = 0,
                Err(error) => {
                    let error = io::Error::new(
                        error.kind(),
                        format!("Could not download {}: {error}", self.address),
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
                    // Asked once the last response is over, or when starting over.
                    if !state.complete {
                        return Some((0, None));
                    }
                } else {
                    let len = self.len.unwrap_or(u64::MAX);
                    let end = state.window.end();
                    if end < len && end < state.position + state.ahead() / 2 {
                        let to = (state.position + state.ahead())
                            .max(end + CHUNK as u64)
                            .min(len);
                        return Some((end, Some(to)));
                    }
                }
            }
            state = self.shared.changed.wait(state).unwrap();
        }
    }

    /// Asks for the part from `from`, up to `to` when not to the end.
    fn request(&self, from: u64, to: Option<u64>) -> io::Result<Response<Body>> {
        let response = request(&self.agent, &self.address, from, to, true)
            .map_err(|error| failed(&self.address, error))?;
        if self.ranges
            && (response.status() != StatusCode::PARTIAL_CONTENT
                || content_range(&response).map(|(start, _)| start) != Some(from))
        {
            return Err(other_part(&self.address));
        }
        Ok(response)
    }

    /// Adds the body of `response`, which brings the file up to `expected`, to the download;
    /// without ranges, as fast as reading goes. Stops early when reading jumps or the file is
    /// dropped.
    fn receive(
        &self,
        response: Response<Body>,
        expected: Option<u64>,
        jumps: u64,
    ) -> io::Result<()> {
        let mut body = response.into_body().into_reader();
        let mut chunk = vec![0; CHUNK];
        loop {
            let read = match body.read(&mut chunk) {
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            let mut state = self.shared.lock();
            if state.closed || state.jumps != jumps {
                return Ok(());
            }
            if read == 0 {
                if expected.is_some_and(|expected| state.window.end() < expected) {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the response ended early",
                    ));
                }
                if !self.ranges {
                    state.complete = true;
                    self.shared.changed.notify_all();
                }
                return Ok(());
            }
            state.window.data.extend(&chunk[..read]);
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
            log::debug!("Giving up downloading {}: {error}", self.address);
            state.error = Some((error.kind(), error.to_string()));
            self.shared.changed.notify_all();
            return true;
        };
        *failures += 1;
        log::debug!("Downloading {} again in {wait:?}: {error}", self.address);
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

/// The response to the opening request, asked again a couple of times when the server failed or
/// the connection broke.
fn first_response(agent: &Agent, address: &str) -> io::Result<Response<Body>> {
    let mut waits = RETRIES[..2].iter();
    loop {
        let error = match request(agent, address, 0, Some(MIN_AHEAD), false) {
            Ok(response) => return Ok(response),
            Err(error) => error,
        };
        let passing = match &error {
            ureq::Error::StatusCode(code) => *code >= 500,
            ureq::Error::Io(error) => matches!(
                error.kind(),
                io::ErrorKind::ConnectionReset
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::BrokenPipe
                    | io::ErrorKind::UnexpectedEof
            ),
            _ => false,
        };
        match waits.next().filter(|_| passing) {
            Some(&wait) => {
                log::debug!("Opening {address} again in {wait:?}: {error}");
                std::thread::sleep(wait);
            }
            None => return Err(failed(address, error)),
        }
    }
}

/// Asks for the file at `address` from `from`, up to `to` (excluded) when not to its end. A
/// `bounded` request has a time limit on its body; a whole file takes as long as playing it.
fn request(
    agent: &Agent,
    address: &str,
    from: u64,
    to: Option<u64>,
    bounded: bool,
) -> Result<Response<Body>, ureq::Error> {
    let mut request = agent.get(address).header("Accept-Encoding", "identity");
    match to {
        Some(to) => request = request.header("Range", format!("bytes={from}-{}", to - 1)),
        None if from > 0 => request = request.header("Range", format!("bytes={from}-")),
        None => {}
    }
    if bounded {
        request = request
            .config()
            .timeout_recv_body(Some(PART_TIMEOUT))
            .build();
    }
    request.call()
}

/// Where the part a response brings starts and the file's size, from its `Content-Range`.
fn content_range(response: &Response<Body>) -> Option<(u64, u64)> {
    let value = response.headers().get("content-range")?.to_str().ok()?;
    let (range, len) = value.strip_prefix("bytes ")?.split_once('/')?;
    let (start, _) = range.split_once('-')?;
    Some((start.trim().parse().ok()?, len.trim().parse().ok()?))
}

fn other_part(address: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{address} did not send the part of the file asked for"),
    )
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
