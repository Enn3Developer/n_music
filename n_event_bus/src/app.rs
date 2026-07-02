use crate::bus::{Bus, SubscriberId};
use crate::event::{Event, Tick};
use crate::job::JobControl;
use crate::message::Envelope;
use crate::scene::Scene;
use crate::subscriber::Subscriber;
use crate::ui::{UiPatch, UiThread};
use crate::{Ctx, Outbox, Registrar};
use std::collections::{HashMap, HashSet};
use std::mem;

type SyncFn = fn(&mut dyn Subscriber, &Ctx) -> Option<UiPatch>;

fn sync_thunk<S: Scene>(subscriber: &mut dyn Subscriber, ctx: &Ctx) -> Option<UiPatch> {
    subscriber
        .as_any_mut()
        .downcast_mut::<S>()
        .and_then(|scene| scene.sync(ctx))
}

/// Owns all subscribers and the bus; drains the queue per incoming event and
/// flushes UI patches for the scenes touched in that cycle.
pub struct App {
    subscribers: HashMap<SubscriberId, Box<dyn Subscriber>>,
    scene_meta: HashMap<SubscriberId, SyncFn>,
    bus: Bus,
    jobs: JobControl,
    ui: Box<dyn UiThread>,
    touched: HashSet<SubscriberId>,
    next_id: u64,
}

impl App {
    pub fn new(jobs: JobControl, ui: Box<dyn UiThread>) -> Self {
        Self {
            subscribers: HashMap::new(),
            scene_meta: HashMap::new(),
            bus: Bus::new(),
            jobs,
            ui,
            touched: HashSet::new(),
            next_id: 0,
        }
    }

    /// Generic entry point: any [Subscriber] can receive messages, no UI involved.
    pub fn register_subscriber<S: Subscriber>(&mut self, subscriber: S) -> SubscriberId {
        let id = SubscriberId(self.next_id);
        self.next_id += 1;

        let mut registrar = Registrar::<S>::new();
        S::register(&mut registrar);
        for (tid, thunk) in registrar.into_pairs() {
            self.bus.subscribe(tid, id, thunk);
        }
        self.subscribers.insert(id, Box::new(subscriber));
        id
    }

    /// [register_subscriber](Self::register_subscriber) plus mounting and a UI-sync pass.
    pub fn register_scene<S: Scene>(&mut self, mut scene: S) -> SubscriberId {
        let mut out = Outbox::new();
        scene.on_mount(&Ctx { jobs: &self.jobs }, &mut out);
        for envelope in out.into_envelopes() {
            self.bus.enqueue(envelope);
        }

        let id = self.register_subscriber(scene);
        self.scene_meta.insert(id, sync_thunk::<S>);
        self.touched.insert(id);
        id
    }

    pub fn enqueue(&mut self, envelope: Envelope) {
        self.bus.enqueue(envelope);
    }

    /// Drains the queue, dispatching each envelope to its subscribers and
    /// enqueueing whatever their outboxes emit, until nothing is pending.
    pub fn dispatch_all(&mut self) {
        while let Some(envelope) = self.bus.pop() {
            for (id, thunk) in self.bus.subscribers_for(envelope.tid()) {
                let Some(subscriber) = self.subscribers.get_mut(&id) else {
                    continue;
                };
                let mut out = Outbox::new();
                thunk(
                    subscriber.as_mut(),
                    envelope.payload(),
                    &Ctx { jobs: &self.jobs },
                    &mut out,
                );
                self.touched.insert(id);
                for follow_up in out.into_envelopes() {
                    self.bus.enqueue(follow_up);
                }
            }
        }
    }

    /// Syncs the scenes touched since the last flush and applies their patches
    /// in one UI-thread hop.
    pub fn flush_ui(&mut self) {
        let touched = mem::take(&mut self.touched);
        let mut patches = Vec::new();
        for id in touched {
            let Some(sync) = self.scene_meta.get(&id) else {
                continue;
            };
            let Some(subscriber) = self.subscribers.get_mut(&id) else {
                continue;
            };
            if let Some(patch) = sync(subscriber.as_mut(), &Ctx { jobs: &self.jobs }) {
                patches.push(patch);
            }
        }
        if !patches.is_empty() {
            self.ui.apply(patches);
        }
    }

    fn enqueue_event(&mut self, event: Event) {
        match event {
            Event::Bus(envelope) => self.bus.enqueue(envelope),
            Event::Tick => self.bus.enqueue(Envelope::new(Tick)),
        }
    }

    /// Async replacement for a blocking main loop: consumes events until the
    /// sending side is dropped.
    pub async fn run_loop(&mut self, rx: flume::Receiver<Event>) {
        // Dispatch whatever on_mount emitted and sync the initial UI state.
        self.dispatch_all();
        self.flush_ui();

        while let Ok(event) = rx.recv_async().await {
            self.enqueue_event(event);
            // Fold everything already pending into this cycle: one dispatch
            // and one UI hop per burst, not per message — otherwise a scan
            // streaming per-track metadata queues a UI patch per track.
            while let Ok(event) = rx.try_recv() {
                self.enqueue_event(event);
            }
            self.dispatch_all();
            self.flush_ui();
        }
    }
}
