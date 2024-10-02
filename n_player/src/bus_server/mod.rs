use crate::runner::Runner;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[cfg(target_os = "linux")]
pub mod linux;

pub enum Property {
    Playing(bool),
    Metadata(Metadata),
    Volume(f64),
}

pub struct Metadata {
    pub title: Option<String>,
    pub artists: Option<Vec<String>>,
    pub length: u64,
    pub id: String,
    pub image_path: Option<String>,
}

pub trait BusServer {
    async fn properties_changed<P: IntoIterator<Item = Property>>(
        &self,
        properties: P,
    ) -> Result<(), String>;
}

pub async fn run<B: BusServer>(server: B, runner: Arc<RwLock<Runner>>) {
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    let mut properties = vec![];
    let mut playback = false;
    let mut volume = 1.0;

    loop {
        interval.tick().await;
        let guard = runner.read().await;

        if playback != guard.playback() {
            playback = guard.playback();
            properties.push(Property::Playing(playback));
        }
        if volume != guard.volume() {
            volume = guard.volume();
            properties.push(Property::Volume(volume))
        }

        if !properties.is_empty() {
            server
                .properties_changed(mem::take(&mut properties))
                .await
                .unwrap();
        }
    }
}
