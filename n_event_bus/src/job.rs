use crate::event::EventWriter;
use crate::message::Tagged;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Background work spawned on tokio that reports back through the bus,
/// tagging everything it emits with `tag` so a superseded run can be ignored.
pub trait Job: Send + 'static {
    fn run(
        self,
        tag: u64,
        writer: EventWriter,
        token: Option<JobToken>,
    ) -> impl Future<Output = ()> + Send;
}

/// Cooperative cancellation flag a long-running job should check between steps.
#[derive(Clone, Default)]
pub struct JobToken {
    cancelled: Arc<AtomicBool>,
}

impl JobToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

/// Owns the spawned task; dropping it aborts the task and flips the token.
pub struct JobHandle {
    join: tokio::task::JoinHandle<()>,
    token: Option<JobToken>,
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        if let Some(token) = &self.token {
            token.cancel();
        }
        self.join.abort();
    }
}

/// A job the caller keeps around to tag-check incoming [Tagged] results against.
pub struct RunningJob {
    tag: u64,
    _handle: JobHandle,
}

impl RunningJob {
    pub fn tag(&self) -> u64 {
        self.tag
    }

    /// Returns the payload only if it comes from this job and not a superseded one.
    pub fn open<'a, P>(&self, tagged: &'a Tagged<P>) -> Option<&'a P> {
        tagged.open(self.tag)
    }
}

/// Spawns jobs and hands out unique tags; clone-cheap, lives in [crate::Ctx].
#[derive(Clone)]
pub struct JobControl {
    writer: EventWriter,
    next: Arc<AtomicU64>,
}

impl JobControl {
    pub fn new(writer: EventWriter) -> Self {
        Self {
            writer,
            next: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn writer(&self) -> &EventWriter {
        &self.writer
    }

    fn next_tag(&self) -> u64 {
        self.next.fetch_add(1, Ordering::Relaxed)
    }

    /// A cancellable job emitting a stream of results; dropping the returned
    /// [RunningJob] aborts it.
    pub fn spawn_stream<J: Job>(&self, job: J) -> RunningJob {
        let tag = self.next_tag();
        let token = JobToken::new();
        let join = tokio::spawn(job.run(tag, self.writer.clone(), Some(token.clone())));
        RunningJob {
            tag,
            _handle: JobHandle {
                join,
                token: Some(token),
            },
        }
    }

    /// A single-result job; dropping the returned [RunningJob] aborts it.
    pub fn spawn_oneshot<J: Job>(&self, job: J) -> RunningJob {
        let tag = self.next_tag();
        let join = tokio::spawn(job.run(tag, self.writer.clone(), None));
        RunningJob {
            tag,
            _handle: JobHandle { join, token: None },
        }
    }

    /// Fire-and-forget: nobody tracks or cancels it.
    pub fn spawn_detached<J: Job>(&self, job: J) {
        let tag = self.next_tag();
        tokio::spawn(job.run(tag, self.writer.clone(), None));
    }
}

/// Declares the message types a job emits, generating `J::subscribe(reg)` so a
/// subscriber registers for all of them in one call.
#[macro_export]
macro_rules! job_emits {
    ($job:ty => $($msg:ty),+ $(,)?) => {
        impl $job {
            pub fn subscribe<S>(reg: &mut $crate::registrar::Registrar<S>)
            where
                S: $crate::subscriber::Subscriber $(+ $crate::Handle<$msg>)+,
            {
                $( reg.on::<$msg>(); )+
            }
        }
    };
}
