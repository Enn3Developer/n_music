use crate::bus_server::Property;
use crate::runner::{Runner, RunnerMessage};
use flume::Sender;
#[cfg(target_os = "linux")]
use pollster::FutureExt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn open_link_desktop(link: String) {
    open::that(link).unwrap();
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn internal_dir_desktop() -> PathBuf {
    let base_dirs = directories::BaseDirs::new().unwrap();
    let local_data_dir = base_dirs.data_local_dir();
    let app_dir = local_data_dir.join("n_music");
    if !app_dir.exists() {
        std::fs::create_dir(app_dir.as_path()).unwrap();
    }
    app_dir
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn ask_music_dir_desktop() -> PathBuf {
    if let Some(path) = rfd::FileDialog::new().pick_folder() {
        path
    } else {
        PathBuf::new()
    }
}

#[allow(async_fn_in_trait, unused_variables)]
pub trait Platform {
    fn open_link(&mut self, link: String);
    fn internal_dir(&self) -> PathBuf;
    fn ask_music_dir(&mut self) -> PathBuf;

    async fn add_runner(&mut self, runner: Arc<RwLock<Runner>>, tx: Sender<RunnerMessage>) {}
    fn properties_changed<P: IntoIterator<Item = Property>>(&mut self, properties: P) {}
}

#[cfg(target_os = "linux")]
pub struct LinuxPlatform {
    server: Option<mpris_server::Server<crate::bus_server::linux::MPRISBridge>>,
}

#[cfg(target_os = "linux")]
impl LinuxPlatform {
    pub fn new() -> Self {
        Self { server: None }
    }
}

#[cfg(target_os = "linux")]
impl Platform for LinuxPlatform {
    fn open_link(&mut self, link: String) {
        open_link_desktop(link)
    }

    fn internal_dir(&self) -> PathBuf {
        internal_dir_desktop()
    }

    fn ask_music_dir(&mut self) -> PathBuf {
        ask_music_dir_desktop()
    }

    async fn add_runner(&mut self, runner: Arc<RwLock<Runner>>, tx: Sender<RunnerMessage>) {
        let server = mpris_server::Server::new(
            "n_music",
            crate::bus_server::linux::MPRISBridge::new(runner, tx.clone()),
        )
        .await
        .unwrap();
        self.server = Some(server);
    }
    fn properties_changed<P: IntoIterator<Item = Property>>(&mut self, properties: P) {
        if let Some(server) = &self.server {
            server
                .properties_changed(properties.into_iter().map(|p| match p {
                    Property::Playing(playing) => {
                        mpris_server::Property::PlaybackStatus(if playing {
                            mpris_server::PlaybackStatus::Playing
                        } else {
                            mpris_server::PlaybackStatus::Paused
                        })
                    }
                    Property::Metadata(metadata) => {
                        let mut meta = mpris_server::Metadata::new();

                        meta.set_title(metadata.title);
                        meta.set_artist(metadata.artists);
                        meta.set_length(Some(mpris_server::Time::from_secs(
                            metadata.length as i64,
                        )));
                        meta.set_art_url(metadata.image_path);
                        meta.set_trackid(Some(
                            mpris_server::zbus::zvariant::ObjectPath::from_string_unchecked(
                                metadata.id,
                            ),
                        ));

                        mpris_server::Property::Metadata(meta)
                    }
                    Property::Volume(volume) => mpris_server::Property::Volume(volume),
                }))
                .block_on()
                .unwrap()
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub struct DesktopPlatform {}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl Platform for DesktopPlatform {
    fn open_link(&mut self, link: String) {
        open_link_desktop(link)
    }

    fn internal_dir(&self) -> PathBuf {
        internal_dir_desktop()
    }

    fn ask_music_dir(&mut self) -> PathBuf {
        ask_music_dir_desktop()
    }
}

#[cfg(target_os = "android")]
pub struct AndroidPlatform {
    app: slint::android::AndroidApp,
    jvm: jni::JavaVM,
    callback: jni::objects::GlobalRef,
}

#[cfg(target_os = "android")]
impl AndroidPlatform {
    pub fn new(
        app: slint::android::AndroidApp,
        jvm: jni::JavaVM,
        callback: jni::objects::GlobalRef,
    ) -> Self {
        Self { app, jvm, callback }
    }
}

#[cfg(target_os = "android")]
impl Platform for AndroidPlatform {
    fn open_link(&mut self, link: String) {
        let mut env = self.jvm.attach_current_thread().unwrap();
        let java_string = env.new_string(link).unwrap();
        env.call_method(
            &self.callback,
            "openLink",
            "(Ljava/lang/String;)V",
            &[(&java_string).into()],
        )
        .unwrap();
    }

    fn internal_dir(&self) -> PathBuf {
        let path = self
            .app
            .external_data_path()
            .expect("can't get external data path")
            .join("config/");
        if !path.exists() {
            std::fs::create_dir(&path).unwrap();
        }
        path
    }

    fn ask_music_dir(&mut self) -> PathBuf {
        let mut env = self.jvm.attach_current_thread().unwrap();
        env.call_method(&self.callback, "askDirectory", "()V", &[])
            .unwrap();
        while let Ok(message) = crate::ANDROID_TX.recv() {
            if let crate::MessageAndroidToRust::Directory(path) = message {
                return PathBuf::from(path);
            } else {
                crate::ANDROID_TX.send(message).unwrap();
            }
        }
        PathBuf::new()
    }
}
