use bitcode::{Decode, Encode};
use slint::private_unstable_api::re_exports::ColorScheme;
use slint::SharedPixelBuffer;

slint::include_modules!();

#[cfg(not(target_os = "android"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub mod app;
pub mod bridges;
pub mod jobs;
pub mod localization;
pub mod messages;
pub mod platform;
pub mod runner;
pub mod scenes;
pub mod services;
pub mod settings;

unsafe impl Send for TrackData {}
unsafe impl Sync for TrackData {}

#[cfg(target_os = "android")]
pub static ANDROID_BUS: std::sync::LazyLock<(
    n_event_bus::EventWriter,
    std::sync::Mutex<Option<n_event_bus::EventReceiver>>,
)> = std::sync::LazyLock::new(|| {
    let (writer, receiver) = n_event_bus::EventWriter::channel();
    (writer, std::sync::Mutex::new(Some(receiver)))
});
#[cfg(target_os = "android")]
pub struct AndroidStarted(
    pub std::sync::Arc<jni::JavaVM>,
    pub std::sync::Arc<jni::objects::GlobalRef>,
);
#[cfg(target_os = "android")]
impl n_event_bus::Message for AndroidStarted {}

#[cfg(target_os = "android")]
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
                        app::run_app_with_events(
                            settings::Settings::read_saved(&platform).await,
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

#[derive(Copy, Clone, Debug, Decode, Encode)]
pub struct WindowSize {
    pub width: usize,
    pub height: usize,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 450,
            height: 625,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Decode, Encode)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Into<ColorScheme> for Theme {
    fn into(self) -> ColorScheme {
        match self {
            Theme::System => ColorScheme::Unknown,
            Theme::Light => ColorScheme::Light,
            Theme::Dark => ColorScheme::Dark,
        }
    }
}

impl From<Theme> for String {
    fn from(value: Theme) -> Self {
        match value {
            Theme::System => String::from("System"),
            Theme::Light => String::from("Light"),
            Theme::Dark => String::from("Dark"),
        }
    }
}
impl From<Theme> for i32 {
    fn from(value: Theme) -> Self {
        match value {
            Theme::System => 0,
            Theme::Light => 1,
            Theme::Dark => 2,
        }
    }
}

impl TryFrom<String> for Theme {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if &value == "System" {
            Ok(Self::System)
        } else if &value == "Light" {
            Ok(Self::Light)
        } else if &value == "Dark" {
            Ok(Self::Dark)
        } else {
            Err(format!("{value} is not a valid theme"))
        }
    }
}

impl TryFrom<i32> for Theme {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value == 0 {
            Ok(Self::System)
        } else if value == 1 {
            Ok(Self::Light)
        } else if value == 2 {
            Ok(Self::Dark)
        } else {
            Err(format!("{value} is not a valid theme"))
        }
    }
}

#[derive(Clone, Debug, Decode, Encode)]
pub struct FileTrack {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub length: f64,
    pub image: Vec<u8>,
}

impl From<FileTrack> for TrackData {
    fn from(mut value: FileTrack) -> Self {
        value.artist.shrink_to_fit();
        value.title.shrink_to_fit();
        value.image.shrink_to_fit();
        Self {
            artist: value.artist.into(),
            cover: if !value.image.is_empty() {
                slint::Image::from_rgb8(SharedPixelBuffer::clone_from_slice(&value.image, 128, 128))
            } else {
                Default::default()
            },
            index: 0,
            time: format!(
                "{:02}:{:02}",
                (value.length / 60.0).floor() as u64,
                value.length.floor() as u64 % 60
            )
            .into(),
            title: value.title.into(),
            visible: true,
        }
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_gotDirectory<'local>(
    mut env: jni::JNIEnv<'local>,
    _: jni::objects::JClass<'local>,
    string: jni::objects::JString<'local>,
    tag: jni::sys::jlong,
) {
    if let Ok(path) = env.get_string(&string) {
        ANDROID_BUS.0.emit_tagged(
            tag as u64,
            jobs::settings::DirectoryChosen(std::path::PathBuf::from(
                path.to_string_lossy().into_owned(),
            )),
        );
    }
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_start<'local>(
    env: jni::JNIEnv<'local>,
    _: jni::objects::JClass<'local>,
    callback: jni::objects::JObject<'local>,
) {
    ANDROID_BUS.0.emit(AndroidStarted(
        std::sync::Arc::new(env.get_java_vm().unwrap()),
        std::sync::Arc::new(env.new_global_ref(callback).unwrap()),
    ));
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_visibilityChanged<'local>(
    _: jni::JNIEnv<'local>,
    _: jni::objects::JClass<'local>,
    visible: jni::sys::jboolean,
) {
    ANDROID_BUS
        .0
        .emit(messages::AppVisibilityChanged(visible != 0));
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_Pause<'local>(
    _: jni::JNIEnv<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(messages::Pause);
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_Play<'local>(
    _: jni::JNIEnv<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(messages::Play);
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_PlayNext<'local>(
    _: jni::JNIEnv<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(messages::PlayNext);
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_PlayPrevious<'local>(
    _: jni::JNIEnv<'local>,
    _: jni::objects::JClass<'local>,
) {
    ANDROID_BUS.0.emit(messages::PlayPrevious);
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_Seek<'local>(
    _: jni::JNIEnv<'local>,
    _: jni::objects::JClass<'local>,
    seek: jni::sys::jdouble,
) {
    ANDROID_BUS.0.emit(messages::Seek::Absolute(seek));
}
