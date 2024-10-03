use crate::get_image;
use crate::runner::Runner;
use flume::{Receiver, Sender};
use hashbrown::HashMap;
use image::imageops::FilterType;
use image::ImageFormat;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::RwLock;

type LoadingImages = Arc<Mutex<Vec<(usize, PathBuf)>>>;
type ImageLoadedMessage = (usize, Vec<u8>);

pub struct ImageLoader {
    loaded_images: HashMap<usize, Vec<u8>>,
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

    pub fn get(&mut self, index: usize) -> Vec<u8> {
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
            vec![]
        } else {
            vec![]
        }
    }
}

fn load_image(tx: Sender<ImageLoadedMessage>, loading_images: LoadingImages) {
    loop {
        let loading = loading_images.lock().unwrap().pop();

        if let Some(loading) = loading {
            let mut image = get_image(loading.1);
            if !image.is_empty() {
                image::load_from_memory(&image)
                    .unwrap()
                    .resize(64, 64, FilterType::Lanczos3)
                    .to_rgb8()
                    .write_to(&mut Cursor::new(&mut image), ImageFormat::Jpeg)
                    .unwrap();
            }
            tx.send((loading.0, image)).unwrap();
        }

        thread::sleep(Duration::from_millis(100));
    }
}
