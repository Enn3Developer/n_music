//! A file on the web, read while a thread downloads it ahead of reading. Where the server
//! answers range requests, a jump costs a request; elsewhere the file comes in one piece, and
//! jumping back downloads it again.

use super::failed;
use crate::source::ahead::{Part, Remote, RemoteFile, MIN_AHEAD, RETRIES};
use std::io;
use std::time::Duration;
use ureq::http::{Response, StatusCode};
use ureq::{Agent, Body};

/// A request for part of the file has this long to bring it.
const PART_TIMEOUT: Duration = Duration::from_secs(120);

/// Opens the file at `address`, once the server answered.
pub(super) fn open(agent: Agent, address: &str) -> io::Result<RemoteFile> {
    let response = first_response(&agent, address)?;
    let (ranges, len) = if response.status() == StatusCode::PARTIAL_CONTENT {
        let (_, len) = content_range(&response)
            .filter(|&(start, _)| start == 0)
            .ok_or_else(|| other_part(address))?;
        (true, Some(len))
    } else {
        (false, response.body().content_length())
    };
    let remote = Http {
        agent,
        address: address.to_string(),
        ranges,
    };
    RemoteFile::open(remote, part(response), ranges, len, "web download")
}

/// The file at an address, a part at a time.
struct Http {
    agent: Agent,
    address: String,
    /// The server answers range requests.
    ranges: bool,
}

impl Remote for Http {
    fn part(&self, from: u64, to: Option<u64>) -> io::Result<Part> {
        let response = request(&self.agent, &self.address, from, to, true)
            .map_err(|error| failed(&self.address, error))?;
        if self.ranges
            && (response.status() != StatusCode::PARTIAL_CONTENT
                || content_range(&response).map(|(start, _)| start) != Some(from))
        {
            return Err(other_part(&self.address));
        }
        Ok(part(response))
    }

    fn name(&self) -> &str {
        &self.address
    }
}

/// What `response` brings.
fn part(response: Response<Body>) -> Part {
    let len = response.body().content_length();
    Part {
        body: Box::new(response.into_body().into_reader()),
        len,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use symphonia::core::io::MediaSource;

    /// Serves `data` on a local port until the test ends, answering range requests when
    /// `ranges`. Returns its address.
    fn serve(data: Arc<Vec<u8>>, ranges: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/song.flac", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    return;
                };
                let mut range = None;
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    let lower = line.to_ascii_lowercase();
                    if let Some(value) = lower.strip_prefix("range: bytes=") {
                        let (from, to) = value.trim().split_once('-').unwrap();
                        let from: usize = from.parse().unwrap();
                        let to = to.parse::<usize>().map_or(data.len(), |to| to + 1);
                        range = Some((from, to.min(data.len())));
                    }
                }
                let head = match range.filter(|_| ranges) {
                    Some((from, to)) => format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\n\
                         Content-Range: bytes {from}-{}/{}\r\nConnection: close\r\n\r\n",
                        to - from,
                        to - 1,
                        data.len()
                    ),
                    None => format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        data.len()
                    ),
                };
                let (from, to) = range.filter(|_| ranges).unwrap_or((0, data.len()));
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&data[from..to]);
            }
        });
        address
    }

    fn agent() -> Agent {
        Agent::config_builder().proxy(None).build().into()
    }

    #[test]
    fn reads_and_seeks_with_and_without_ranges() {
        let data: Arc<Vec<u8>> =
            Arc::new((0..1_500_000).map(|index| (index % 253) as u8).collect());
        for ranges in [true, false] {
            let address = serve(data.clone(), ranges);
            let mut file = open(agent(), &address).unwrap();
            assert_eq!(file.byte_len(), Some(data.len() as u64));
            assert_eq!(file.is_seekable(), ranges);
            file.seek(SeekFrom::Start(1_200_000)).unwrap();
            let mut buf = [0; 100];
            file.read_exact(&mut buf).unwrap();
            assert_eq!(buf, data[1_200_000..1_200_100], "ranges: {ranges}");
            file.seek(SeekFrom::Start(0)).unwrap();
            let mut all = vec![];
            file.read_to_end(&mut all).unwrap();
            assert_eq!(all, *data, "ranges: {ranges}");
        }
    }

    #[test]
    fn a_missing_file_is_not_found() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/gone.mp3", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut request = [0; 1024];
                let _ = stream.read(&mut request);
                let _ = stream.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });
        let error = open(agent(), &address).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
