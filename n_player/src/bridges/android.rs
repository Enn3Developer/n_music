use crate::messages::{
    Play, PlayNext, PlayPrevious, PlaybackChanged, PositionChanged, Seek, TogglePause, TrackChanged,
};
use crate::services::metadata::{MetadataJob, MetadataLoaded, MetadataLoader};
use crate::{MediaCommand, MessageAndroidToRust, ANDROID_TX};
use n_event_bus::{Ctx, EventWriter, Handle, Job, JobToken, Outbox, Registrar, Subscriber, Tagged};
use std::any::Any;
use std::sync::Arc;
use tempfile::NamedTempFile;

pub struct AndroidEventJob;

impl Job for AndroidEventJob {
    async fn run(self, _tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        while let Ok(message) = ANDROID_TX.recv_async().await {
            if let MessageAndroidToRust::Callback(command) = message {
                match command {
                    MediaCommand::TogglePause => writer.emit(TogglePause),
                    MediaCommand::PlayNext => writer.emit(PlayNext),
                    MediaCommand::PlayPrevious => writer.emit(PlayPrevious),
                    MediaCommand::SeekAbsolute(position) => writer.emit(Seek::Absolute(position)),
                    MediaCommand::Play => writer.emit(Play),
                }
            } else {
                let _ = ANDROID_TX.send(message);
                tokio::task::yield_now().await;
            }
        }
    }
}

pub struct AndroidBridge {
    jvm: Arc<jni::JavaVM>,
    callback: Arc<jni::objects::GlobalRef>,
    tmp: Option<NamedTempFile>,
    metadata_loader: MetadataLoader,
    last_position: f64,
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
            tmp: None,
            metadata_loader: MetadataLoader::default(),
            last_position: 0.0,
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
        reg.on::<PositionChanged>();
    }
}

impl Handle<PlaybackChanged> for AndroidBridge {
    fn handle(&mut self, msg: &PlaybackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        let mut env = self.jvm.attach_current_thread().unwrap();
        env.call_method(
            self.callback.as_ref(),
            "changePlaybackStatus",
            "(Z)V",
            &[msg.0.into()],
        )
        .unwrap();
    }
}

impl Handle<PositionChanged> for AndroidBridge {
    fn handle(&mut self, msg: &PositionChanged, _ctx: &Ctx, _out: &mut Outbox) {
        if (msg.0.position - self.last_position).abs() > 0.5 {
            self.last_position = msg.0.position;
            let mut env = self.jvm.attach_current_thread().unwrap();
            env.call_method(
                self.callback.as_ref(),
                "changePlaybackSeek",
                "(D)V",
                &[msg.0.position.into()],
            )
            .unwrap();
        }
    }
}

impl Handle<TrackChanged> for AndroidBridge {
    fn handle(&mut self, msg: &TrackChanged, ctx: &Ctx, _out: &mut Outbox) {
        self.metadata_loader.load(msg.path.clone(), ctx);
    }
}

impl Handle<Tagged<MetadataLoaded>> for AndroidBridge {
    fn handle(&mut self, msg: &Tagged<MetadataLoaded>, _ctx: &Ctx, _out: &mut Outbox) {
        let Some(loaded) = self.metadata_loader.take(msg) else {
            return;
        };
        let meta = loaded.metadata;
        let cover_path = loaded
            .cover
            .as_ref()
            .map(|file| file.path().to_string_lossy().into_owned())
            .unwrap_or_default();
        self.tmp = loaded.cover;
        let mut env = self.jvm.attach_current_thread().unwrap();
        let title = env.new_string(meta.title).unwrap();
        let artist = env.new_string(meta.artist).unwrap();
        let cover_path = env.new_string(cover_path).unwrap();
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
        )
        .unwrap();
    }
}
