use crate::messages::{
    Play, PlayNext, PlayPrevious, PlaybackChanged, PositionChanged, Seek, TogglePause,
    TrackChanged,
};
use crate::services::image::get_image_squared;
use crate::{MediaCommand, MessageAndroidToRust, ANDROID_TX};
use n_audio::music_track::MusicTrack;
use n_audio::remove_ext;
use n_event_bus::{Ctx, EventWriter, Handle, Job, JobToken, Outbox, Registrar, Subscriber};
use std::any::Any;
use std::sync::Arc;
use tempfile::NamedTempFile;
use zune_image::codecs::ImageFormat;

/// Replaces Platform::tick()'s 250ms poll of ANDROID_TX with a push-based
/// loop re-emitting media-session callbacks as bus messages.
pub struct AndroidEventJob;

impl Job for AndroidEventJob {
    async fn run(self, _tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        while let Ok(message) = ANDROID_TX.recv_async().await {
            if let MessageAndroidToRust::Callback(command) = message {
                match command {
                    MediaCommand::TogglePause => writer.emit(TogglePause),
                    MediaCommand::PlayNext => writer.emit(PlayNext),
                    MediaCommand::PlayPrevious => writer.emit(PlayPrevious),
                    MediaCommand::SeekAbsolute(position) => {
                        writer.emit(Seek::Absolute(position))
                    }
                    MediaCommand::Play => writer.emit(Play),
                }
            } else {
                // Not ours (directory/file dialog results): put it back for
                // the Platform methods waiting on it.
                let _ = ANDROID_TX.send(message);
                tokio::task::yield_now().await;
            }
        }
    }
}

/// Consumes PlaybackEngine's *Changed stream and forwards it to the Android
/// media notification over JNI. Not a Scene — it never touches the UI.
pub struct AndroidBridge {
    jvm: Arc<jni::JavaVM>,
    callback: Arc<jni::objects::GlobalRef>,
    tmp: Arc<NamedTempFile>,
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
            tmp: Arc::new(NamedTempFile::new().unwrap()),
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
        // Same 0.5s threshold the old bus_server applied before notifying.
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
    fn handle(&mut self, msg: &TrackChanged, _ctx: &Ctx, _out: &mut Outbox) {
        let path = msg.path.clone();
        let name = msg.name.clone();
        let jvm = self.jvm.clone();
        let callback = self.callback.clone();
        let tmp = self.tmp.clone();
        tokio::spawn(async move {
            let Ok(track) = MusicTrack::new(path.to_string_lossy().to_string()) else {
                return;
            };
            let Ok(Ok(meta)) = tokio::task::spawn_blocking(move || track.get_meta()).await else {
                return;
            };
            let image = get_image_squared(path, 0, 0).await;
            let cover_path = image
                .map(|image| {
                    let _ = image.save_to(tmp.path(), ImageFormat::PNG);
                    tmp.path().to_str().unwrap().to_string()
                })
                .unwrap_or_default();

            let title = if !meta.title.is_empty() {
                meta.title
            } else {
                remove_ext(name.as_ref())
            };

            let mut env = jvm.attach_current_thread().unwrap();
            let title = env.new_string(title).unwrap();
            let artist = env.new_string(meta.artist).unwrap();
            let cover_path = env.new_string(cover_path).unwrap();
            env.call_method(
                callback.as_ref(),
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
        });
    }
}
