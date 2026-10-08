//! A file read a chunk at a time, the way Telegram hands files over: chunks of 4 KiB to 512 KiB,
//! a power of two, each starting at a multiple of its size, so that none crosses a MiB.

use crate::source::ahead::Part;
use std::io::{self, Read};
use std::sync::{Arc, Mutex};

const MIN_CHUNK: u64 = 4 * 1024;
const MAX_CHUNK: u64 = 512 * 1024;
/// Reading that starts anywhere, like after a seek, starts at a multiple of this.
const START: u64 = 64 * 1024;

/// Downloads `limit` bytes of a file from `offset`; fewer at its end.
pub(super) type Fetch = Box<dyn Fn(u64, u64) -> io::Result<Vec<u8>> + Send + Sync>;

/// A file of `size` bytes, with what downloads its chunks.
pub(super) struct Chunks {
    fetch: Fetch,
    size: u64,
    /// The last chunk downloaded: the next part, which starts where the one before stopped,
    /// takes its first bytes from it instead of downloading them again.
    last: Mutex<Option<Chunk>>,
}

/// Bytes of the file, from `offset`.
#[derive(Clone)]
struct Chunk {
    offset: u64,
    bytes: Arc<Vec<u8>>,
}

impl Chunk {
    fn end(&self) -> u64 {
        self.offset + self.bytes.len() as u64
    }
}

impl Chunks {
    pub(super) fn new(size: u64, fetch: Fetch) -> Arc<Self> {
        Arc::new(Self {
            fetch,
            size,
            last: Mutex::new(None),
        })
    }

    /// The part of the file from `from`, up to `to` (excluded) when not to its end.
    pub(super) fn part(self: &Arc<Self>, from: u64, to: Option<u64>) -> Part {
        let to = to.unwrap_or(self.size).min(self.size);
        let last = self.last.lock().unwrap().clone();
        let (chunk, at, next) = match last {
            Some(last) if last.offset <= from && from < last.end() => {
                let at = (from - last.offset) as usize;
                let next = last.end();
                (last.bytes, at, next)
            }
            _ => (Arc::default(), 0, from - from % START),
        };
        let len = to.saturating_sub(from);
        Part {
            body: Box::new(ChunkPart {
                chunks: self.clone(),
                chunk,
                at,
                skip: from.saturating_sub(next) as usize,
                next,
                left: len,
            }),
            len: Some(len),
        }
    }
}

/// A part of a file, read a chunk at a time.
struct ChunkPart {
    chunks: Arc<Chunks>,
    chunk: Arc<Vec<u8>>,
    /// Where reading is in `chunk`.
    at: usize,
    /// Bytes of the next chunk before the part starts.
    skip: usize,
    /// Where the next chunk starts.
    next: u64,
    /// Bytes still to read.
    left: u64,
}

impl ChunkPart {
    /// Downloads the next chunk, as large as its offset allows and what is left needs.
    fn fetch(&mut self) -> io::Result<()> {
        let wanted = (self.skip as u64 + self.left).next_power_of_two();
        let limit = chunk_size(self.next).min(wanted.max(START));
        let bytes = (self.chunks.fetch)(self.next, limit)?;
        if bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Telegram sent no bytes",
            ));
        }
        let chunk = Chunk {
            offset: self.next,
            bytes: Arc::new(bytes),
        };
        self.next = chunk.end();
        self.at = self.skip.min(chunk.bytes.len());
        self.skip -= self.at;
        self.chunk = chunk.bytes.clone();
        *self.chunks.last.lock().unwrap() = Some(chunk);
        Ok(())
    }
}

impl Read for ChunkPart {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.left == 0 {
            return Ok(0);
        }
        while self.at >= self.chunk.len() {
            if self.next >= self.chunks.size {
                return Ok(0);
            }
            self.fetch()?;
        }
        let available = self.chunk.len() - self.at;
        let left = usize::try_from(self.left).unwrap_or(usize::MAX);
        let count = buf.len().min(available).min(left);
        buf[..count].copy_from_slice(&self.chunk[self.at..self.at + count]);
        self.at += count;
        self.left -= count as u64;
        Ok(count)
    }
}

/// The largest chunk Telegram takes from `offset`, a multiple of 4 KiB.
fn chunk_size(offset: u64) -> u64 {
    match offset {
        0 => MAX_CHUNK,
        offset => (1 << offset.trailing_zeros()).clamp(MIN_CHUNK, MAX_CHUNK),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::ahead::{Remote, RemoteFile, MIN_AHEAD};
    use std::io::{Seek, SeekFrom};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A file in memory, handed over the way Telegram does: it panics on a chunk Telegram
    /// would refuse.
    fn memory(data: &Arc<Vec<u8>>, fetched: &Arc<AtomicU64>) -> Arc<Chunks> {
        let (data, fetched) = (data.clone(), fetched.clone());
        Chunks::new(
            data.len() as u64,
            Box::new(move |offset, limit| {
                assert!(limit.is_power_of_two(), "limit {limit}");
                assert!((MIN_CHUNK..=MAX_CHUNK).contains(&limit), "limit {limit}");
                assert_eq!(offset % limit, 0, "offset {offset}, limit {limit}");
                assert!(offset < data.len() as u64, "offset {offset} past the end");
                let end = (offset + limit).min(data.len() as u64);
                fetched.fetch_add(end - offset, Ordering::SeqCst);
                Ok(data[offset as usize..end as usize].to_vec())
            }),
        )
    }

    struct Memory(Arc<Chunks>);

    impl Remote for Memory {
        fn part(&self, from: u64, to: Option<u64>) -> io::Result<Part> {
            Ok(self.0.part(from, to))
        }

        fn name(&self) -> &str {
            "memory"
        }
    }

    fn file(size: usize) -> (RemoteFile, Arc<Vec<u8>>, Arc<AtomicU64>) {
        let data: Arc<Vec<u8>> = Arc::new((0..size).map(|index| (index % 241) as u8).collect());
        let fetched = Arc::new(AtomicU64::new(0));
        let chunks = memory(&data, &fetched);
        let first = chunks.part(0, Some(MIN_AHEAD.min(size as u64)));
        let remote = Memory(chunks);
        let file =
            RemoteFile::open(remote, first, true, Some(size as u64), "test download").unwrap();
        (file, data, fetched)
    }

    #[test]
    fn chunks_start_at_a_multiple_of_their_size() {
        assert_eq!(chunk_size(0), MAX_CHUNK);
        assert_eq!(chunk_size(1024 * 1024), MAX_CHUNK);
        assert_eq!(chunk_size(256 * 1024), 256 * 1024);
        assert_eq!(chunk_size(64 * 1024 * 3), 64 * 1024);
        assert_eq!(chunk_size(4096 * 5), MIN_CHUNK);
        for offset in (0..8 * 1024 * 1024).step_by(4096) {
            let size = chunk_size(offset);
            assert_eq!(offset % size, 0);
            assert_eq!(offset / (1024 * 1024), (offset + size - 1) / (1024 * 1024));
        }
    }

    #[test]
    fn reading_through_downloads_every_byte_once() {
        let (mut file, data, fetched) = file(5 * 1024 * 1024 + 4321);
        let mut read = vec![];
        file.read_to_end(&mut read).unwrap();
        assert_eq!(read, *data);
        assert_eq!(fetched.load(Ordering::SeqCst), data.len() as u64);
    }

    #[test]
    fn parts_start_anywhere() {
        let data: Arc<Vec<u8>> = Arc::new((0..3_000_000).map(|index| index as u8).collect());
        let fetched = Arc::new(AtomicU64::new(0));
        let chunks = memory(&data, &fetched);
        for (from, to) in [
            (0, 10),
            (1_234_567, 1_300_000),
            (1_300_000, 2_999_999),
            (5, 3_000_000),
            (2_999_990, 3_000_000),
        ] {
            let mut part = chunks.part(from, Some(to));
            assert_eq!(part.len, Some(to - from));
            let mut read = vec![];
            part.body.read_to_end(&mut read).unwrap();
            assert_eq!(read, data[from as usize..to as usize], "{from}..{to}");
        }
    }

    #[test]
    fn seeks_read_the_right_bytes() {
        let (mut file, data, _) = file(3 * 1024 * 1024);
        for offset in [2_000_123_u64, 17, 1_048_575, 3 * 1024 * 1024 - 9] {
            file.seek(SeekFrom::Start(offset)).unwrap();
            let mut buf = [0; 9];
            file.read_exact(&mut buf).unwrap();
            let offset = offset as usize;
            assert_eq!(buf, data[offset..offset + 9]);
        }
    }
}
