//! The track after the current one, opened in the background: a slow source neither blocks
//! the playback loop nor holds up pause, seek or stop.

use super::decoding::{Decoding, PlaybackSource};
use super::Next;
use crate::library::track::ReplayGainMode;
use std::io;
use std::thread::JoinHandle;

pub(super) struct Successor {
    next: Next,
    state: State,
}

enum State {
    Opening(JoinHandle<io::Result<Decoding>>),
    Ready(Box<Decoding>),
    /// Not retried: the queue moves on to it after the current track and skips it then.
    Failed,
}

impl Successor {
    /// Starts opening `next`; the calling thread is woken once it is open or failed.
    pub(super) fn open(
        next: Next,
        source: PlaybackSource,
        replay_gain_mode: ReplayGainMode,
    ) -> Self {
        let waiting = std::thread::current();
        let item = next.item;
        let opening = std::thread::Builder::new()
            .name(String::from("next track"))
            .spawn(move || {
                let opened = Decoding::open(source, item, replay_gain_mode);
                waiting.unpark();
                opened
            });
        let state = match opening {
            Ok(thread) => State::Opening(thread),
            Err(error) => {
                log::warn!("Could not open the next track {}: {error}", next.locator);
                State::Failed
            }
        };
        Self { next, state }
    }

    pub(super) fn next(&self) -> &Next {
        &self.next
    }

    /// Whether it is still being opened; collects the result once the thread is done.
    pub(super) fn is_opening(&mut self) -> bool {
        match &self.state {
            State::Opening(thread) if !thread.is_finished() => return true,
            State::Opening(_) => {}
            State::Ready(_) | State::Failed => return false,
        }
        let State::Opening(thread) = std::mem::replace(&mut self.state, State::Failed) else {
            return false;
        };
        self.state = match thread.join() {
            Ok(Ok(track)) => State::Ready(Box::new(track)),
            Ok(Err(error)) => {
                log::warn!(
                    "Could not open the next track {}: {error}",
                    self.next.locator
                );
                State::Failed
            }
            Err(_) => {
                log::error!("Opening the next track {} panicked", self.next.locator);
                State::Failed
            }
        };
        false
    }

    /// The opened track, once it is ready.
    pub(super) fn take(&mut self) -> Option<Decoding> {
        if self.is_opening() {
            return None;
        }
        match std::mem::replace(&mut self.state, State::Failed) {
            State::Ready(track) => Some(*track),
            _ => None,
        }
    }
}
