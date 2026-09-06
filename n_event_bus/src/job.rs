use crate::event::EventWriter;
use crate::message::Tagged;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

pub trait Job: Send + 'static {
    fn run(
        self,
        tag: u64,
        writer: EventWriter,
        token: Option<JobToken>,
    ) -> impl Future<Output = ()> + Send;
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

    pub fn spawn_oneshot<J: Job>(&self, job: J) -> RunningJob {
        let tag = self.next_tag();
        let join = tokio::spawn(job.run(tag, self.writer.clone(), None));
        RunningJob {
            tag,
            _handle: JobHandle { join, token: None },
        }
    }

    pub fn spawn_detached<J: Job>(&self, job: J) {
        let tag = self.next_tag();
        tokio::spawn(job.run(tag, self.writer.clone(), None));
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
