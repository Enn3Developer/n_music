use n_music_core::jobs::settings::DirectoryChosen;
use n_music_core::platform::Platform;
use std::path::PathBuf;

fn open_link_desktop(link: String) {
    open::that(link).unwrap();
}

fn internal_dir_desktop() -> PathBuf {
    let base_dirs = directories::BaseDirs::new().unwrap();
    let local_data_dir = base_dirs.data_local_dir();
    let app_dir = local_data_dir.join("n_music");
    if !app_dir.exists() {
        std::fs::create_dir(app_dir.as_path()).unwrap();
    }
    app_dir
}

fn ask_music_dir_desktop() -> PathBuf {
    rfd::FileDialog::new().pick_folder().unwrap_or_default()
}

fn set_clipboard_text_desktop(text: String) {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new().unwrap();
    clipboard.set_text(text).unwrap();
}

pub struct DesktopPlatform;

impl DesktopPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Platform for DesktopPlatform {
    fn set_clipboard_text(&self, text: String) {
        set_clipboard_text_desktop(text);
    }

    fn open_link(&self, link: String) {
        open_link_desktop(link)
    }

    fn internal_dir(&self) -> PathBuf {
        internal_dir_desktop()
    }

    fn ask_music_dir(&self, tag: u64, writer: n_event_bus::EventWriter) {
        writer.emit_tagged(tag, DirectoryChosen(ask_music_dir_desktop()));
    }
}
