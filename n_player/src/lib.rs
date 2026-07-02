use bitcode::{Decode, Encode};
#[cfg(target_os = "android")]
use flume::{Receiver, RecvError, SendError, Sender, TryRecvError};
#[cfg(target_os = "android")]
use once_cell::sync::Lazy;
use slint::private_unstable_api::re_exports::ColorScheme;
use slint::SharedPixelBuffer;

slint::include_modules!();

// glibc parks scan-churn in per-thread arenas and never returns it to the OS
// (see PLAN.md's memory findings); jemalloc decays dirty pages back to the OS.
// Kept off Android to not complicate the NDK build.
#[cfg(not(target_os = "android"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// Returns freed pages to the OS. jemalloc only purges during allocation
/// activity, so without this the scan churn (~1GB) stays resident while the
/// app idles after a scan (see PLAN.md). "arena.4096.purge" purges all arenas
/// (4096 = MALLCTL_ARENAS_ALL).
#[cfg(not(target_os = "android"))]
pub fn purge_freed_memory() {
    // SAFETY: plain FFI into the already-linked jemalloc, nothing else.
    // oldp/oldlenp/newp are all null and newlen is 0, so mallctl neither
    // reads nor writes caller memory; the name is a valid NUL-terminated
    // string. The call is advisory (madvise freed pages back to the OS) —
    // it changes no allocator configuration and on failure only returns an
    // error code, which has no consequence worth handling.
    unsafe {
        tikv_jemalloc_sys::mallctl(
            c"arena.4096.purge".as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        );
    }
}

#[cfg(target_os = "android")]
pub fn purge_freed_memory() {}

pub mod app;
pub mod bridges;
pub mod jobs;
pub mod localization;
pub mod messages;
pub mod platform;
pub mod playback;
pub mod scenes;
pub mod services;
pub mod settings;

unsafe impl Send for TrackData {}
unsafe impl Sync for TrackData {}

/// Media-session commands coming from the Android side, re-emitted as bus
/// messages by AndroidEventJob.
#[cfg(target_os = "android")]
pub enum MediaCommand {
    TogglePause,
    PlayNext,
    PlayPrevious,
    SeekAbsolute(f64),
    Play,
}

#[cfg(target_os = "android")]
pub struct SenderReceiver<M> {
    tx: Sender<M>,
    rx: Receiver<M>,
}

#[cfg(target_os = "android")]
impl<M> SenderReceiver<M> {
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();
        Self { tx, rx }
    }

    pub fn send(&self, message: M) -> Result<(), SendError<M>> {
        self.tx.send(message)
    }

    pub fn recv(&self) -> Result<M, RecvError> {
        self.rx.recv()
    }

    pub fn try_recv(&self) -> Result<M, TryRecvError> {
        self.rx.try_recv()
    }

    pub async fn send_async(&self, message: M) -> Result<(), SendError<M>> {
        self.tx.send_async(message).await
    }

    pub async fn recv_async(&self) -> Result<M, RecvError> {
        self.rx.recv_async().await
    }
}

#[cfg(target_os = "android")]
pub static ANDROID_RX: Lazy<SenderReceiver<MessageRustToAndroid>> =
    Lazy::new(|| SenderReceiver::new());
#[cfg(target_os = "android")]
pub static ANDROID_TX: Lazy<SenderReceiver<MessageAndroidToRust>> =
    Lazy::new(|| SenderReceiver::new());

#[cfg(target_os = "android")]
pub enum MessageAndroidToRust {
    Callback(MediaCommand),
    Directory(String),
    File(String),
    Start(jni::JavaVM, jni::objects::GlobalRef),
}
#[cfg(target_os = "android")]
pub enum MessageRustToAndroid {
    AskDirectory,
    OpenLink(String),
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    use crate::app::run_app;
    use crate::platform::AndroidPlatform;
    use crate::settings::Settings;

    slint::android::init(app.clone()).unwrap();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let platform = if let Ok(MessageAndroidToRust::Start(jvm, callback)) =
                ANDROID_TX.recv_async().await
            {
                AndroidPlatform::new(app, jvm, callback)
            } else {
                unreachable!()
            };

            run_app(Settings::read_saved(&platform).await, platform).await;
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
    _class: jni::objects::JClass<'local>,
    string: jni::objects::JString<'local>,
) {
    ANDROID_TX
        .send(MessageAndroidToRust::Directory(
            env.get_string(&string)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        ))
        .unwrap()
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_gotFile<'local>(
    mut env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
    string: jni::objects::JString<'local>,
) {
    ANDROID_TX
        .send(MessageAndroidToRust::File(
            env.get_string(&string)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        ))
        .unwrap()
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MainActivity_start<'local>(
    env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
    callback: jni::objects::JObject<'local>,
) {
    let jvm = env.get_java_vm().unwrap();
    let callback = env.new_global_ref(callback).unwrap();
    ANDROID_TX
        .send(MessageAndroidToRust::Start(jvm, callback))
        .unwrap()
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_TogglePause<'local>(
    _class: jni::objects::JClass<'local>,
) {
    ANDROID_TX
        .send(MessageAndroidToRust::Callback(MediaCommand::TogglePause))
        .unwrap()
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_PlayNext<'local>(
    _class: jni::objects::JClass<'local>,
) {
    ANDROID_TX
        .send(MessageAndroidToRust::Callback(MediaCommand::PlayNext))
        .unwrap()
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_PlayPrevious<'local>(
    _class: jni::objects::JClass<'local>,
) {
    ANDROID_TX
        .send(MessageAndroidToRust::Callback(MediaCommand::PlayPrevious))
        .unwrap()
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_MediaCallback_Seek<'local>(
    _class: jni::objects::JClass<'local>,
    seek: jni::sys::jdouble,
) {
    ANDROID_TX
        .send(MessageAndroidToRust::Callback(MediaCommand::SeekAbsolute(
            seek,
        )))
        .unwrap();
    ANDROID_TX
        .send(MessageAndroidToRust::Callback(MediaCommand::Play))
        .unwrap()
}
