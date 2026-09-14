use n_music_core::queue::LoopStatus;
use n_music_core::messages::{
    LoopStatusChanged, PlaybackChanged, PositionChanged, QueueReplaced, TrackChanged,
};
use n_music_core::services::metadata::{MetadataJob, MetadataLoaded, MetadataLoader, TrackMetadata};
use n_event_bus::{
    Ctx, EventWriter, Handle, Job, JobToken, Outbox, Registrar, RunningJob, Subscriber, Tagged,
};
use std::any::Any;
use std::sync::Arc;

// androidx.media3.common.Player REPEAT_MODE_OFF / ONE / ALL
const REPEAT_MODE_ONE: i32 = 1;
const REPEAT_MODE_ALL: i32 = 2;

pub struct AndroidBridge {
    jvm: Arc<jni::JavaVM>,
    callback: Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
    notification: Option<RunningJob>,
    pending: Option<TrackMetadata>,
    metadata_loader: MetadataLoader,
    position: f64,
    playing: bool,
}

impl AndroidBridge {
    pub fn new(
        jvm: Arc<jni::JavaVM>,
        callback: Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
    ) -> Self {
        jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
            env.call_method(
                callback.as_ref(),
                jni::jni_str!("createNotification"),
                jni::jni_sig!("()V"),
                &[],
            )?;
            Ok(())
        })
        .unwrap();
        Self {
            jvm,
            callback,
            notification: None,
            pending: None,
            metadata_loader: MetadataLoader::default(),
            position: 0.0,
            playing: false,
        }
    }
}

impl Subscriber for AndroidBridge {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<PlaybackChanged>();
        reg.on::<TrackChanged>();
        reg.on::<QueueReplaced>();
        reg.on::<LoopStatusChanged>();
        MetadataJob::subscribe(reg);
        NotificationJob::subscribe(reg);
        reg.on::<PositionChanged>();
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
                )?;
                Ok(())
            })
            .unwrap();
    }

    fn change_repeat_mode(&self, mode: i32) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("changeRepeatMode"),
                    jni::jni_sig!("(I)V"),
                    &[mode.into()],
                )?;
                Ok(())
            })
            .unwrap();
    }

    fn change_queue(&self, names: &[String]) {
        // U+001F (unit separator) cannot appear in a path, so it is a safe delimiter.
        let joined = names.join("\u{1f}");
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                let string = env.new_string(joined)?;
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("changeQueue"),
                    jni::jni_sig!("(Ljava/lang/String;)V"),
                    &[(&string).into()],
                )?;
                Ok(())
            })
            .unwrap();
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

impl Handle<QueueReplaced> for AndroidBridge {
    fn handle(&mut self, msg: &QueueReplaced, _: &Ctx, _: &mut Outbox) {
        self.change_queue(&msg.names);
    }
}

impl Handle<LoopStatusChanged> for AndroidBridge {
    fn handle(&mut self, msg: &LoopStatusChanged, _: &Ctx, _: &mut Outbox) {
        let mode = match msg.0 {
            LoopStatus::Playlist => REPEAT_MODE_ALL,
            LoopStatus::File => REPEAT_MODE_ONE,
        };
        self.change_repeat_mode(mode);
    }
}

impl Handle<TrackChanged> for AndroidBridge {
    fn handle(&mut self, msg: &TrackChanged, ctx: &Ctx, _out: &mut Outbox) {
        self.metadata_loader.load(msg.path.clone(), ctx);
    }
}

impl AndroidBridge {
    fn flush_notification(&mut self, ctx: &Ctx) {
        if self.notification.is_some() {
            return;
        }
        if let Some(metadata) = self.pending.take() {
            self.notification = Some(ctx.jobs.spawn_oneshot(NotificationJob {
                jvm: self.jvm.clone(),
                callback: self.callback.clone(),
                metadata,
            }));
        }
    }
}
impl Handle<Tagged<MetadataLoaded>> for AndroidBridge {
    fn handle(&mut self, msg: &Tagged<MetadataLoaded>, ctx: &Ctx, _: &mut Outbox) {
        if let Some(metadata) = self.metadata_loader.take(msg) {
            self.pending = Some(metadata);
            self.flush_notification(ctx);
        }
    }
}
struct NotificationJob {
    jvm: Arc<jni::JavaVM>,
    callback: Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
    metadata: TrackMetadata,
}
struct NotificationFinished;
n_event_bus::job_emits!(NotificationJob => Tagged<NotificationFinished>);
impl Job for NotificationJob {
    async fn run(self, tag: u64, writer: EventWriter, _: Option<JobToken>) {
        let result = tokio::task::spawn_blocking(move || -> jni::errors::Result<()> {
            let meta = self.metadata.metadata;
            let cover_path = self
                .metadata
                .cover
                .as_ref()
                .map(|file| file.path().to_string_lossy().into_owned())
                .unwrap_or_default();
            self.jvm
                .attach_current_thread(|env| -> jni::errors::Result<()> {
                    let title = env.new_string(meta.title)?;
                    let artist = env.new_string(meta.artist)?;
                    let cover_path = env.new_string(cover_path)?;
                    env.call_method(
                        self.callback.as_ref(),
                        jni::jni_str!("changeNotification"),
                        jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;D)V"),
                        &[
                            (&title).into(),
                            (&artist).into(),
                            (&cover_path).into(),
                            meta.time.length.into(),
                        ],
                    )?;
                    Ok(())
                })?;
            Ok(())
        })
        .await;
        if !matches!(result, Ok(Ok(()))) {
            eprintln!("Could not update Android notification: {result:?}");
        }
        writer.emit_tagged(tag, NotificationFinished);
    }
}
impl Handle<Tagged<NotificationFinished>> for AndroidBridge {
    fn handle(&mut self, msg: &Tagged<NotificationFinished>, ctx: &Ctx, _: &mut Outbox) {
        if self
            .notification
            .as_ref()
            .and_then(|job| job.open(msg))
            .is_none()
        {
            return;
        }
        self.notification = None;
        self.flush_notification(ctx);
    }
}
