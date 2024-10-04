use crate::get_image;
use crate::runner::Runner;
use crate::storage::Storage;
use flume::{Receiver, Sender};
use hashbrown::HashMap;
use image::imageops::FilterType;
use image::ImageFormat;
use n_audio::remove_ext;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fs, thread};
use tokio::sync::RwLock;

type LoadingImages = Arc<Mutex<Vec<(usize, PathBuf)>>>;
type ImageLoadedMessage = (usize, PathBuf);

pub struct ImageLoader {
    loaded_images: HashMap<usize, PathBuf>,
    runner: Arc<RwLock<Runner>>,
    loading_images: LoadingImages,
    rx: Receiver<ImageLoadedMessage>,
    already_loading: Vec<usize>,
}

impl ImageLoader {
    pub fn new(runner: Arc<RwLock<Runner>>) -> Self {
        let loading_images = Arc::new(Mutex::new(vec![]));
        let (tx, rx) = flume::unbounded();
        for _ in 0..num_cpus::get() {
            let tx = tx.clone();
            let loading_images = loading_images.clone();
            thread::spawn(move || load_image(tx, loading_images));
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
                .lock()
                .unwrap()
                .push((index, self.runner.blocking_read().get_path_for_file(index)));
            PathBuf::new()
        } else {
            PathBuf::new()
        }
    }
}

fn load_image(tx: Sender<ImageLoadedMessage>, loading_images: LoadingImages) {
    loop {
        let loading = loading_images.lock().unwrap().pop();

        if let Some(loading) = loading {
            let mut image = get_image(loading.1.as_path());
            if !image.is_empty() {
                image::load_from_memory(&image)
                    .unwrap()
                    .resize(128, 128, FilterType::Lanczos3)
                    .to_rgb8()
                    .write_to(&mut Cursor::new(&mut image), ImageFormat::Jpeg)
                    .unwrap();
            }
            let images_dir = Storage::app_dir().join("images");
            if !images_dir.exists() {
                fs::create_dir(images_dir.as_path()).unwrap();
            }
            let path = images_dir.join(format!("{}.jpg", remove_ext(loading.1)));
            fs::write(path.as_path(), image).unwrap();
            tx.send((loading.0, path)).unwrap();
        }

        thread::sleep(Duration::from_millis(200));
    }
}
