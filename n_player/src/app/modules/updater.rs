use crate::app::modules::window::WindowModule;
use crate::app::modules::{Module, ModuleMessage};
use crate::app::{AppMessage, Messenger, Platform, Runner, Settings};
use crate::{FileTrack, TrackData};
use flume::Receiver;
use std::time::Duration;

pub enum UpdaterMessage {
    Metadata(u16, FileTrack),
    ChangingTime,
    StartedLoading(Vec<FileTrack>),
    Searching(String),
    Quit,
}

impl ModuleMessage for UpdaterMessage {
    fn quit() -> Self {
        Self::Quit
    }
}

pub struct UpdaterModule<P: crate::platform::Platform + Send + 'static> {
    platform: Platform<P>,
    settings: Settings,
    runner: Runner,
    messenger: Messenger<P>,
    rx: Receiver<UpdaterMessage>,
}

#[async_trait::async_trait]
impl<P: crate::platform::Platform + Send + 'static> Module<P> for UpdaterModule<P> {
    type Message = UpdaterMessage;

    async fn setup(
        platform: Platform<P>,
        settings: Settings,
        runner: Runner,
        messenger: Messenger<P>,
        rx: Receiver<Self::Message>,
    ) -> Self {
        Self {
            platform,
            settings,
            runner,
            messenger,
            rx,
        }
    }

    async fn start(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        let mut searching = String::new();
        let mut old_index = u16::MAX;
        let mut loaded = 0u16;
        let threshold = (num_cpus::get() * 4) as u16;
        let mut tracks: Vec<TrackData> = vec![];
        'exit: loop {
            interval.tick().await;
            let guard = self.runner.read().await;
            let mut index = guard.index();
            let len = guard.len() as u16;
            if index > len {
                index = 0;
            }
            let playback = guard.playback();
            let time = guard.time();
            let length = time.length;
            let time_float = time.position;
            let volume = guard.volume();
            let position = time.format_pos();
            let mut change_time = false;
            let mut new_loaded = false;
            let mut updated_search = false;

            while let Ok(message) = self.rx.try_recv() {
                match message {
                    UpdaterMessage::Metadata(index, file_track) => {
                        let file = file_track.clone();
                        self.settings.lock().await.tracks.push(file);
                        tracks[index as usize] = file_track.into();
                        tracks[index as usize].index = index as i32;
                        loaded += 1;
                        if loaded % threshold == 0 || loaded == len {
                            new_loaded = true;
                        }
                    }
                    UpdaterMessage::ChangingTime => change_time = true,
                    UpdaterMessage::StartedLoading(file_tracks) => {
                        tracks = file_tracks
                            .into_iter()
                            .enumerate()
                            .map(|(index, track)| {
                                let mut track = TrackData::from(track);
                                track.index = index as i32;
                                track
                            })
                            .collect();
                        new_loaded = true;
                    }
                    UpdaterMessage::Searching(search_string) => {
                        searching = search_string;
                        updated_search = true;
                    }
                    UpdaterMessage::Quit => {
                        break 'exit;
                    }
                }
            }

            let progress = loaded as f64 / len as f64;
            let mut playing_track = None;
            if old_index != index || new_loaded {
                if let Some(track) = tracks.get(index as usize) {
                    playing_track = Some(track.clone());
                    old_index = index;
                }
            }

            let mut t = vec![];

            let is_searching = !searching.is_empty();

            if new_loaded || updated_search {
                t = tracks.clone();
            }

            if is_searching && (updated_search || new_loaded) {
                t = t
                    .into_iter()
                    .filter(|track| {
                        let search = searching.to_lowercase();
                        track.title.to_lowercase().contains(&search)
                            || track.artist.to_lowercase().contains(&search)
                    })
                    .collect();
            }
            self.platform.lock().await.tick().await;

            self.messenger
                .send_async(AppMessage::WindowMessage(
                    <WindowModule<P> as Module<P>>::Message::SetPlayingIndex(index),
                ))
                .await
                .unwrap();
            self.messenger
                .send_async(AppMessage::WindowMessage(
                    <WindowModule<P> as Module<P>>::Message::SetTimePosition(position),
                ))
                .await
                .unwrap();
            self.messenger
                .send_async(AppMessage::WindowMessage(
                    <WindowModule<P> as Module<P>>::Message::SetLength(length),
                ))
                .await
                .unwrap();
            self.messenger
                .send_async(AppMessage::WindowMessage(
                    <WindowModule<P> as Module<P>>::Message::SetPlayback(playback),
                ))
                .await
                .unwrap();
            self.messenger
                .send_async(AppMessage::WindowMessage(
                    <WindowModule<P> as Module<P>>::Message::SetVolume(volume),
                ))
                .await
                .unwrap();
            if change_time {
                self.messenger
                    .send_async(AppMessage::WindowMessage(
                        <WindowModule<P> as Module<P>>::Message::SetTime(time_float),
                    ))
                    .await
                    .unwrap();
            }
            if let Some(playing_track) = playing_track {
                self.messenger
                    .send_async(AppMessage::WindowMessage(
                        <WindowModule<P> as Module<P>>::Message::SetPlayingTrack(playing_track),
                    ))
                    .await
                    .unwrap();
            }
            if new_loaded {
                let progress = if progress == 1.0 { 0.0 } else { progress };
                self.messenger
                    .send_async(AppMessage::WindowMessage(
                        <WindowModule<P> as Module<P>>::Message::SetProgress(progress),
                    ))
                    .await
                    .unwrap();
            }
            if new_loaded || updated_search {
                self.messenger
                    .send_async(AppMessage::WindowMessage(
                        <WindowModule<P> as Module<P>>::Message::SetTracks(t),
                    ))
                    .await
                    .unwrap();
            }
        }
    }

    fn start_sync(&self) {
        unreachable!("UpdaterModule was declared as async but the app function started it as sync")
    }
}
