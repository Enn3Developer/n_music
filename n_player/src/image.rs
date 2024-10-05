use crate::get_image;
use crate::runner::Runner;
use crate::storage::Storage;
use flume::{Receiver, Sender};
use hashbrown::HashMap;
use image::imageops::FilterType;
use image::ImageFormat;
use n_audio::remove_ext;
use pollster::FutureExt;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{fs, thread};
use tokio::sync::RwLock;

type ImageLoadedMessage = (usize, PathBuf);

pub struct ImageLoader {
    loaded_images: HashMap<usize, PathBuf>,
    runner: Arc<RwLock<Runner>>,
    loading_images: Sender<(usize, PathBuf)>,
    rx: Receiver<ImageLoadedMessage>,
    already_loading: Vec<usize>,
}

impl ImageLoader {
    pub fn new(runner: Arc<RwLock<Runner>>) -> Self {
        let (loading_images, thread_rx) = flume::unbounded();
        let (tx, rx) = flume::unbounded();
        let thread_rx = Arc::new(Mutex::new(thread_rx));
        for _ in 0..num_cpus::get() {
            let thread_rx = thread_rx.clone();
            let tx = tx.clone();
            thread::spawn(move || load_image(tx, thread_rx));
        }
        Self {
            runner,
            loading_images,
            rx,
            loaded_images: HashMap::new(),
            already_loading: vec![],
        }
    }

    pub fn get(&mut self, index: usize) -> PathBuf {
        while let Ok(loaded_image) = self.rx.try_recv() {
            self.loaded_images.insert(loaded_image.0, loaded_image.1);
        }

        if self.loaded_images.contains_key(&index) {
            self.loaded_images.get(&index).unwrap().clone()
        } else if !self.already_loading.contains(&index) {
            self.already_loading.push(index);
            self.loading_images
                .send((
                    index,
                    self.runner
                        .blocking_read()
                        .get_path_for_file(index)
                        .block_on(),
                ))
                .unwrap();
            PathBuf::new()
        } else {
            PathBuf::new()
        }
    }
}

fn load_image(tx: Sender<ImageLoadedMessage>, rx: Arc<Mutex<Receiver<(usize, PathBuf)>>>) {
    loop {
        let loading = rx.lock().unwrap().recv();

        if let Ok(loading) = loading {
            let mut image = get_image(loading.1.as_path());
            let path = if !image.is_empty() {
                image::load_from_memory(&image)
                    .unwrap()
                    .resize(128, 128, FilterType::Lanczos3)
                    .to_rgb8()
                    .write_to(&mut Cursor::new(&mut image), ImageFormat::Jpeg)
                    .unwrap();

                let images_dir = Storage::app_dir().join("images");
                if !images_dir.exists() {
                    fs::create_dir(images_dir.as_path()).unwrap();
                }
                let path = images_dir.join(format!("{}.jpg", remove_ext(loading.1)));
                fs::write(path.as_path(), image).unwrap();
                path
            } else {
                PathBuf::new().join("thisdoesntexistsodontworryaboutit")
            };
            tx.send((loading.0, path)).unwrap();
        }
    }
}
