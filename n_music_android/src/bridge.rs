use n_event_bus::{
    Ctx, EventWriter, Handle, Job, JobToken, Outbox, Registrar, RunningJob, ShutdownRequested,
    Subscriber, Tagged,
};
use n_music_core::library::LibraryPaths;
use n_music_core::messages::{
    LoopStatusChanged, PlaybackChanged, PositionChanged, QueueChanged, ShuffleChanged,
    ThemeChangeRequested, TrackChanged,
};
use n_music_core::queue::{ItemId, LoopStatus};
use n_music_core::services::metadata::{MetadataJob, MetadataLoaded, MetadataLoader};
use n_music_core::source::{Locator, Providers};
use n_music_core::Track;
use std::any::Any;
use std::sync::{Arc, Mutex};

/// The items of the Media3 playlist, in its order, so a seek to one of its indexes finds the
/// item even while the bus is busy.
static QUEUE: Mutex<Vec<ItemId>> = Mutex::new(Vec::new());

/// The item at `index` of the Media3 playlist.
pub fn queue_item(index: usize) -> Option<ItemId> {
    QUEUE.lock().unwrap().get(index).copied()
}

fn queue_index(item: ItemId) -> Option<usize> {
    QUEUE
        .lock()
        .unwrap()
        .iter()
        .position(|&queued| queued == item)
}

// androidx.media3.common.Player REPEAT_MODE_OFF / ONE / ALL
const REPEAT_MODE_OFF: i32 = 0;
const REPEAT_MODE_ONE: i32 = 1;
const REPEAT_MODE_ALL: i32 = 2;

pub struct AndroidBridge {
    jvm: Arc<jni::JavaVM>,
    callback: Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
    notification: Option<RunningJob>,
    pending: Option<Track>,
    metadata_loader: MetadataLoader,
    position: f64,
    playing: bool,
    current: Option<ItemId>,
}

impl AndroidBridge {
    pub fn new(
        jvm: Arc<jni::JavaVM>,
        callback: Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
        theme: i32,
        providers: Arc<Providers>,
        paths: LibraryPaths,
    ) -> Self {
        jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
            env.call_method(
                callback.as_ref(),
                jni::jni_str!("createNotification"),
                jni::jni_sig!("()V"),
                &[],
            )
            .inspect_err(|error| {
                crate::platform::log_jni_error(env, "MainActivity.createNotification", error)
            })?;
            Ok(())
        })
        .expect("JNI call MainActivity.createNotification failed");
        let bridge = Self {
            jvm,
            callback,
            notification: None,
            pending: None,
            metadata_loader: MetadataLoader::new(providers, paths),
            position: 0.0,
            playing: false,
            current: None,
        };
        bridge.change_theme(theme);
        bridge
    }
}

impl Subscriber for AndroidBridge {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<PlaybackChanged>();
        reg.on::<TrackChanged>();
        reg.on::<QueueChanged>();
        reg.on::<LoopStatusChanged>();
        reg.on::<ShuffleChanged>();
        reg.on::<ThemeChangeRequested>();
        MetadataJob::subscribe(reg);
        NotificationJob::subscribe(reg);
        reg.on::<PositionChanged>();
        reg.on::<ShutdownRequested>();
    }
}

impl AndroidBridge {
    fn update_playback(&self) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("changePlaybackState"),
                    jni::jni_sig!("(ZD)V"),
                    &[self.playing.into(), self.position.into()],
                )
                .inspect_err(|error| {
                    crate::platform::log_jni_error(env, "MainActivity.changePlaybackState", error)
                })?;
                Ok(())
            })
            .expect("JNI call MainActivity.changePlaybackState failed");
    }

    fn change_repeat_mode(&self, mode: i32) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("changeRepeatMode"),
                    jni::jni_sig!("(I)V"),
                    &[mode.into()],
                )
                .inspect_err(|error| {
                    crate::platform::log_jni_error(env, "MainActivity.changeRepeatMode", error)
                })?;
                Ok(())
            })
            .expect("JNI call MainActivity.changeRepeatMode failed");
    }

    fn change_shuffle_mode(&self, shuffle: bool) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("changeShuffleMode"),
                    jni::jni_sig!("(Z)V"),
                    &[shuffle.into()],
                )
                .inspect_err(|error| {
                    crate::platform::log_jni_error(env, "MainActivity.changeShuffleMode", error)
                })?;
                Ok(())
            })
            .expect("JNI call MainActivity.changeShuffleMode failed");
    }

    fn change_theme(&self, theme: i32) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("set_theme"),
                    jni::jni_sig!("(I)V"),
                    &[theme.into()],
                )
                .inspect_err(|error| {
                    crate::platform::log_jni_error(env, "MainActivity.set_theme", error)
                })?;
                Ok(())
            })
            .expect("JNI call MainActivity.set_theme failed");
    }

    fn change_queue(&self, tracks: &[&Locator], current: usize) {
        // Locators only serve as media IDs. U+001F (unit separator) cannot appear in a path,
        // so it is a safe delimiter.
        let joined = tracks
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\u{1f}");
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                let string = env.new_string(joined)?;
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("changeQueue"),
                    jni::jni_sig!("(Ljava/lang/String;I)V"),
                    &[(&string).into(), (current as i32).into()],
                )
                .inspect_err(|error| {
                    crate::platform::log_jni_error(env, "MainActivity.changeQueue", error)
                })?;
                Ok(())
            })
            .expect("JNI call MainActivity.changeQueue failed");
    }

    fn change_track(&self, index: usize) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("changeTrack"),
                    jni::jni_sig!("(I)V"),
                    &[(index as i32).into()],
                )
                .inspect_err(|error| {
                    crate::platform::log_jni_error(env, "MainActivity.changeTrack", error)
                })?;
                Ok(())
            })
            .expect("JNI call MainActivity.changeTrack failed");
    }
}
impl Handle<PlaybackChanged> for AndroidBridge {
    fn handle(&mut self, msg: &PlaybackChanged, _: &Ctx, _: &mut Outbox) {
        self.playing = msg.0;
        self.update_playback();
    }
}
impl Handle<PositionChanged> for AndroidBridge {
    fn handle(&mut self, msg: &PositionChanged, _: &Ctx, _: &mut Outbox) {
        self.position = msg.0.position;
        if msg.2 {
            self.update_playback();
        }
    }
}

impl Handle<QueueChanged> for AndroidBridge {
    fn handle(&mut self, msg: &QueueChanged, _: &Ctx, _: &mut Outbox) {
        *QUEUE.lock().unwrap() = msg.entries.iter().map(|entry| entry.item).collect();
        let current = self.current.and_then(queue_index).unwrap_or(0);
        let locators: Vec<&Locator> = msg.entries.iter().map(|entry| &entry.locator).collect();
        self.change_queue(&locators, current);
    }
}

impl Handle<LoopStatusChanged> for AndroidBridge {
    fn handle(&mut self, msg: &LoopStatusChanged, _: &Ctx, _: &mut Outbox) {
        let mode = match msg.0 {
            LoopStatus::Off => REPEAT_MODE_OFF,
            LoopStatus::Playlist => REPEAT_MODE_ALL,
            LoopStatus::File => REPEAT_MODE_ONE,
        };
        self.change_repeat_mode(mode);
    }
}

impl Handle<ShuffleChanged> for AndroidBridge {
    fn handle(&mut self, msg: &ShuffleChanged, _: &Ctx, _: &mut Outbox) {
        self.change_shuffle_mode(msg.0);
    }
}

impl Handle<ThemeChangeRequested> for AndroidBridge {
    fn handle(&mut self, msg: &ThemeChangeRequested, ctx: &Ctx, _: &mut Outbox) {
        if !ctx.shutting_down {
            self.change_theme(msg.0);
        }
    }
}

impl Handle<TrackChanged> for AndroidBridge {
    fn handle(&mut self, msg: &TrackChanged, ctx: &Ctx, _out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        self.current = Some(msg.item);
        self.change_track(queue_index(msg.item).unwrap_or(0));
        self.metadata_loader.load(msg.locator.clone(), ctx);
    }
}

impl AndroidBridge {
    fn flush_notification(&mut self, ctx: &Ctx) {
        if self.notification.is_some() {
            return;
        }
        if let Some(track) = self.pending.take() {
            self.notification = Some(ctx.jobs.spawn_oneshot(NotificationJob {
                jvm: self.jvm.clone(),
                callback: self.callback.clone(),
                track,
            }));
        }
    }
}
impl Handle<Tagged<MetadataLoaded>> for AndroidBridge {
    fn handle(&mut self, msg: &Tagged<MetadataLoaded>, ctx: &Ctx, _: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        if let Some(track) = self.metadata_loader.take(msg) {
            self.pending = Some(track);
            self.flush_notification(ctx);
        }
    }
}
struct NotificationJob {
    jvm: Arc<jni::JavaVM>,
    callback: Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
    track: Track,
}
struct NotificationFinished;
n_event_bus::job_emits!(NotificationJob => Tagged<NotificationFinished>);
impl Job for NotificationJob {
    fn run(self, tag: u64, writer: EventWriter, _: Option<JobToken>) {
        let result = (|| -> jni::errors::Result<()> {
            let track = &self.track;
            let cover_path = track
                .cover
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.jvm
                .attach_current_thread(|env| -> jni::errors::Result<()> {
                    let title = env.new_string(&track.title)?;
                    let artist = env.new_string(track.artist())?;
                    let cover_path = env.new_string(cover_path)?;
                    env.call_method(
                        self.callback.as_ref(),
                        jni::jni_str!("changeNotification"),
                        jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;D)V"),
                        &[
                            (&title).into(),
                            (&artist).into(),
                            (&cover_path).into(),
                            track.length.into(),
                        ],
                    )
                    .inspect_err(|error| {
                        crate::platform::log_jni_error(
                            env,
                            "MainActivity.changeNotification",
                            error,
                        )
                    })?;
                    Ok(())
                })?;
            Ok(())
        })();
        if let Err(error) = result {
            log::warn!("Could not update Android notification: {error:?}");
        }
        writer.emit_tagged(tag, NotificationFinished);
    }
}
impl Handle<Tagged<NotificationFinished>> for AndroidBridge {
    fn handle(&mut self, msg: &Tagged<NotificationFinished>, ctx: &Ctx, out: &mut Outbox) {
        if self
            .notification
            .as_ref()
            .and_then(|job| job.open(msg))
            .is_none()
        {
            return;
        }
        self.notification = None;
        if ctx.shutting_down {
            out.shutdown_ready();
        } else {
            self.flush_notification(ctx);
        }
    }
}

impl Handle<ShutdownRequested> for AndroidBridge {
    fn handle(&mut self, _: &ShutdownRequested, _: &Ctx, out: &mut Outbox) {
        self.pending = None;
        self.metadata_loader.cancel();
        self.playing = false;
        self.update_playback();
        if self.notification.is_none() {
            out.shutdown_ready();
        }
    }
}
