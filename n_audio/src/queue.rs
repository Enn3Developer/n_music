use crate::music_track::MusicTrack;
use crate::player::Player;
use crate::{remove_ext, strip_absolute_path, NError};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, ErrorKind, Seek, SeekFrom, Write};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct QueuePlayer {
    queue_file: Arc<RwLock<BufReader<File>>>,
    path: String,
    player: Player,
    index: usize,
    index_map: Vec<u64>,
}

impl Default for QueuePlayer {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl QueuePlayer {
    pub fn new(path: String) -> Self {
        let player = Player::new(1.0, 1.0);
        let queue_file = Arc::new(RwLock::new(BufReader::new(tempfile::tempfile().unwrap())));

        QueuePlayer {
            queue_file,
            player,
            index: usize::MAX - 1,
            path,
            index_map: vec![],
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn len(&self) -> usize {
        self.index_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index_map.is_empty()
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn set_path(&mut self, path: String) {
        self.path = path;
    }

    pub async fn get_path_for_file(&self, i: usize) -> Option<PathBuf> {
        let mut guard = self.queue_file.write().await;
        guard
            .seek(SeekFrom::Start(*self.index_map.get(i)?))
            .unwrap();
        let mut name = String::new();
        guard.read_line(&mut name).unwrap();
        name = name.replace("\n", "");

        Some(PathBuf::from(&self.path).join(name))
    }

    pub fn queue(&self) -> Arc<RwLock<BufReader<File>>> {
        self.queue_file.clone()
    }

    pub fn index_map(&self) -> Vec<u64> {
        self.index_map.clone()
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.path.shrink_to_fit();
    }

    #[inline]
    pub async fn add<P: Into<String>>(&mut self, path: P) -> io::Result<()> {
        self.index_map
            .push(self.queue_file.write().await.stream_position()?);
        self.queue_file
            .write()
            .await
            .get_mut()
            .write_all(format!("{}\n", strip_absolute_path(path.into())).as_bytes())?;
        Ok(())
    }

    pub async fn add_all<P: Into<String>>(
        &mut self,
        paths: impl IntoIterator<Item = P>,
    ) -> io::Result<()> {
        let mut data = Vec::with_capacity(8192);
        for path in paths {
            let path = format!("{}\n", strip_absolute_path(path.into()));
            let mut path = path.as_bytes().to_vec();
            self.index_map.push(data.len() as u64);
            data.append(&mut path);
        }
        self.queue_file
            .write()
            .await
            .get_mut()
            .write_all(data.as_slice())
    }

    #[inline]
    pub fn remove(&mut self, index: usize) {
        self.index_map.remove(index);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.queue_file.blocking_write().get_mut().rewind().unwrap();
        self.index_map.clear();
        self.index = usize::MAX - 1;
    }

    #[inline]
    pub fn shuffle(&mut self) {
        self.index_map.shuffle(&mut thread_rng());
    }

    pub async fn current_track_name(&self) -> Option<String> {
        let seek = if self.index >= self.len() {
            self.index_map.first()?.clone()
        } else {
            self.index_map.get(self.index)?.clone()
        };

        let mut guard = self.queue_file.write().await;
        guard.seek(SeekFrom::Start(seek)).unwrap();
        let mut name = String::new();
        guard.read_line(&mut name).unwrap();
        name = name.replace("\n", "");

        Some(name)
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

    pub async fn play_next(&mut self) -> io::Result<()> {
        self.index += 1;

        if self.index >= self.len() {
            self.index = 0;
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

    pub fn get_index_from_track_name(&self, name: &str) -> Result<usize, NError> {
        let mut guard = self.queue_file.blocking_write();
        for (index, seek) in self.index_map.iter().enumerate() {
            guard.seek(SeekFrom::Start(*seek)).unwrap();
            let mut track = String::new();
            guard.read_line(&mut track).unwrap();
            track = track.replace("\n", "");
            if remove_ext(track) == name {
                return Ok(index);
            }
        }

        Err(NError::NoTrack)
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
