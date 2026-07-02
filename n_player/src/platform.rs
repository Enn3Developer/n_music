use async_trait::async_trait;
use std::path::PathBuf;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn open_link_desktop(link: String) {
    open::that(link).unwrap();
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
async fn internal_dir_desktop() -> PathBuf {
    let base_dirs = directories::BaseDirs::new().unwrap();
    let local_data_dir = base_dirs.data_local_dir();
    let app_dir = local_data_dir.join("n_music");
    if !app_dir.exists() {
        tokio::fs::create_dir(app_dir.as_path()).await.unwrap();
    }
    app_dir
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
async fn ask_music_dir_desktop() -> PathBuf {
    if let Some(path) = rfd::AsyncFileDialog::new().pick_folder().await {
        PathBuf::from(path)
    } else {
        PathBuf::new()
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
async fn ask_file_desktop() -> Vec<PathBuf> {
    if let Some(path) = rfd::AsyncFileDialog::new().pick_files().await {
        path.into_iter().map(|fd| PathBuf::from(fd)).collect()
    } else {
        vec![]
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn set_clipboard_text_desktop(text: String) {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new().unwrap();
    clipboard.set_text(text).unwrap();
}

#[async_trait]
/// Abstraction over a number of platforms (desktop and mobile)
pub trait Platform: Send + Sync {
    /// Ask the platform to "copy" a given text
    fn set_clipboard_text(&self, text: String);

    /// Ask underlying platform to open a web link
    async fn open_link(&self, link: String);
    /// Ask underlying platform to get the app directory
    async fn internal_dir(&self) -> PathBuf;
    /// Ask underlying platform to ask user for the music dir
    async fn ask_music_dir(&self) -> PathBuf;
    /// Ask underlying platform to ask user for files
    async fn ask_file(&self) -> Vec<PathBuf>;
    /// JNI handles for the Android bridge
    #[cfg(target_os = "android")]
    fn jni_handles(&self) -> (std::sync::Arc<jni::JavaVM>, std::sync::Arc<jni::objects::GlobalRef>);
}

#[cfg(target_os = "linux")]
pub struct LinuxPlatform;

#[cfg(target_os = "linux")]
impl LinuxPlatform {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
impl Platform for LinuxPlatform {
    fn set_clipboard_text(&self, text: String) {
        set_clipboard_text_desktop(text);
    }

    async fn open_link(&self, link: String) {
        open_link_desktop(link)
    }

    async fn internal_dir(&self) -> PathBuf {
        internal_dir_desktop().await
    }

    async fn ask_music_dir(&self) -> PathBuf {
        ask_music_dir_desktop().await
    }

    async fn ask_file(&self) -> Vec<PathBuf> {
        ask_file_desktop().await
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub struct DesktopPlatform {}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[async_trait]
impl Platform for DesktopPlatform {
    fn set_clipboard_text(&self, text: String) {
        set_clipboard_text_desktop(text);
    }

    async fn open_link(&self, link: String) {
        open_link_desktop(link)
    }

    async fn internal_dir(&self) -> PathBuf {
        internal_dir_desktop().await
    }

    async fn ask_music_dir(&self) -> PathBuf {
        ask_music_dir_desktop().await
    }

    async fn ask_file(&self) -> Vec<PathBuf> {
        ask_file_desktop().await
    }
}

#[cfg(target_os = "android")]
pub struct AndroidPlatform {
    app: slint::android::AndroidApp,
    jvm: std::sync::Arc<jni::JavaVM>,
    callback: std::sync::Arc<jni::objects::GlobalRef>,
}

#[cfg(target_os = "android")]
impl AndroidPlatform {
    pub fn new(
        app: slint::android::AndroidApp,
        jvm: jni::JavaVM,
        callback: jni::objects::GlobalRef,
    ) -> Self {
        Self {
            app,
            jvm: std::sync::Arc::new(jvm),
            callback: std::sync::Arc::new(callback),
        }
    }

}

#[cfg(target_os = "android")]
#[async_trait]
impl Platform for AndroidPlatform {
    fn set_clipboard_text(&self, text: String) {
        let mut env = self.jvm.attach_current_thread().unwrap();
        let java_string = env.new_string(text).unwrap();
        env.call_method(
            self.callback.as_ref(),
            "set_clipboard_text",
            "(Ljava/lang/String;)V",
            &[(&java_string).into()],
        )
        .unwrap();
    }

    async fn open_link(&self, link: String) {
        let mut env = self.jvm.attach_current_thread().unwrap();
        let java_string = env.new_string(link).unwrap();
        env.call_method(
            self.callback.as_ref(),
            "openLink",
            "(Ljava/lang/String;)V",
            &[(&java_string).into()],
        )
        .unwrap();
    }

    async fn internal_dir(&self) -> PathBuf {
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

    async fn ask_music_dir(&self) -> PathBuf {
        let mut env = self.jvm.attach_current_thread().unwrap();
        env.call_method(self.callback.as_ref(), "askDirectory", "()V", &[])
            .unwrap();
        while let Ok(message) = crate::ANDROID_TX.recv() {
            if let crate::MessageAndroidToRust::Directory(path) = message {
                println!("got directory from user");
                return PathBuf::from(path);
            } else {
                crate::ANDROID_TX.send(message).unwrap();
            }
        }
        PathBuf::new()
    }

    async fn ask_file(&self) -> Vec<PathBuf> {
        let mut env = self.jvm.attach_current_thread().unwrap();
        env.call_method(self.callback.as_ref(), "askFile", "()V", &[])
            .unwrap();
        while let Ok(message) = crate::ANDROID_TX.recv() {
            if let crate::MessageAndroidToRust::File(path) = message {
                return vec![PathBuf::from(path)];
            } else {
                crate::ANDROID_TX.send(message).unwrap();
            }
        }
        vec![]
    }

    fn jni_handles(
        &self,
    ) -> (
        std::sync::Arc<jni::JavaVM>,
        std::sync::Arc<jni::objects::GlobalRef>,
    ) {
        (self.jvm.clone(), self.callback.clone())
    }
}
