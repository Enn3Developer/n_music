#![cfg(target_os = "android")]

mod bridge;
mod platform;

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
                        let bridge =
                            bridge::AndroidBridge::new(started.0.clone(), started.1.clone());
                        n_player::app::run_app_with_events(
                            n_player::settings::Settings::read_saved(&platform).await,
                            platform,
                            ANDROID_BUS.0.clone(),
                            rx,
                            pending,
                            move |app| {
                                app.register_subscriber(bridge);
                            },
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
            n_player::jobs::settings::DirectoryChosen(std::path::PathBuf::from(path)),
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
        .emit(n_player::messages::AppVisibilityChanged(visible));
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_Pause<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(n_player::messages::Pause);
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_Play<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(n_player::messages::Play);
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_PlayNext<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(n_player::messages::PlayNext);
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_PlayPrevious<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(n_player::messages::PlayPrevious);
}
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_Seek<'local>(
    _: jni::EnvUnowned<'local>,
    _: jni::objects::JClass<'local>,
    seek: jni::sys::jdouble,
) {
    ANDROID_BUS.0.emit(n_player::messages::Seek::Absolute(seek));
}
