use crate::music_track::MusicTrack;
use crate::player::Player;
use crate::{remove_ext, strip_absolute_path};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::cmp::PartialEq;
use std::io;
use std::io::ErrorKind;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Default, Eq, PartialEq, Debug)]
pub enum LoopStatus {
    #[default]
    Playlist,
    File,
}

pub struct QueuePlayer {
    queue: Vec<Arc<str>>,
    path: String,
    player: Player,
    index: usize,
    loop_status: LoopStatus,
}

impl Default for QueuePlayer {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl QueuePlayer {
    pub fn new(path: String) -> Self {
        let player = Player::new(1.0, 1.0);

        QueuePlayer {
            queue: vec![],
            player,
            index: usize::MAX - 1,
            path,
            loop_status: LoopStatus::Playlist,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn set_path(&mut self, path: String) {
        self.path = path;
    }

    pub fn set_loop_status(&mut self, loop_status: LoopStatus) {
        self.loop_status = loop_status;
    }

    pub async fn get_path_for_file(&self, i: usize) -> Option<PathBuf> {
        Some(PathBuf::from(&self.path).join(self.queue.get(i)?.as_ref()))
    }

    pub fn queue(&self) -> &[Arc<str>] {
        &self.queue
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.path.shrink_to_fit();
        self.queue.shrink_to_fit();
    }

    #[inline]
    pub async fn add<P: Into<Arc<str>>>(&mut self, path: P) {
        self.queue.push(path.into());
    }

    pub async fn add_all<P: Into<String>>(&mut self, paths: impl IntoIterator<Item = P>) {
        self.queue.append(
            &mut paths
                .into_iter()
                .map(|p| strip_absolute_path(p.into()).into())
                .collect::<Vec<Arc<str>>>(),
        );
    }

    #[inline]
    pub fn remove(&mut self, index: usize) {
        self.queue.remove(index);
    }

    #[inline]
    pub async fn clear(&mut self) {
        self.queue.clear();
        self.index = usize::MAX - 1;
    }

    #[inline]
    pub fn shuffle(&mut self) {
        self.queue.shuffle(&mut thread_rng());
    }

    pub async fn current_track_name(&self) -> Option<Arc<str>> {
        self.queue.get(self.index).map(|t| t.clone())
    }

    pub async fn play(&mut self) -> io::Result<()> {
        let track = MusicTrack::new(
            self.get_path_for_file(self.index)
                .await
                .ok_or(io::Error::from(ErrorKind::NotFound))?
                .to_str()
                .unwrap(),
        )?;
        let format = tokio::task::spawn_blocking(move || track.get_format()).await??;

        self.player.play(format);
        Ok(())
    }

    pub async fn play_index(&mut self, index: usize) -> io::Result<()> {
        self.index = index;

        self.play().await
    }

    pub async fn play_next(&mut self, ignore_loop: bool) -> io::Result<()> {
        if ignore_loop || self.loop_status == LoopStatus::Playlist {
            self.index += 1;

            if self.index >= self.len() {
                self.index = 0;
            }
        }
        self.play().await
    }

    pub async fn play_previous(&mut self) -> io::Result<()> {
        if self.index == 0 {
            self.index = self.len();
        }

        self.index -= 1;

        self.play().await
    }

    pub fn get_index_from_track_name(&self, name: &str) -> Option<usize> {
        self.queue
            .iter()
            .map(|t| remove_ext(t.as_ref()))
            .enumerate()
            .find(|(_i, t)| t == name)
            .map(|(i, _t)| i)
    }
}

impl Deref for QueuePlayer {
    type Target = Player;

    fn deref(&self) -> &Self::Target {
        &self.player
    }
}

impl DerefMut for QueuePlayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.player
    }
}
