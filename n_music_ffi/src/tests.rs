use super::*;
use n_music_core::source::Locator;
use std::time::Instant;

const WAIT: Duration = Duration::from_secs(30);

/// A second of silence, as a 16-bit mono WAV file.
fn write_wav(path: &Path) {
    let rate: u32 = 8000;
    let data_len = rate * 2;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes()); // bytes per second
    bytes.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.resize(44 + data_len as usize, 0);
    std::fs::write(path, bytes).unwrap();
}

/// Takes events as Kotlin does until `wanted` picks one; panics after [`WAIT`].
fn wait_for<T>(core: &Core, mut wanted: impl FnMut(&CoreEvent) -> Option<T>) -> T {
    let deadline = Instant::now() + WAIT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let event = core.events.recv_timeout(left).expect("the core went quiet");
        if let Some(found) = wanted(&core.taken(event)) {
            return found;
        }
    }
}

#[test]
fn reports_a_scanned_folder_and_answers_queries() {
    let data = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let music = tempfile::tempdir().unwrap();
    write_wav(&music.path().join("silence.wav"));
    let core = Core::launch(data.path(), cache.path());

    let root = Locator::Local(music.path().to_string_lossy().into_owned());
    core.send(Command::SetLibraryRoots {
        roots: vec![root.clone()],
    });
    wait_for(&core, |event| match event {
        CoreEvent::LibraryRootsChanged { roots } if *roots == [root.clone()] => Some(()),
        _ => None,
    });

    // Placeholders come first; the track is complete once its metadata loaded.
    let deadline = Instant::now() + WAIT;
    let track = loop {
        let tracks = core.tracks(Query::library(), 0, u32::MAX);
        if let [track] = tracks.as_slice() {
            if track.length > 0.0 {
                break track.clone();
            }
        }
        assert!(
            Instant::now() < deadline,
            "the track never loaded: {tracks:?}"
        );
        wait_for(&core, |event| {
            matches!(event, CoreEvent::LibraryChanged { .. }).then_some(())
        });
    };
    assert_eq!(track.title, "silence");
    assert!((track.length - 1.0).abs() < 0.01, "length {}", track.length);
    assert_eq!(core.count(Query::library()), 1);
    assert!(core.tracks(Query::library(), 1, 10).is_empty());

    core.send(Command::SetShuffle { enabled: true });
    wait_for(&core, |event| {
        matches!(event, CoreEvent::ShuffleChanged { enabled: true }).then_some(())
    });

    assert_eq!(core.setting(String::from("android.ui")), None);
    core.set_setting(
        String::from("android.ui"),
        String::from(r#"{"theme": "Dark", "locale": "it"}"#),
    );
    core.set_setting(String::from("android.ui.broken"), String::from("{"));
    let stored: serde_json::Value =
        serde_json::from_str(&core.setting(String::from("android.ui")).unwrap()).unwrap();
    assert_eq!(stored, serde_json::json!({"theme": "Dark", "locale": "it"}));
    assert_eq!(core.setting(String::from("android.ui.broken")), None);
}

#[test]
fn library_changes_come_one_at_a_time() {
    let (events, receiver) = flume::unbounded();
    let pending = Arc::new(AtomicBool::new(false));
    let read = Arc::new(AtomicU64::new(0));
    let bridge = KotlinBridge::new(events, Library::default(), pending.clone(), read.clone());
    let data = tempfile::tempdir().unwrap();
    let core = Core {
        writer: EventWriter::channel().0,
        events: receiver,
        library: Library::default(),
        library_changed: pending,
        scan_read: read.clone(),
        storage: Arc::new(JsonFileStorage::open(settings_path(data.path()))),
    };

    bridge.library_changed();
    bridge.library_changed();
    assert_eq!(core.events.len(), 1, "a burst queues a single change");

    read.store(7, Ordering::Release);
    let first = core.events.try_recv().unwrap();
    assert!(
        matches!(core.taken(first), CoreEvent::LibraryChanged { read: 7 }),
        "the count is the one when Kotlin takes the change"
    );
    bridge.library_changed();
    assert_eq!(
        core.events.len(),
        1,
        "a change after Kotlin took the last one is announced again"
    );
}
