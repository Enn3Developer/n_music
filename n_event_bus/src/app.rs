use crate::bus::{Bus, SubscriberId};
use crate::event::Event;
use crate::job::JobControl;
use crate::message::Envelope;
use crate::subscriber::Subscriber;
use crate::{Ctx, Outbox, Registrar};
use std::collections::HashMap;

pub struct App {
    subscribers: HashMap<SubscriberId, Box<dyn Subscriber>>,
    bus: Bus,
    jobs: JobControl,
    next_id: u64,
}

impl App {
    pub fn new(jobs: JobControl) -> Self {
        Self {
            subscribers: HashMap::new(),
            bus: Bus::new(),
            jobs,
            next_id: 0,
        }
    }

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

    pub fn enqueue(&mut self, envelope: Envelope) {
        self.bus.enqueue(envelope);
    }

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
                for follow_up in out.into_envelopes() {
                    self.bus.enqueue(follow_up);
                }
            }
        }
    }

    fn enqueue_event(&mut self, event: Event) -> bool {
        match event {
            Event::Bus(envelope) => self.bus.enqueue(envelope),
            Event::Shutdown => return true,
        }
        false
    }

    pub fn run_loop(&mut self, rx: flume::Receiver<Event>) {
        self.dispatch_all();

        while let Ok(event) = rx.recv() {
            let mut shutdown = self.enqueue_event(event);
            while let Ok(event) = rx.try_recv() {
                shutdown |= self.enqueue_event(event);
            }
            self.dispatch_all();
            if shutdown {
                break;
            }
        }
    }
}
