//! Discord Rich Presence: the track playing, its cover and its progress on the user's profile.
//!
//! The crate is a bus subscriber, so hosts only have to register it:
//!
//! ```no_run
//! # fn example(app: &mut n_event_bus::App) {
//! app.register_subscriber(n_music_discord::DiscordPresence::new(true));
//! # }
//! ```

mod cover;
mod presence;

use crate::presence::Command;
use n_event_bus::{Ctx, Handle, Message, Outbox, Registrar, ShutdownRequested, Subscriber};
use n_music_core::messages::{PlaybackChanged, PositionChanged, TrackChanged};
use std::any::Any;
use std::thread::JoinHandle;

/// Turns the presence on or off.
pub struct SetDiscordPresence(pub bool);

impl Message for SetDiscordPresence {}

/// Bridges the event bus to Discord's local client.
pub struct DiscordPresence {
    commands: flume::Sender<Command>,
    thread: Option<JoinHandle<()>>,
}

impl DiscordPresence {
    /// Talks to Discord from a thread of its own: its IPC and the cover lookups block.
    pub fn new(enabled: bool) -> Self {
        let (commands, receiver) = flume::unbounded();
        let thread = std::thread::Builder::new()
            .name(String::from("n_music_discord"))
            .spawn(move || presence::run(receiver, enabled))
            .inspect_err(|error| log::error!("Could not start Discord Rich Presence: {error}"))
            .ok();
        Self { commands, thread }
    }

    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        self.send(Command::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Subscriber for DiscordPresence {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<TrackChanged>();
        reg.on::<PlaybackChanged>();
        reg.on::<PositionChanged>();
        reg.on::<SetDiscordPresence>();
        reg.on::<ShutdownRequested>();
    }
}

impl Handle<TrackChanged> for DiscordPresence {
    fn handle(&mut self, msg: &TrackChanged, ctx: &Ctx, _out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let track = &msg.track;
        self.send(Command::Track {
            title: track.title.clone(),
            artist: track.artist(),
            album: track.album.clone(),
            length: track.length,
        });
    }
}

impl Handle<PlaybackChanged> for DiscordPresence {
    fn handle(&mut self, msg: &PlaybackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        self.send(Command::Playing(msg.0));
    }
}

impl Handle<PositionChanged> for DiscordPresence {
    fn handle(&mut self, msg: &PositionChanged, _ctx: &Ctx, _out: &mut Outbox) {
        // Discord counts the time itself: it only needs to hear when the position jumps.
        if msg.discontinuity {
            self.send(Command::Position {
                position: msg.time.position,
                length: msg.time.length,
            });
        }
    }
}

impl Handle<SetDiscordPresence> for DiscordPresence {
    fn handle(&mut self, msg: &SetDiscordPresence, _ctx: &Ctx, _out: &mut Outbox) {
        self.send(Command::Enabled(msg.0));
    }
}

impl Handle<ShutdownRequested> for DiscordPresence {
    fn handle(&mut self, _: &ShutdownRequested, _: &Ctx, out: &mut Outbox) {
        self.send(Command::Quit);
        out.shutdown_ready();
    }
}
