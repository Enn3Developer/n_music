use bitcode::{Decode, Encode};
#[cfg(target_os = "android")]
use flume::{Receiver, RecvError, SendError, Sender, TryRecvError};
use multitag::data::Picture;
use multitag::Tag;
use n_audio::queue::QueuePlayer;
#[cfg(target_os = "android")]
use once_cell::sync::Lazy;
use slint::private_unstable_api::re_exports::ColorScheme;
use slint::SharedPixelBuffer;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::Path;

slint::include_modules!();

pub mod app;
pub mod bus_server;
pub mod localization;
pub mod platform;
pub mod runner;
pub mod settings;

unsafe impl Send for TrackData {}
unsafe impl Sync for TrackData {}

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
    Directory(String),
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
    use crate::platform::Platform;
    use crate::settings::Settings;
    use slint::platform::WindowEvent;
    use std::sync::Arc;

    slint::android::init(app.clone()).unwrap();
    let platform = if let Ok(MessageAndroidToRust::Start(jvm, callback)) = ANDROID_TX.recv() {
        Arc::new(std::sync::Mutex::new(AndroidPlatform::new(
            app, jvm, callback,
        )))
    } else {
        unreachable!()
    };

    let settings = Arc::new(std::sync::Mutex::new(Settings::read_saved(
        platform.lock().unwrap(),
    )));
    if !Path::new(&settings.lock().unwrap().path).exists() {
        let window = AndroidWindow::new().unwrap();
        let handle = window.as_weak();
        let settings = settings.clone();
        let platform = platform.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let path = platform.lock().unwrap().ask_music_dir();
            settings.lock().unwrap().path = path.to_str().unwrap().to_string();
            handle
                .upgrade_in_event_loop(|window| {
                    window.window().dispatch_event(WindowEvent::CloseRequested);
                })
                .unwrap();
        });
        window.run().unwrap();
    }
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            run_app(
                Arc::into_inner(settings).unwrap().into_inner().unwrap(),
                Arc::into_inner(platform).unwrap().into_inner().unwrap(),
            )
            .await;
        });
}

pub fn get_image<P: AsRef<Path> + Debug>(path: P) -> Vec<u8> {
    if let Ok(tag) = Tag::read_from_path(path.as_ref()) {
        if let Some(album) = tag.get_album_info() {
            if let Some(cover) = album.cover {
                return cover.data;
            } else {
                if let Tag::OpusTag { inner } = tag {
                    let cover = inner.pictures().first().cloned().map(Picture::from);
                    if let Some(cover) = cover {
                        return cover.data;
                    }
                } else if let Tag::Id3Tag { inner } = tag {
                    let cover = inner.pictures().next().cloned().map(Picture::from);
                    if let Some(cover) = cover {
                        return cover.data;
                    }
                } else {
                    eprintln!("not an opus or mp3 tag {path:?}");
                }
            }
        } else {
            eprintln!("no album for {path:?}");
        }
    }

    vec![]
}

pub async fn add_all_tracks_to_player<P: AsRef<Path> + AsRef<OsStr> + From<String>>(
    player: &mut QueuePlayer,
    path: P,
) {
    if let Ok(mut dir) = tokio::fs::read_dir(path).await {
        let mut paths = vec![];
        while let Ok(Some(file)) = dir.next_entry().await {
            if file.file_type().await.unwrap().is_file() {
                if let Ok(Some(mime)) = infer::get_from_path(&file.path()) {
                    if mime.mime_type().contains("audio") {
                        let mut p = file.path().to_str().unwrap().to_string();
                        p.shrink_to_fit();
                        paths.push(p);
                    }
                }
            }
        }
        player.add_all(paths).await.unwrap();
        player.shrink_to_fit();

        player.shuffle();
    }
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
