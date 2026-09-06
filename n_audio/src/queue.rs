use crate::player::{PlaybackTask, Player};
use crate::{remove_ext, strip_absolute_path};
use rand::prelude::SliceRandom;
use rand::rng;
use std::cmp::PartialEq;
use std::io;
use std::io::ErrorKind;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Default, Eq, PartialEq, Debug, Clone)]
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

    pub fn loop_status(&self) -> LoopStatus {
        self.loop_status.clone()
    }

    pub fn get_path_for_file(&self, i: usize) -> Option<PathBuf> {
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
    pub fn add<P: Into<Arc<str>>>(&mut self, path: P) {
        self.queue.push(path.into());
    }

    pub fn add_all<P: Into<String>>(&mut self, paths: impl IntoIterator<Item = P>) {
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
    pub fn clear(&mut self) {
        self.player.end_current();
        self.queue.clear();
        self.index = usize::MAX - 1;
    }

    #[inline]
    pub fn shuffle(&mut self) {
        self.queue.shuffle(&mut rng());
    }

    pub fn current_track_name(&self) -> Option<Arc<str>> {
        self.queue.get(self.index).map(|t| t.clone())
    }

    pub fn prepare_index(&mut self, index: usize) -> io::Result<PlaybackTask> {
        if self.queue.is_empty() {
            return Err(ErrorKind::NotFound.into());
        }
        self.index = index % self.len();
        let path = self
            .get_path_for_file(self.index)
            .ok_or(ErrorKind::NotFound)?;
        Ok(self.player.prepare_path(path))
    }

    pub fn prepare_next(&mut self, ignore_loop: bool) -> io::Result<PlaybackTask> {
        let index = if self.index >= self.len() {
            0
        } else if ignore_loop || self.loop_status == LoopStatus::Playlist {
            self.index + 1
        } else {
            self.index
        };
        self.prepare_index(index)
    }

    pub fn prepare_previous(&mut self) -> io::Result<PlaybackTask> {
        let index = if self.index == 0 || self.index >= self.len() {
            self.len().saturating_sub(1)
        } else {
            self.index - 1
        };
        self.prepare_index(index)
    }

    pub fn play(&mut self) -> io::Result<()> {
        self.play_index(self.index)
    }
    pub fn play_index(&mut self, index: usize) -> io::Result<()> {
        self.prepare_index(index)?.spawn();
        Ok(())
    }
    pub fn play_next(&mut self, ignore_loop: bool) -> io::Result<()> {
        self.prepare_next(ignore_loop)?.spawn();
        Ok(())
    }
    pub fn play_previous(&mut self) -> io::Result<()> {
        self.prepare_previous()?.spawn();
        Ok(())
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
