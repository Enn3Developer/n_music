use n_event_bus::{
    job_emits, App, Ctx, Envelope, EventWriter, Handle, Job, JobControl, JobToken, Message,
    Outbox, Registrar, RunningJob, Scene, Subscriber, Tagged, UiPatch, UiThread,
};
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

struct Ping;
impl Message for Ping {}

struct Pong;
impl Message for Pong {}

struct NoopUi;
impl UiThread for NoopUi {
    fn apply(&self, patches: Vec<UiPatch>) {
        for patch in patches {
            patch();
        }
    }
}

fn test_app() -> (App, flume::Receiver<n_event_bus::Event>) {
    let (tx, rx) = flume::unbounded();
    let jobs = JobControl::new(EventWriter::new(tx));
    (App::new(jobs, Box::new(NoopUi)), rx)
}

/// A plain subscriber: proves dispatch doesn't require `Scene`.
struct Counter {
    pings: Arc<AtomicUsize>,
    pongs: Arc<AtomicUsize>,
}

impl Subscriber for Counter {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<Ping>();
        reg.on::<Pong>();
    }
}

impl Handle<Ping> for Counter {
    fn handle(&mut self, _msg: &Ping, _ctx: &Ctx, out: &mut Outbox) {
        self.pings.fetch_add(1, Ordering::SeqCst);
        // Follow-up emission: must land back on this same subscriber.
        out.emit(Pong);
    }
}

impl Handle<Pong> for Counter {
    fn handle(&mut self, _msg: &Pong, _ctx: &Ctx, _out: &mut Outbox) {
        self.pongs.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn plain_subscriber_receives_messages_and_outbox_chains() {
    let (mut app, _rx) = test_app();
    let pings = Arc::new(AtomicUsize::new(0));
    let pongs = Arc::new(AtomicUsize::new(0));
    app.register_subscriber(Counter {
        pings: pings.clone(),
        pongs: pongs.clone(),
    });

    app.enqueue(Envelope::new(Ping));
    app.dispatch_all();

    assert_eq!(pings.load(Ordering::SeqCst), 1);
    assert_eq!(pongs.load(Ordering::SeqCst), 1);
}

#[test]
fn unsubscribed_message_type_is_ignored() {
    struct Unheard;
    impl Message for Unheard {}

    let (mut app, _rx) = test_app();
    let pings = Arc::new(AtomicUsize::new(0));
    app.register_subscriber(Counter {
        pings: pings.clone(),
        pongs: Arc::new(AtomicUsize::new(0)),
    });

    app.enqueue(Envelope::new(Unheard));
    app.dispatch_all();

    assert_eq!(pings.load(Ordering::SeqCst), 0);
}

#[test]
fn tagged_open_guards_against_stale_tags() {
    let tagged = Tagged::new(7, "payload");
    assert_eq!(tagged.open(7), Some(&"payload"));
    assert_eq!(tagged.open(8), None);
}

struct SceneState {
    value: usize,
    synced: Arc<AtomicUsize>,
}

impl Subscriber for SceneState {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<Ping>();
    }
}

impl Handle<Ping> for SceneState {
    fn handle(&mut self, _msg: &Ping, _ctx: &Ctx, _out: &mut Outbox) {
        self.value += 1;
    }
}

impl Scene for SceneState {
    fn on_mount(&mut self, _ctx: &Ctx, out: &mut Outbox) {
        out.emit(Ping);
    }

    fn sync(&mut self, _ctx: &Ctx) -> Option<UiPatch> {
        let synced = self.synced.clone();
        let value = self.value;
        Some(Box::new(move || {
            synced.store(value, Ordering::SeqCst);
        }))
    }
}

#[test]
fn scene_mount_emission_dispatches_and_sync_flushes() {
    let (mut app, _rx) = test_app();
    let synced = Arc::new(AtomicUsize::new(0));
    app.register_scene(SceneState {
        value: 0,
        synced: synced.clone(),
    });

    // on_mount emitted Ping; dispatch bumps value to 1, flush applies the patch.
    app.dispatch_all();
    app.flush_ui();
    assert_eq!(synced.load(Ordering::SeqCst), 1);

    // A scene not touched by a cycle isn't synced again.
    synced.store(99, Ordering::SeqCst);
    app.dispatch_all();
    app.flush_ui();
    assert_eq!(synced.load(Ordering::SeqCst), 99);
}

struct EchoJob;

struct EchoResult(u64);

job_emits!(EchoJob => Tagged<EchoResult>);

impl Job for EchoJob {
    async fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        writer.emit_tagged(tag, EchoResult(tag));
    }
}

struct JobConsumer {
    running: Option<RunningJob>,
    accepted: Arc<AtomicUsize>,
}

impl Subscriber for JobConsumer {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        EchoJob::subscribe(reg);
    }
}

impl Handle<Tagged<EchoResult>> for JobConsumer {
    fn handle(&mut self, msg: &Tagged<EchoResult>, _ctx: &Ctx, _out: &mut Outbox) {
        if let Some(running) = &self.running {
            if running.open(msg).is_some() {
                self.accepted.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn superseded_job_results_are_dropped() {
    let (tx, rx) = flume::unbounded();
    let jobs = JobControl::new(EventWriter::new(tx.clone()));

    // Two runs of the same job: the consumer only holds on to the second one,
    // so the first job's result must be dropped by the tag guard.
    let stale = jobs.spawn_oneshot(EchoJob);
    let current = jobs.spawn_oneshot(EchoJob);
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(stale);

    let accepted = Arc::new(AtomicUsize::new(0));
    let mut app = App::new(jobs.clone(), Box::new(NoopUi));
    app.register_subscriber(JobConsumer {
        running: Some(current),
        accepted: accepted.clone(),
    });

    // Exactly two results are in flight; JobControl still holds a sender
    // clone, so draining until channel-close would hang forever.
    drop(tx);
    for _ in 0..2 {
        let event = tokio::time::timeout(Duration::from_secs(5), rx.recv_async())
            .await
            .expect("job results should arrive")
            .expect("channel alive");
        if let n_event_bus::Event::Bus(envelope) = event {
            app.enqueue(envelope);
        }
    }
    app.dispatch_all();

    // Both results were dispatched, but only the current job's tag passes.
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
}
