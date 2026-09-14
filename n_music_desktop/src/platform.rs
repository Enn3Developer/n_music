use async_trait::async_trait;
use n_player::jobs::settings::DirectoryChosen;
use n_player::platform::Platform;
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
fn set_clipboard_text_desktop(text: String) {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new().unwrap();
    clipboard.set_text(text).unwrap();
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

    async fn ask_music_dir(&self, tag: u64, writer: n_event_bus::EventWriter) {
        writer.emit_tagged(tag, DirectoryChosen(ask_music_dir_desktop().await));
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

    async fn ask_music_dir(&self, tag: u64, writer: n_event_bus::EventWriter) {
        writer.emit_tagged(tag, DirectoryChosen(ask_music_dir_desktop().await));
    }
}
