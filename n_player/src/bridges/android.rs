use crate::messages::{PlaybackChanged, PositionChanged, TrackChanged};
use crate::services::metadata::{MetadataJob, MetadataLoaded, MetadataLoader, TrackMetadata};
use n_event_bus::{
    Ctx, EventWriter, Handle, Job, JobToken, Outbox, Registrar, RunningJob, Subscriber, Tagged,
};
use std::any::Any;
use std::sync::Arc;

pub struct AndroidBridge {
    jvm: Arc<jni::JavaVM>,
    callback: Arc<jni::objects::GlobalRef>,
    notification: Option<RunningJob>,
    pending: Option<TrackMetadata>,
    metadata_loader: MetadataLoader,
    position: f64,
    playing: bool,
}

impl AndroidBridge {
    pub fn new(jvm: Arc<jni::JavaVM>, callback: Arc<jni::objects::GlobalRef>) -> Self {
        let mut env = jvm.attach_current_thread().unwrap();
        env.call_method(callback.as_ref(), "createNotification", "()V", &[])
            .unwrap();
        drop(env);
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
        MetadataJob::subscribe(reg);
        NotificationJob::subscribe(reg);
        reg.on::<PositionChanged>();
    }
}

impl AndroidBridge {
    fn update_playback(&self) {
        let mut env = self.jvm.attach_current_thread().unwrap();
        env.call_method(
            self.callback.as_ref(),
            "changePlaybackState",
            "(ZD)V",
            &[self.playing.into(), self.position.into()],
        )
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
    callback: Arc<jni::objects::GlobalRef>,
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
            let mut env = self.jvm.attach_current_thread()?;
            let title = env.new_string(meta.title)?;
            let artist = env.new_string(meta.artist)?;
            let cover_path = env.new_string(cover_path)?;
            env.call_method(
                self.callback.as_ref(),
                "changeNotification",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;D)V",
                &[
                    (&title).into(),
                    (&artist).into(),
                    (&cover_path).into(),
                    meta.time.length.into(),
                ],
            )?;
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
