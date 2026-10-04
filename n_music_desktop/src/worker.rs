//! Runs library queries off the Qt thread, so a large library never stalls the interface.
//!
//! Each view has one slot: a newer job replaces the one waiting, keeping its earlier due time,
//! so refreshes delayed while a scan streams metadata still run regularly.

use crate::hub::hub;
use n_music_core::library::catalog::Catalog;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub type Job = Box<dyn FnOnce(&Catalog) + Send>;

struct Pending {
    due: Instant,
    job: Job,
}

#[derive(Default)]
struct Worker {
    jobs: Mutex<HashMap<u64, Pending>>,
    wake: Condvar,
}

static WORKER: OnceLock<&'static Worker> = OnceLock::new();

fn worker() -> &'static Worker {
    WORKER.get_or_init(|| {
        let worker: &'static Worker = Box::leak(Box::default());
        std::thread::Builder::new()
            .name(String::from("library queries"))
            .spawn(|| worker.run())
            .expect("failed to spawn the query thread");
        worker
    })
}

/// A new slot for a view's jobs.
pub fn slot() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Runs `job` with the catalog after `delay`, replacing what waits in `slot`.
pub fn submit(slot: u64, delay: Duration, job: Job) {
    let worker = worker();
    let due = Instant::now() + delay;
    let mut jobs = worker.jobs.lock().unwrap();
    let due = jobs.get(&slot).map_or(due, |pending| pending.due.min(due));
    jobs.insert(slot, Pending { due, job });
    worker.wake.notify_one();
}

impl Worker {
    fn run(&self) {
        loop {
            let job = self.next();
            job(&hub().library().read());
        }
    }

    /// Waits for the job due first.
    fn next(&self) -> Job {
        let mut jobs = self.jobs.lock().unwrap();
        loop {
            let first = jobs
                .iter()
                .min_by_key(|(_, pending)| pending.due)
                .map(|(&slot, pending)| (slot, pending.due));
            match first {
                Some((slot, due)) => {
                    let now = Instant::now();
                    if due <= now {
                        return jobs.remove(&slot).unwrap().job;
                    }
                    jobs = self.wake.wait_timeout(jobs, due - now).unwrap().0;
                }
                None => jobs = self.wake.wait(jobs).unwrap(),
            }
        }
    }
}

/// How long a refresh waits while metadata streams in, so it runs about once a second.
pub const STREAMING: Duration = Duration::from_secs(1);
