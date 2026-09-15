use crate::event::EventWriter;
use crate::message::Tagged;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

pub trait Job: Send + 'static {
    fn run(self, tag: u64, writer: EventWriter, token: Option<JobToken>);
}

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

pub struct JobHandle {
    _join: std::thread::JoinHandle<()>,
    token: Option<JobToken>,
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        // Threads cannot be aborted: cancellation is cooperative through the
        // token, and the detached thread exits when it observes the flag.
        if let Some(token) = &self.token {
            token.cancel();
        }
    }
}

pub struct RunningJob {
    tag: u64,
    _handle: JobHandle,
}

impl RunningJob {
    pub fn tag(&self) -> u64 {
        self.tag
    }

    pub fn open<'a, P>(&self, tagged: &'a Tagged<P>) -> Option<&'a P> {
        tagged.open(self.tag)
    }
}

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

    fn spawn_thread<J: Job>(&self, job: J, tag: u64, token: Option<JobToken>) -> JobHandle {
        let writer = self.writer.clone();
        let thread_token = token.clone();
        let job_type = std::any::type_name::<J>();
        let join = std::thread::Builder::new()
            .name(format!("{job_type} ({tag})"))
            .spawn(move || job.run(tag, writer, thread_token))
            .unwrap_or_else(|error| panic!("Failed to spawn {job_type} job {tag}: {error}"));
        JobHandle { _join: join, token }
    }

    pub fn spawn_stream<J: Job>(&self, job: J) -> RunningJob {
        let tag = self.next_tag();
        let token = JobToken::new();
        let _handle = self.spawn_thread(job, tag, Some(token.clone()));
        RunningJob { tag, _handle }
    }

    pub fn spawn_oneshot<J: Job>(&self, job: J) -> RunningJob {
        let tag = self.next_tag();
        RunningJob {
            tag,
            _handle: self.spawn_thread(job, tag, None),
        }
    }

    pub fn spawn_detached<J: Job>(&self, job: J) {
        let tag = self.next_tag();
        drop(self.spawn_thread(job, tag, None));
    }
}

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
