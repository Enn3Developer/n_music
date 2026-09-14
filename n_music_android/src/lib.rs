#![cfg(target_os = "android")]

slint::include_modules!();

mod app;
mod bridge;
mod localization;
mod platform;
mod scenes;
mod ui;

use n_event_bus::{EventWriter, Message};
use std::sync::{Arc, LazyLock, Mutex};

static ANDROID_BUS: LazyLock<(
    EventWriter,
    Mutex<Option<n_event_bus::EventReceiver>>,
)> = LazyLock::new(|| {
    let (writer, receiver) = EventWriter::channel();
    (writer, Mutex::new(Some(receiver)))
});

pub struct AndroidStarted(
    pub Arc<jni::JavaVM>,
    pub Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
);
impl Message for AndroidStarted {}

// androidx.media3.common.Player REPEAT_MODE_OFF / ONE / ALL
const REPEAT_MODE_ONE: i32 = 1;

#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app.clone()).unwrap();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let rx = ANDROID_BUS
                .1
                .lock()
                .unwrap()
                .take()
                .expect("Android bus already running");
            let mut pending = Vec::new();
            while let Ok(event) = rx.recv_async().await {
                if let n_event_bus::Event::Bus(envelope) = &event {
                    if let Some(started) = envelope.payload().downcast_ref::<AndroidStarted>() {
                        let platform = platform::AndroidPlatform::new(
                            app,
                            started.0.clone(),
                            started.1.clone(),
                        );
                        crate::app::run(
                            n_music_core::settings::Settings::read_saved(&platform).await,
                            platform,
                            ANDROID_BUS.0.clone(),
                            rx,
                            pending,
                        )
                        .await;
                        return;
                    }
                }
                pending.push(event);
            }
        });
}

#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_gotDirectory<'local>(
    mut env: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
    string: jni::objects::JString<'local>,
    tag: jni::sys::jlong,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let path = string.try_to_string(env)?;
        ANDROID_BUS.0.emit_tagged(
            tag as u64,
            n_music_core::jobs::settings::DirectoryChosen(std::path::PathBuf::from(path)),
        );
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_start<'local>(
    mut env: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
    callback: jni::objects::JObject<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        ANDROID_BUS.0.emit(AndroidStarted(
            std::sync::Arc::new(env.get_java_vm()?),
            std::sync::Arc::new(env.new_global_ref(&callback)?),
        ));
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_visibilityChanged<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
    visible: jni::sys::jboolean,
) {
    ANDROID_BUS
        .0
        .emit(n_music_core::messages::AppVisibilityChanged(visible));
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_mediaPause<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(n_music_core::messages::Pause);
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_mediaPlay<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(n_music_core::messages::Play);
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_mediaPlayNext<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(n_music_core::messages::PlayNext);
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_mediaPlayPrevious<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(n_music_core::messages::PlayPrevious);
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_mediaSeek<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
    seek: jni::sys::jdouble,
) {
    ANDROID_BUS.0.emit(n_music_core::messages::Seek::Absolute(seek));
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_mediaRepeatMode<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
    mode: jni::sys::jint,
) {
    let loop_status = if mode == REPEAT_MODE_ONE {
        n_music_core::queue::LoopStatus::File
    } else {
        n_music_core::queue::LoopStatus::Playlist
    };
    ANDROID_BUS
        .0
        .emit(n_music_core::messages::SetLoopStatus(loop_status));
}
