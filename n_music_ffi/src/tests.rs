use super::*;
use bridge::KotlinBridge;
use n_event_bus::EventWriter;
use n_music_core::library::catalog::Library;
use n_music_core::library::query::{Filter, SortField};
use n_music_core::library::query::{PlaylistId, Query};
use n_music_core::library::LibraryPaths;
use n_music_core::queue::ItemId;
use n_music_core::settings::JsonFileStorage;
use n_music_core::settings::OutputDevice;
use n_music_core::source::telegram::TelegramCredentials;
use n_music_core::source::telegram::{TelegramError, TelegramStatus};
use n_music_core::source::Locator;
use n_music_core::source::Providers;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
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
    // No libraries to start with, rather than the default one in the home folder.
    std::fs::write(
        settings_path(data.path()),
        r#"{"core.library": {"libraries": []}}"#,
    )
    .unwrap();
    let core = Core::launch(data.path(), cache.path(), data.path(), None);

    let root = Locator::Local(music.path().to_string_lossy().into_owned());
    core.send(Command::SetLibraryRoots {
        roots: vec![root.clone()],
    });
    let mut telegram = false;
    wait_for(&core, |event| match event {
        CoreEvent::TelegramStatusChanged { .. } => {
            telegram = true;
            None
        }
        CoreEvent::LibraryRootsChanged { roots } if *roots == [root.clone()] => Some(()),
        _ => None,
    });
    assert!(
        !telegram,
        "without Telegram's credentials, there is no Telegram"
    );

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
    assert!(track.loaded);
    assert_eq!(core.track(track.locator.clone()), Some(track.clone()));

    let everything = Filter::All(vec![]);
    let summary = core.summary(everything.clone());
    assert_eq!(summary.tracks, 1);
    assert!((summary.length - 1.0).abs() < 0.01);
    let albums = core.albums(everything.clone(), String::new(), GroupSort::Name);
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].name, None, "the untagged track has no album");
    let [source] = core.sources(vec![root.clone()]).try_into().unwrap();
    assert_eq!(
        (source.tracks, source.listed, source.reachable),
        (1, true, true)
    );
    assert!(core.missing_tracks(root.clone()).is_empty());
    assert_eq!(
        core.facets().codecs.first().map(|codec| codec.tracks),
        Some(1)
    );
    assert!(!core.playback_options().resume);
    assert!(
        core.playback_options().match_rate,
        "the device follows the track's rate unless that was turned off"
    );

    // Turning it off never switches the device: nothing plays here anyway.
    core.send(Command::SetMatchRate { enabled: false });
    core.send(Command::SetShuffle { enabled: true });
    wait_for(&core, |event| {
        matches!(event, CoreEvent::ShuffleChanged { enabled: true }).then_some(())
    });
    assert!(
        !core.playback_options().match_rate,
        "the player saved the choice before it handled the next command"
    );

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
    let bridge = KotlinBridge::new(
        events,
        Library::default(),
        pending.clone(),
        read.clone(),
        state::Shared::default(),
    );
    let data = tempfile::tempdir().unwrap();
    let core = Core {
        writer: EventWriter::channel().0,
        events: receiver,
        library: Arc::new(OnceLock::new()),
        library_changed: pending,
        scan_read: read.clone(),
        storage: Arc::new(JsonFileStorage::open(settings_path(data.path()))),
        paths: LibraryPaths::new(data.path(), data.path()),
        providers: Providers::default(),
        state: state::Shared::default(),
        first_run: Arc::new(AtomicBool::new(false)),
        data_dir: data.path().to_path_buf(),
        bus: Mutex::new(None),
        stream_cache: Arc::new(Default::default()),
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

#[test]
fn plays_once_listed_when_play_comes_first() {
    let data = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let music = tempfile::tempdir().unwrap();
    write_wav(&music.path().join("silence.wav"));
    let root = Locator::Local(music.path().to_string_lossy().into_owned());
    std::fs::write(
        settings_path(data.path()),
        // Silent and at the device's own rate, whatever plays.
        serde_json::json!({
            "core.library": { "libraries": [root] },
            "core.playback": { "volume": 0.0, "match_rate": false },
        })
        .to_string(),
    )
    .unwrap();
    let core = Core::launch(data.path(), cache.path(), data.path(), None);
    assert!(!core.playback_options().match_rate);

    // As a headset's Play right after launch, before the scan listed the library.
    core.send(Command::Play);
    let title = wait_for(&core, |event| match event {
        CoreEvent::TrackChanged { track, .. } => Some(track.title.clone()),
        _ => None,
    });
    assert_eq!(title, "silence");
}

/// Where signing in to Telegram is, as the next `TelegramStatusChanged` tells it.
fn telegram_status(core: &Core) -> (TelegramStatus, bool, Option<TelegramError>) {
    wait_for(core, |event| match event {
        CoreEvent::TelegramStatusChanged {
            status,
            busy,
            error,
        } => Some((status.clone(), *busy, error.clone())),
        _ => None,
    })
}

#[test]
fn signs_in_to_telegram_with_commands() {
    let data = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    std::fs::write(
        settings_path(data.path()),
        r#"{"core.library": {"libraries": []}}"#,
    )
    .unwrap();
    let credentials = TelegramCredentials::new(Some("1"), Some("hash"));
    let core = Core::launch(data.path(), cache.path(), private.path(), credentials);

    assert_eq!(
        telegram_status(&core),
        (TelegramStatus::SignedOut, false, None),
        "the core tells where signing in is as it starts"
    );
    assert!(private.path().join("telegram.db").exists());
    assert!(!data.path().join("telegram.db").exists());

    // A number Telegram can't have fails before anything is sent to it.
    core.send(Command::TelegramSignIn {
        phone: String::from("+39 12"),
    });
    assert_eq!(
        telegram_status(&core),
        (TelegramStatus::SignedOut, true, None)
    );
    assert_eq!(
        telegram_status(&core),
        (
            TelegramStatus::SignedOut,
            false,
            Some(TelegramError::PhoneInvalid)
        )
    );

    core.send(Command::FindTelegramChats {
        query: String::from("jazz"),
    });
    let found = wait_for(&core, |event| match event {
        CoreEvent::TelegramChatsFound {
            query,
            chats,
            error,
        } => Some((query.clone(), chats.len(), error.clone())),
        _ => None,
    });
    assert_eq!(
        found,
        (String::from("jazz"), 0, Some(TelegramError::SignedOut))
    );

    core.send(Command::TelegramSignOut);
    assert_eq!(
        telegram_status(&core),
        (TelegramStatus::SignedOut, true, None)
    );
    assert_eq!(
        telegram_status(&core),
        (TelegramStatus::SignedOut, false, None)
    );
}

/// A core whose services never start: what it keeps itself, and commands sent to nowhere.
fn idle_core(dir: &Path) -> Core {
    Core {
        writer: EventWriter::channel().0,
        events: flume::unbounded().1,
        library: Arc::new(OnceLock::new()),
        library_changed: Arc::new(AtomicBool::new(false)),
        scan_read: Arc::new(AtomicU64::new(0)),
        storage: Arc::new(JsonFileStorage::open(settings_path(dir))),
        paths: LibraryPaths::new(dir, dir),
        providers: Providers::default(),
        state: state::Shared::default(),
        first_run: Arc::new(AtomicBool::new(true)),
        data_dir: dir.to_path_buf(),
        bus: Mutex::new(None),
        stream_cache: Arc::new(Default::default()),
    }
}

/// Takes events until `done` holds; panics after [`WAIT`].
fn wait_until(core: &Core, mut done: impl FnMut(&Core) -> bool) {
    let deadline = Instant::now() + WAIT;
    while !done(core) {
        let left = deadline.saturating_duration_since(Instant::now());
        let event = core.events.recv_timeout(left).expect("the core went quiet");
        core.taken(event);
    }
}

#[test]
fn answers_what_the_desktop_pages_ask() {
    let data = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let music = tempfile::tempdir().unwrap();
    write_wav(&music.path().join("silence.wav"));
    std::fs::write(
        settings_path(data.path()),
        r#"{"core.library": {"libraries": []}}"#,
    )
    .unwrap();
    let core = Core::launch(data.path(), cache.path(), data.path(), None);
    assert!(!core.first_run(), "sources were chosen before");
    assert_eq!(core.logs_dir(), data.path().to_string_lossy());

    wait_until(&core, |core| core.library_roots().is_some());
    assert_eq!(core.library_roots(), Some(vec![]));
    let root = Locator::Local(music.path().to_string_lossy().into_owned());
    assert!(core.add_source(root.clone(), String::from("  Silence  ")));
    assert!(
        !core.add_source(root.clone(), String::new()),
        "a source is added once"
    );
    // Read, named and scanned to the end.
    wait_until(&core, |core| {
        let status = core.scan_status();
        let named = core.source_name(root.clone()) == "Silence";
        let loaded = core
            .track_entries(Query::library(), None, 0, 1)
            .first()
            .is_some_and(|entry| entry.track.loaded);
        named && loaded && !status.running && status.updated.is_some()
    });
    // The last report of a scan tells none runs, and counts nothing.
    let status = core.scan_status();
    assert_eq!((status.found, status.read), (0, 0));
    assert!(status.updating.is_empty());

    let [entry] = core
        .track_entries(Query::library(), None, 0, 10)
        .try_into()
        .unwrap();
    assert_eq!(entry.track.title, "silence");
    assert!(entry.location.ends_with("silence.wav"));
    assert_eq!((entry.genre.as_str(), entry.added), ("", None));
    let codec = entry.codec.clone().expect("the codec was read");
    assert_eq!(entry.format, codec.to_uppercase());
    assert_eq!(core.field_values(ValueField::Codec), [codec]);
    assert!(core.field_values(ValueField::Artist).is_empty());
    let locator = entry.track.locator.clone();
    assert_eq!(core.track_entry(locator.clone(), None), Some(entry.clone()));
    assert_eq!(core.index_of(Query::library(), locator.clone()), Some(0));

    let summary = core.list_summary(tracks_query(
        String::from("nothing like it"),
        String::new(),
        String::from("title"),
        None,
    ));
    assert_eq!((summary.tracks, summary.total), (0, 1));
    let summary = core.list_summary(Query::library());
    assert_eq!((summary.tracks, summary.total), (1, 1));
    assert!((summary.length - 1.0).abs() < 0.01);

    let [source] = core.source_list().try_into().unwrap();
    assert_eq!(source.root, root);
    assert_eq!(source.location, source_location(root.clone()));
    assert_eq!(source.kind, SourceType::Folder);
    assert_eq!(
        (source.name.as_str(), source.display_name.as_deref()),
        ("Silence", Some("Silence"))
    );
    assert_eq!((source.tracks, source.available), (1, true));
    assert_eq!(source.status, SourceStatus::Ready);

    // A source's page lists its tracks by its rule.
    let rule = collection_filter(
        CollectionKind::Source,
        source.location.clone(),
        String::new(),
    );
    assert_eq!(
        core.count(tracks_query(String::new(), rule, String::new(), None)),
        1
    );

    core.send(Command::CreatePlaylist {
        name: String::from("Quiet"),
        rule: None,
        sort: vec![],
        tracks: vec![locator.clone()],
    });
    let id = wait_for(&core, |event| match event {
        CoreEvent::PlaylistsChanged { playlists } => playlists.first().map(|playlist| playlist.id),
        _ => None,
    });
    assert_eq!(core.playlists_with(locator.clone()), [id]);
    let query = tracks_query(
        String::new(),
        String::new(),
        String::from("-added"),
        Some(id),
    );
    assert_eq!(
        query.sort.len(),
        1,
        "a playlist sorts by when tracks were added"
    );
    let [entry] = core
        .track_entries(query, Some(id), 0, 10)
        .try_into()
        .unwrap();
    assert!(entry.added.is_some());

    assert!(core.remove_source(root.clone()));
    assert!(!core.remove_source(root));
    wait_until(&core, |core| core.count(Query::library()) == 0);
    assert!(core.source_list().is_empty());
}

#[test]
fn keeps_what_the_core_reported_and_the_choices_made() {
    let data = tempfile::tempdir().unwrap();
    let core = idle_core(data.path());

    // The sources are changed once the library told what they are.
    let root = Locator::Local(String::from("/music"));
    assert!(!core.add_source(root.clone(), String::new()));
    assert!(core.first_run());
    core.set_sources(
        vec![
            root.clone(),
            root.clone(),
            Locator::Web(String::from("https://a.b/c.m3u")),
        ],
        vec![String::from("Mine")],
    );
    assert!(!core.first_run(), "choosing the sources ends the first run");
    assert_eq!(core.library_roots().map(|roots| roots.len()), Some(2));
    assert!(!core.add_source(root.clone(), String::new()));
    assert!(core.remove_source(root.clone()));
    assert!(core.add_source(root, String::new()));

    assert!(core.scan_status().running, "the core starts with a scan");
    assert_eq!(core.telegram_state(), None);
    assert_eq!(core.stream_cache(), None);

    assert_eq!(core.output_device(), None);
    assert!(core.output_choices().is_empty());
    core.choose_output_device(Some(String::from("usb")));
    let usb = OutputDevice {
        id: String::from("usb"),
        name: String::from("usb"),
    };
    assert_eq!(core.output_device(), Some(usb.clone()));
    assert_eq!(
        core.output_choices(),
        [OutputChoice {
            device: usb.clone(),
            connected: true
        }],
        "before devices are listed, the chosen one shows as there"
    );
    let speakers = OutputDevice {
        id: String::from("speakers"),
        name: String::from("Speakers"),
    };
    core.state.lock().devices = Some(vec![speakers.clone()]);
    assert_eq!(
        core.output_choices(),
        [
            OutputChoice {
                device: usb,
                connected: false
            },
            OutputChoice {
                device: speakers.clone(),
                connected: true
            }
        ]
    );
    core.choose_output_device(Some(String::from("speakers")));
    assert_eq!(
        core.output_device(),
        Some(speakers.clone()),
        "named as listed"
    );
    core.send(Command::SetOutputDevice { device: None });
    assert_eq!(core.output_device(), None);
    assert_eq!(core.output_choices().len(), 1);
}

#[test]
fn keeps_the_desktop_preferences_where_the_qt_app_does() {
    let data = tempfile::tempdir().unwrap();
    std::fs::write(
        settings_path(data.path()),
        serde_json::json!({"desktop.ui": {
            "theme": "Dark",
            "locale": "it",
            "window_size": {"width": 1400, "height": 900, "maximized": true},
            "save_window_size": true,
            "accent": "teal",
            "hidden_columns": ["genre", "lastPlayed"],
            "kept": 7
        }})
        .to_string(),
    )
    .unwrap();
    let core = idle_core(data.path());

    let ui = core.desktop_ui();
    assert_eq!(ui.theme, UiTheme::Dark);
    assert_eq!(ui.locale.as_deref(), Some("it"));
    assert_eq!(
        (ui.window_width, ui.window_height, ui.window_maximized),
        (1400, 900, true)
    );
    assert_eq!(ui.accent, "teal");
    assert_eq!(ui.hidden_columns, ["genre", "lastPlayed"]);
    assert!(
        ui.check_updates && ui.mini_on_top,
        "missing ones are at their defaults"
    );

    core.set_desktop_ui(DesktopUi {
        theme: UiTheme::Light,
        locale: None,
        reduce_motion: true,
        ..ui
    });
    let stored = core.setting(String::from("desktop.ui")).unwrap();
    let stored: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored["theme"], "Light");
    assert_eq!(stored["locale"], serde_json::Value::Null);
    assert_eq!(stored["reduce_motion"], true);
    assert_eq!(stored["window_size"]["width"], 1400);
    assert_eq!(stored["kept"], 7, "what else the section holds stays");
    assert_eq!(core.desktop_ui().theme, UiTheme::Light);

    assert_eq!(
        core.setting_field(String::from("desktop.ui"), String::from("accent")),
        Some(String::from("\"teal\""))
    );
    core.set_setting_field(
        String::from("macos.ui"),
        String::from("sidebar"),
        String::from("false"),
    );
    core.set_setting_field(
        String::from("macos.ui"),
        String::from("broken"),
        String::from("{"),
    );
    assert_eq!(
        core.setting(String::from("macos.ui")),
        Some(String::from(r#"{"sidebar":false}"#))
    );
    assert_eq!(
        core.setting_field(String::from("macos.ui"), String::from("missing")),
        None
    );

    let fresh = tempfile::tempdir().unwrap();
    assert_eq!(idle_core(fresh.path()).desktop_ui(), default_desktop_ui());
}

#[test]
fn reads_and_writes_the_filter_editors_rules() {
    let json = r#"{"match": "any", "rules": [
        {"field": "genre", "op": "is", "value": " J-Pop "},
        {"field": "artist", "op": "is_not", "value": "Neru"},
        {"field": "artist", "op": "is", "value": "  "},
        {"field": "year", "op": "between", "from": "1990", "to": 1999},
        {"field": "plays", "op": "at_least", "from": "5", "to": "9"},
        {"field": "played_within", "op": "the_last", "amount": "2", "unit": "weeks"},
        {"field": "not_played_within", "op": "the_last", "amount": "0", "unit": "days"},
        {"field": "source", "op": "is", "value": "https://a.b/c.m3u"},
        {"field": "playlist", "op": "not_in", "value": "4"},
        {"group": "none", "rules": [{"field": "codec", "op": "is", "value": "mp3"}]},
        {"group": "all", "rules": [{"field": "album", "op": "is", "value": ""}]}
    ]}"#;
    let filter = parse_filter(json.to_string()).unwrap();
    let not = |filter: Filter| Filter::Not(Box::new(filter));
    assert_eq!(
        filter,
        Filter::Any(vec![
            Filter::Genre(String::from("J-Pop")),
            not(Filter::Artist(String::from("Neru"))),
            Filter::Year {
                from: Some(1990),
                to: Some(1999)
            },
            Filter::Plays {
                min: Some(5),
                max: None
            },
            Filter::PlayedWithin {
                seconds: 14 * 24 * 60 * 60
            },
            Filter::Library(Locator::Web(String::from("https://a.b/c.m3u"))),
            not(Filter::Playlist(PlaylistId(4))),
            not(Filter::Any(vec![Filter::Codec(String::from("mp3"))])),
        ]),
        "rules missing a value, periods of no time and empty groups are dropped"
    );
    let spec = filter_spec(filter.clone()).unwrap();
    assert_eq!(parse_filter(spec), Some(filter));
    assert_eq!(
        parse_filter(String::from(r#"{"match": "all", "rules": []}"#)),
        None
    );
    assert_eq!(parse_filter(String::from("not json")), None);
    assert_eq!(
        filter_spec(Filter::All(vec![Filter::All(vec![Filter::Any(vec![])])])),
        None,
        "groups in groups can't be edited"
    );
    assert_eq!(
        filter_spec(not(Filter::Year {
            from: Some(1),
            to: None
        })),
        None
    );

    let sort = parse_sort(String::from("-year, album,added,nonsense"), None);
    assert_eq!(sort_string(sort.clone()), "-year,album");
    let sort = parse_sort(String::from("-added,lastPlayed"), Some(PlaylistId(3)));
    assert_eq!(sort[0].field, SortField::Added(PlaylistId(3)));
    assert_eq!(sort_string(sort), "-added,lastPlayed");

    let query = tracks_query(
        String::from("  blue "),
        String::from(r#"{"rules": [{"field": "genre", "op": "is", "value": "Jazz"}]}"#),
        String::from("artist,album"),
        Some(PlaylistId(2)),
    );
    assert_eq!(
        query.filter,
        Filter::All(vec![
            Filter::Playlist(PlaylistId(2)),
            Filter::Search(String::from("blue")),
            Filter::All(vec![Filter::Genre(String::from("Jazz"))]),
        ])
    );
    assert_eq!(query.sort.len(), 2);

    assert_eq!(
        parse_filter(collection_filter(
            CollectionKind::Album,
            String::from("Paper"),
            String::from("Mira")
        )),
        Some(Filter::All(vec![
            Filter::Album(String::from("Paper")),
            Filter::Any(vec![
                Filter::AlbumArtist(String::from("Mira")),
                Filter::Artist(String::from("Mira")),
            ]),
        ]))
    );
    assert_eq!(
        parse_filter(collection_filter(
            CollectionKind::Genre,
            String::new(),
            String::new()
        )),
        Some(Filter::All(vec![Filter::Untagged(
            n_music_core::library::query::Tag::Genre
        )]))
    );

    for root in [
        Locator::Local(String::from("/home/me/Music")),
        Locator::Web(String::from("https://a.b/c.m3u")),
        Locator::TelegramChat(String::from("telegram:-100123")),
    ] {
        let location = source_location(root.clone());
        assert_eq!(source_from_location(location), root);
    }
    assert_eq!(
        normalize_web_address(String::from(" a.b/c.pls ")),
        Some(Locator::Web(String::from("https://a.b/c.pls")))
    );
    assert_eq!(normalize_web_address(String::from("ftp://a.b/c.pls")), None);
}

fn queue_row(item: u64, queued: bool) -> QueueRow {
    QueueRow {
        item: ItemId(item),
        queued,
        track: TrackRow::from(&n_music_core::TrackInfo::placeholder(Locator::Local(
            format!("/{item}.flac"),
        ))),
    }
}

/// The items of `rows`, marked `h` when played, `*` when current and `q` when queued.
fn marks(rows: &[QueueLayoutRow]) -> Vec<String> {
    rows.iter()
        .map(|row| {
            let mut mark = row.entry.item.0.to_string();
            if row.history {
                mark.push('h');
            }
            if row.current {
                mark.push('*');
            }
            if row.entry.queued {
                mark.push('q');
            }
            mark
        })
        .collect()
}

#[test]
fn lays_out_the_queue_and_moves_items_as_the_desktop_does() {
    let entries = vec![
        queue_row(1, false),
        queue_row(2, false),
        queue_row(3, true),
        queue_row(4, false),
        queue_row(5, true),
        queue_row(6, false),
    ];
    let layout = queue_layout(entries.clone(), Some(ItemId(2)), true, true);
    assert_eq!(marks(&layout.rows), ["1h", "2*", "3q", "4", "5q", "6"]);
    assert_eq!(
        (
            layout.history,
            layout.up_next,
            layout.left,
            layout.current_row
        ),
        (1, 2, 4, Some(1))
    );
    let hidden = queue_layout(entries.clone(), Some(ItemId(2)), false, false);
    assert_eq!(marks(&hidden.rows), ["3q", "4", "5q", "6"]);
    assert_eq!((hidden.history, hidden.current_row), (1, None));

    // Played items stay where they are.
    assert_eq!(queue_move(layout.rows.clone(), 0, 3), None);
    assert_eq!(queue_move(layout.rows.clone(), 3, 3), None);

    // An upcoming item moves among the others.
    let moved = queue_move(layout.rows.clone(), 5, 2).unwrap();
    assert_eq!(
        (moved.item, moved.before, moved.current),
        (ItemId(6), Some(ItemId(3)), false)
    );
    assert_eq!(marks(&moved.rows), ["1h", "2*", "6", "3q", "4", "5q"]);

    // Above the current item, it counts as played and is no longer queued.
    let moved = queue_move(layout.rows.clone(), 4, 0).unwrap();
    assert_eq!(moved.before, Some(ItemId(1)));
    assert_eq!(marks(&moved.rows), ["5h", "1h", "2*", "3q", "4", "6"]);

    // The current item going down: the context's items it passes count as played, the queued
    // ones play right after it.
    let moved = queue_move(layout.rows.clone(), 1, 4).unwrap();
    assert_eq!(
        (moved.item, moved.before, moved.current),
        (ItemId(2), Some(ItemId(6)), true)
    );
    assert_eq!(marks(&moved.rows), ["1h", "4h", "2*", "3q", "5q", "6"]);

    // Going up, those it passes play again after it.
    let layout = queue_layout(entries.clone(), Some(ItemId(4)), true, true);
    assert_eq!(marks(&layout.rows), ["1h", "2h", "3hq", "4*", "5q", "6"]);
    let moved = queue_move(layout.rows.clone(), 3, 1).unwrap();
    assert_eq!(moved.before, Some(ItemId(2)));
    assert_eq!(marks(&moved.rows), ["1h", "4*", "2", "3q", "5q", "6"]);

    // Past the end, it goes last.
    let moved = queue_move(layout.rows, 4, 99).unwrap();
    assert_eq!((moved.item, moved.before), (ItemId(5), None));
}

#[test]
fn names_telegram_errors_as_the_desktop_strings_do() {
    assert_eq!(
        telegram_error_text(TelegramError::Wait { seconds: 61 }),
        TelegramErrorText {
            key: String::from("telegram_error_wait"),
            detail: String::from("2")
        }
    );
    assert_eq!(
        telegram_error_text(TelegramError::Failed(String::from("no"))).detail,
        "no"
    );
    assert_eq!(
        telegram_error_text(TelegramError::CodeInvalid).key,
        "telegram_error_code_invalid"
    );
    assert_eq!(
        telegram_chat_kind_key(n_music_core::source::telegram::TelegramChatKind::SavedMessages),
        "telegram_kind_saved"
    );
}

#[test]
fn shutdown_waits_for_the_core_and_writes_the_settings() {
    let data = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(
        settings_path(data.path()),
        r#"{"core.library": {"libraries": []}, "core.playback": {"volume": 0.0}}"#,
    )
    .unwrap();
    let core = Core::launch(data.path(), cache.path(), data.path(), None);
    // As the window size the app saves as its window closes, right before it quits: the
    // settings writer would only write it 500 ms later.
    core.set_setting(
        String::from("macos.ui"),
        String::from(r#"{"window":[1280,800]}"#),
    );

    // Twice at once, as a quit from the menu racing one from the last window closing: both
    // return once the core is down.
    std::thread::scope(|scope| {
        scope.spawn(|| core.shutdown());
        core.shutdown();
    });

    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(settings_path(data.path())).unwrap())
            .unwrap();
    assert_eq!(saved["macos.ui"]["window"][0], 1280);
    assert_eq!(saved["core.playback"]["volume"], 0.0, "the rest stays");
    // The services are gone: no more events, and commands go nowhere.
    let left = core.events.drain().count();
    assert!(
        matches!(
            core.events.recv_timeout(Duration::from_millis(100)),
            Err(flume::RecvTimeoutError::Disconnected)
        ),
        "the events end once the core is down ({left} were left)"
    );
    core.send(Command::PlayNext);
    core.shutdown();

    // Settings stored after the shutdown are written at once, without the writer thread.
    core.set_setting(
        String::from("macos.ui"),
        String::from(r#"{"window":[900,600]}"#),
    );
    let saved = std::fs::read_to_string(settings_path(data.path())).unwrap();
    assert!(saved.contains("900"), "{saved}");
}
