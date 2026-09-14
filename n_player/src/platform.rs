use async_trait::async_trait;
use std::path::PathBuf;

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
    async fn ask_music_dir(&self, tag: u64, writer: n_event_bus::EventWriter);
}
