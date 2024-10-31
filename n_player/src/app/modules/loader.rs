use crate::app::modules::updater::UpdaterModule;
use crate::app::modules::{Module, ModuleMessage};
use crate::app::{AppMessage, Messenger, Platform, Runner, Settings};
use crate::{add_all_tracks_to_player, get_image, FileTrack};
use flume::{Receiver, Sender, TryRecvError};
use n_audio::music_track::MusicTrack;
use n_audio::remove_ext;
use rimage::codecs::webp::WebPDecoder;
use rimage::operations::resize::{FilterType, ResizeAlg};
use std::io::Cursor;
use std::mem;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_image::image::Image;
use zune_image::traits::{DecoderTrait, OperationsTrait};
use zune_imageprocs::crop::Crop;

enum InternalLoaderMessage {
    Metadata(u16, FileTrack),
    Quit,
}

pub enum LoaderMessage {
    Load(PathBuf),
    Quit,
}

impl ModuleMessage for LoaderMessage {
    fn quit() -> Self {
        Self::Quit
    }
}

pub struct LoaderModule<P: crate::platform::Platform + Send + 'static> {
    platform: Platform<P>,
    settings: Settings,
    runner: Runner,
    messenger: Messenger<P>,
    rx: Receiver<LoaderMessage>,
    start_loading: bool,
}

impl<P: crate::platform::Platform + Send + 'static> LoaderModule<P> {
    async fn load_path(&self, path: String, check_cache: bool) -> Option<bool> {
        if let Ok(true) = tokio::fs::try_exists(&path).await {
            {
                let mut guard = self.runner.write().await;
                guard.clear().await;
                guard.set_path(path.clone());
                add_all_tracks_to_player(guard.deref_mut(), path.clone()).await;
            }
            let runner = self.runner.read().await;
            let settings = self.settings.lock().await;
            let mut tracks = Vec::with_capacity(runner.len());
            let is_cached = if check_cache {
                let check_timestamp = settings.check_timestamp().await;
                check_timestamp && !settings.tracks.is_empty()
            } else {
                false
            };
            for i in 0..runner.len() as u16 {
                let track_path = runner.get_path_for_file(i).await.unwrap();
                let track_without_ext = remove_ext(track_path.clone());
                let track = if is_cached {
                    if let Some(file_track) = settings
                        .tracks
                        .iter()
                        .find(|file_track| file_track.path == track_without_ext)
                    {
                        file_track.clone()
                    } else {
                        // If the cache is corrupted, just delete it
                        self.settings
                            .lock()
                            .await
                            .delete(self.platform.lock().await)
                            .await;
                        panic!("Music dir was modified but somehow it's still detected as cached or the config file is corrupted\nDeleted config file, try restarting");
                    }
                } else {
                    FileTrack {
                        path: track_path.to_str().unwrap().to_string(),
                        title: track_without_ext,
                        artist: "".to_string(),
                        length: 0.0,
                        image: vec![],
                    }
                };
                tracks.push(track);
            }
            self.messenger
                .send(AppMessage::UpdaterMessage(<UpdaterModule<P> as Module<
                    P,
                >>::Message::StartedLoading(
                    tracks
                )))
                .unwrap();
            Some(is_cached)
        } else {
            None
        }
    }

    async fn spawn_loader_tasks(
        rx: Receiver<InternalLoaderMessage>,
        tx: Sender<InternalLoaderMessage>,
        runner: Runner,
    ) {
        let len = runner.read().await.len();
        let mut tasks = vec![];
        let (tx_l, rx_l) = flume::unbounded();
        let rx_l = Arc::new(Mutex::new(rx_l));
        let cpus = num_cpus::get() * 4;
        for _ in 0..cpus {
            let runner = runner.clone();
            let tx = tx.clone();
            let rx_l = rx_l.clone();
            tasks.push(tokio::task::spawn(loader_task(tx, rx_l, runner)));
        }
        for i in 0..len {
            tx_l.send_async(i as u16).await.unwrap();
        }
        for _ in 0..cpus {
            tx_l.send_async(u16::MAX).await.unwrap();
        }
        loop {
            let message = rx.try_recv();
            if let Ok(InternalLoaderMessage::Quit) = message {
                for task in &tasks {
                    task.abort();
                }
            } else if let Err(TryRecvError::Disconnected) = message {
                return;
            } else {
                let mut all_finished = true;
                for task in &tasks {
                    all_finished &= task.is_finished();
                }
                if all_finished {
                    return;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl<P: crate::platform::Platform + Send + 'static> Module<P> for LoaderModule<P> {
    type Message = LoaderMessage;

    async fn setup(
        platform: Platform<P>,
        settings: Settings,
        runner: Runner,
        messenger: Messenger<P>,
        rx: Receiver<Self::Message>,
    ) -> Self {
        let path = settings.lock().await.path.clone();
        let mut loader = Self {
            platform,
            settings,
            runner,
            messenger,
            rx,
            start_loading: false,
        };
        if let Some(false) = loader.load_path(path, true).await {
            loader.start_loading = true;
        }

        loader
    }

    async fn start(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(1000));
        let mut load = self.start_loading;
        let mut handle: Option<JoinHandle<()>> = None;
        let (tx, rx_tasks) = flume::unbounded();
        let (tx_tasks, rx) = flume::unbounded();
        'exit: loop {
            interval.tick().await;
            while let Ok(message) = self.rx.try_recv() {
                match message {
                    LoaderMessage::Load(path_buf) => {
                        println!("loading path: {path_buf:?}");
                        let path_string = path_buf.to_str().unwrap().to_string();
                        if let Some(false) = self.load_path(path_string.clone(), false).await {
                            load = true;
                        }
                    }
                    LoaderMessage::Quit => {
                        break 'exit;
                    }
                }
            }
            if load {
                println!("starting to load metadata");
                if let Some(handle) = mem::take(&mut handle) {
                    if let Err(e) = tx.send_async(InternalLoaderMessage::Quit).await {
                        eprintln!("error occurred when closing loader tasks, maybe it's already closed: {e}");
                    }
                    handle.await.unwrap();
                    rx.drain();
                    println!("closed previous loaders");
                }
                handle = Some(tokio::task::spawn(Self::spawn_loader_tasks(
                    rx_tasks.clone(),
                    tx_tasks.clone(),
                    self.runner.clone(),
                )));
                println!("started loaders");
                load = false;
            }
            while let Ok(InternalLoaderMessage::Metadata(index, track)) = rx.try_recv() {
                self.messenger
                    .send(AppMessage::UpdaterMessage(<UpdaterModule<P> as Module<
                        P,
                    >>::Message::Metadata(
                        index, track
                    )))
                    .unwrap();
            }
        }
    }

    fn start_sync(&self) {
        unreachable!("LoaderModule was declared as async but the app function started it as sync")
    }
}

async fn loader_task(
    tx: Sender<InternalLoaderMessage>,
    rx: Arc<Mutex<Receiver<u16>>>,
    runner: Runner,
) {
    loop {
        if let Ok(index) = rx.lock().await.recv_async().await {
            if index == u16::MAX {
                return;
            }
            if let Some(path) = runner.read().await.get_path_for_file(index).await {
                if let Ok(track) = MusicTrack::new(path.to_string_lossy().to_string()) {
                    if let Ok(Ok(meta)) =
                        tokio::task::spawn_blocking(move || track.get_meta()).await
                    {
                        let p = path.clone();
                        let image = if let Ok(image) =
                            tokio::task::spawn_blocking(move || get_image(p)).await
                        {
                            if !image.is_empty() {
                                let zune_image = if let Ok(image) =
                                    Image::read(ZCursor::new(&image), DecoderOptions::new_fast())
                                {
                                    Some(image)
                                } else if let Ok(mut webp_decoder) =
                                    WebPDecoder::try_new(Cursor::new(&image))
                                {
                                    if let Ok(image) = webp_decoder.decode() {
                                        Some(image)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                if let Some(mut zune_image) = zune_image {
                                    zune_image.convert_color(ColorSpace::RGB).unwrap();
                                    let (width, height) = zune_image.dimensions();
                                    if width != height {
                                        let difference = width.abs_diff(height);
                                        let min = width.min(height);
                                        let is_height = height < width;
                                        let x = if is_height { difference / 2 } else { 0 };
                                        let y = if !is_height { difference / 2 } else { 0 };
                                        tokio::task::block_in_place(|| {
                                            Crop::new(min, min, x, y)
                                                .execute(&mut zune_image)
                                                .unwrap()
                                        });
                                    }
                                    tokio::task::block_in_place(|| {
                                        rimage::operations::resize::Resize::new(
                                            128,
                                            128,
                                            ResizeAlg::Convolution(FilterType::Hamming),
                                        )
                                        .execute(&mut zune_image)
                                        .unwrap()
                                    });
                                    zune_image.flatten_to_u8()[0].clone()
                                } else {
                                    vec![]
                                }
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        };

                        if let Err(e) = tx
                            .send_async(InternalLoaderMessage::Metadata(
                                index,
                                FileTrack {
                                    path: remove_ext(path),
                                    title: meta.title,
                                    artist: meta.artist,
                                    length: meta.time.length,
                                    image,
                                },
                            ))
                            .await
                        {
                            eprintln!("error happened during metadata transfer, probably because the app was closed: {e}");
                        }
                    }
                }
            }
        }
    }
}
