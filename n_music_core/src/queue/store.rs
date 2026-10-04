//! Saves the session on its own thread, so the event loop never waits on the disk. Only the
//! latest of each kind of write matters: older pending ones are skipped.

use crate::library::user_data::{SessionItems, SessionState};
use crate::library::LibraryPaths;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;

enum Write {
    Items(SessionItems),
    State(SessionState),
}

pub(super) struct SessionStore {
    tx: Option<Sender<Write>>,
    thread: Option<JoinHandle<()>>,
}

impl SessionStore {
    pub fn new(paths: LibraryPaths) -> Self {
        let (tx, rx) = channel();
        let thread = std::thread::Builder::new()
            .name(String::from("session store"))
            .spawn(move || run(paths, rx))
            .inspect_err(|error| log::error!("Could not start saving the session: {error}"))
            .ok();
        Self {
            tx: thread.is_some().then_some(tx),
            thread,
        }
    }

    pub fn save_items(&self, items: SessionItems) {
        self.send(Write::Items(items));
    }

    pub fn save_state(&self, state: SessionState) {
        self.send(Write::State(state));
    }

    fn send(&self, write: Write) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(write);
        }
    }

    /// Waits until everything sent is saved.
    pub fn close(&mut self) {
        self.tx = None;
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                log::error!("The session store panicked");
            }
        }
    }
}

impl Drop for SessionStore {
    fn drop(&mut self) {
        self.close();
    }
}

fn run(paths: LibraryPaths, rx: Receiver<Write>) {
    let mut db = match paths.open_db() {
        Ok(db) => db,
        Err(error) => {
            log::error!("Could not open the library database to save the session: {error}");
            return;
        }
    };
    while let Ok(write) = rx.recv() {
        let mut items = None;
        let mut state = None;
        for write in std::iter::once(write).chain(rx.try_iter()) {
            match write {
                Write::Items(latest) => items = Some(latest),
                Write::State(latest) => state = Some(latest),
            }
        }
        if let Some(items) = items {
            if let Err(error) = db.save_session_items(&items) {
                log::error!("Could not save the queue: {error}");
            }
        }
        if let Some(state) = state {
            if let Err(error) = db.save_session_state(&state) {
                log::error!("Could not save the playback position: {error}");
            }
        }
    }
}
