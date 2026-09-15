use crate::bus::{Bus, SubscriberId};
use crate::event::{Event, ShutdownRequested};
use crate::job::JobControl;
use crate::message::Envelope;
use crate::subscriber::Subscriber;
use crate::{Ctx, Outbox, Registrar};
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShutdownOutcome {
    /// All participants acknowledged and their queued follow-ups were dispatched.
    Complete,
    /// The deadline expired. An empty list means only queued follow-ups remained.
    TimedOut { pending: Vec<SubscriberId> },
    /// Ingress disconnected before graceful shutdown completed.
    Disconnected,
}

struct Shutdown {
    pending: HashSet<SubscriberId>,
    started: Instant,
}

pub struct App {
    subscribers: HashMap<SubscriberId, Box<dyn Subscriber>>,
    bus: Bus,
    jobs: JobControl,
    next_id: u64,
    shutdown: Option<Shutdown>,
    outcome: Option<ShutdownOutcome>,
}

impl App {
    pub fn new(jobs: JobControl) -> Self {
        Self {
            subscribers: HashMap::new(),
            bus: Bus::new(),
            jobs,
            next_id: 0,
            shutdown: None,
            outcome: None,
        }
    }

    pub fn register_subscriber<S: Subscriber>(&mut self, subscriber: S) -> SubscriberId {
        assert!(
            self.shutdown.is_none() && self.outcome.is_none(),
            "cannot register subscribers after shutdown has started"
        );
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
        assert!(
            self.outcome.is_none(),
            "cannot enqueue after the bus stopped"
        );
        self.bus.enqueue(envelope);
    }

    pub fn enqueue_event(&mut self, event: Event) {
        self.enqueue(match event {
            Event::Bus(envelope) => envelope,
            Event::Shutdown => Envelope::new(ShutdownRequested),
        });
    }

    /// Drains the local queue, including shutdown notifications and follow-ups.
    /// Hosts using manual dispatch are responsible for receiving replies and
    /// enforcing their own deadline; `run_loop` handles both automatically.
    pub fn dispatch_all(&mut self) {
        while self.dispatch_next() {}
    }

    pub fn shutdown_outcome(&self) -> Option<&ShutdownOutcome> {
        self.outcome.as_ref()
    }

    fn dispatch_next(&mut self) -> bool {
        if self.outcome.is_some() {
            return false;
        }
        let Some(envelope) = self.bus.pop() else {
            return false;
        };
        let mut targets = self.bus.subscribers_for(envelope.tid());
        if envelope.tid() == TypeId::of::<ShutdownRequested>() {
            if self.shutdown.is_some() {
                targets.clear();
            } else {
                let mut pending = HashSet::new();
                targets.retain(|(id, _)| pending.insert(*id));
                self.shutdown = Some(Shutdown {
                    pending,
                    started: Instant::now(),
                });
            }
        }
        for (id, thunk) in targets {
            let Some(subscriber) = self.subscribers.get_mut(&id) else {
                continue;
            };
            let mut out = Outbox::new();
            thunk(
                subscriber.as_mut(),
                envelope.payload(),
                &Ctx {
                    jobs: &self.jobs,
                    shutting_down: self.shutdown.is_some(),
                },
                &mut out,
            );
            if out.shutdown_ready {
                if let Some(shutdown) = &mut self.shutdown {
                    shutdown.pending.remove(&id);
                }
            }
            for follow_up in out.into_envelopes() {
                self.bus.enqueue(follow_up);
            }
        }
        if self.bus.is_empty()
            && self
                .shutdown
                .as_ref()
                .is_some_and(|shutdown| shutdown.pending.is_empty())
        {
            self.outcome = Some(ShutdownOutcome::Complete);
        }
        true
    }

    /// Runs until subscriber shutdown completes, its deadline expires, or ingress
    /// disconnects. The timeout starts at the first shutdown notification, not at
    /// application startup, and repeated requests do not extend it.
    pub fn run_loop(
        &mut self,
        rx: flume::Receiver<Event>,
        shutdown_timeout: Duration,
    ) -> ShutdownOutcome {
        const BATCH: usize = 64;
        let mut disconnected = false;

        while self.outcome.is_none() {
            if let Some(shutdown) = &self.shutdown {
                if shutdown.started.elapsed() >= shutdown_timeout {
                    let mut pending: Vec<_> = shutdown.pending.iter().copied().collect();
                    pending.sort_unstable_by_key(|id| id.0);
                    self.outcome = Some(ShutdownOutcome::TimedOut { pending });
                    break;
                }
            }

            // Bound both phases so ingress floods and follow-up chains cannot
            // prevent shutdown requests, replies, or deadline checks from running.
            if !self
                .shutdown
                .as_ref()
                .is_some_and(|shutdown| shutdown.pending.is_empty())
            {
                for _ in 0..BATCH {
                    match rx.try_recv() {
                        Ok(event) => self.enqueue_event(event),
                        Err(flume::TryRecvError::Empty) => break,
                        Err(flume::TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                    }
                }
            }
            for _ in 0..BATCH {
                if !self.dispatch_next()
                    || self
                        .shutdown
                        .as_ref()
                        .is_some_and(|shutdown| shutdown.started.elapsed() >= shutdown_timeout)
                {
                    break;
                }
            }
            if self.outcome.is_some() {
                break;
            }
            if !self.bus.is_empty() {
                continue;
            }
            if disconnected {
                self.outcome = Some(ShutdownOutcome::Disconnected);
                break;
            }

            let received = if let Some(shutdown) = &self.shutdown {
                rx.recv_timeout(shutdown_timeout.saturating_sub(shutdown.started.elapsed()))
            } else {
                rx.recv().map_err(|_| flume::RecvTimeoutError::Disconnected)
            };
            match received {
                Ok(event) => self.enqueue_event(event),
                Err(flume::RecvTimeoutError::Timeout) => continue,
                Err(flume::RecvTimeoutError::Disconnected) => {
                    self.outcome = Some(ShutdownOutcome::Disconnected);
                }
            }
        }
        self.outcome.clone().unwrap()
    }
}
