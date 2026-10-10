# N Music for Android

The Android app is a Jetpack Compose frontend over the Rust core. This file covers how to build
it, how Kotlin reaches the core, and what keeps playback going while the app is in the
background.

## Building

You need:

- the Android SDK with platform 36, build-tools 35.0.0 and NDK 28.2.13676358
- `rustup target add aarch64-linux-android` and `cargo install cargo-ndk --version 4.1.2 --locked`
- clang and mold on the host, which the repository's cargo config links host binaries with

Then, from this folder:

```shell
./compile_rust.sh
./gradlew :android:assembleDebug
```

`compile_rust.sh` builds `n_music_ffi` for arm64 into `android/build/rustJniLibs`, then runs
`generate_bindings.sh`, which writes the matching Kotlin bindings into
`android/build/generated/uniffi`. Gradle compiles both into the app, so run the script again
whenever the Rust side changes. `compile_rust_release.sh` does the same with the release profile.

## Screens

The app follows the N Music Android design: Material 3 Expressive, the Figtree typeface,
graphite surfaces and a choice of accent colours. It has:

- the library's Tracks, Albums, Artists and Genres tabs, each as a list or a grid, with sorts,
  filters, a fast scroller, search, and album and artist pages
- selection: a long press picks tracks to play next, queue or add to a playlist
- the mini player, which opens into the now playing page, with the queue, a sleep timer that
  fades playback out, and Android's output switcher
- playlists, plain and smart, with rules, sorting and removals you can undo
- sources: local folders and web playlists, a page for each, and the first run's welcome
- settings for playback, the theme and accent, the mini player's buttons and the language, and
  the about page
- the player and Shuffle everything home screen widgets, and the media notification
- layouts for foldables and tablets: a rail or a drawer, the queue beside the player, a now
  playing pane, and tracks in a table

Each area has its package under `ui`: `library`, `player`, `playlists`, `sources` and
`settings`, with `sheets` and `dialogs` for what they open. `ui/components` holds the pieces
they share. `ui/theme` holds the colours, type, shapes and icons, and `NMotion`, the springs
every animation runs on: Material 3 Expressive's six, plus one without bounce for screen-sized
edges. The long-press fill, the bars beside the playing track and the seek bar's wave run on a
clock instead. With Android's Remove animations setting on, every animation jumps to its end and
the wave stands still.

The app's own settings, like the theme, the accent, the mini player's buttons and each list's
view and sort, stay in the core's settings file, in the `android.ui` section the Slint app used,
so its theme carries over. The strings are Android resources in `android/src/main/res/values`.

## How Kotlin reaches the core

- `n_music_ffi` exports the core with UniFFI. `Core.start` reads the settings and starts the core
  services on their own bus thread, which opens the library database and the saved session, so
  the main thread does not wait for them. `Core.send` emits the message a `Command` stands for,
  and `Core.nextEvent` suspends until the core announces something. `tracks`, `count` and the
  lists of albums, artists, genres, playlists and sources read the library directly, off the
  bus, like the desktop's worker. `playbackOptions` and `setting` read the settings file, for
  what the core announces no event for.
- A `KotlinBridge` subscriber copies each event into a `CoreEvent` and queues it, so the bus
  thread never waits on Kotlin. A scan reports every track it reads; the bridge folds those into
  one `LibraryChanged` until Kotlin takes it.
- `NativeLibrary.init(context)` is the one hand-written JNI call left. It hands the JVM and the
  application context to the native side: cpal needs them to list output devices, and the
  Storage Access Framework provider needs them to read the picked folder.
- `CoreRepository` reads `nextEvent` in one loop and keeps a `StateFlow` for each piece of
  state. The Compose screens collect those flows, and so does `NPlayer`.
- `NPlayer` is a Media3 `SimpleBasePlayer` that mirrors the core for the media session. It
  never decodes audio.
- If the bus thread panics, the core ends the process, and `CoreRepository` does the same if
  `nextEvent` ever returns nothing. Screens and a notification stuck on a core that is gone, with
  the wake lock held, are worse than Android starting the app again.
- Rust's stdout and stderr show in logcat under the tag `RustStdoutStderr`, as they did in the
  Slint app: a copy of every log line, and panic messages. The log files stay in
  `<external files>/config`.

## Background playback

### What runs where

The core lives in the process, not in an activity. `NMusicApplication.onCreate` starts it, and
that runs on every process start, whether Android started the process for the UI, for
`PlaybackService` or for a media button. The Slint app ran the core inside `android_main`, so it
lived and died with the activity.

What keeps the process alive is `PlaybackService`, a Media3 `MediaSessionService`:

- Media3 runs it in the foreground, with the media notification, while `NPlayer` reports
  playing. The notification shows from the first play on, as in the Slint app. When the user
  dismisses it, `NPlayer` reports idle, and the notification stays away until playback starts
  again.
- After a pause it stays in the foreground for 10 more minutes, so the notification or a headset
  can resume playback without running into Android's limits on starting a foreground service
  from the background. `setForegroundServiceTimeoutMs` changes that window. Media3 does this
  since 1.6; the app uses 1.9.4.
- When the user swipes the app away from recents, Media3's default `onTaskRemoved` keeps the
  service if playback is ongoing and stops it otherwise.
- While the UI shows, `MainActivity` keeps a `MediaController` connected, so the service already
  exists when playback starts from the app.

While playing, `NPlayer` also does what ExoPlayer does for itself, with the helpers Media3 1.9
made public:

- `WakeLockManager` holds a partial wake lock, so the native decoder keeps running with the
  screen off.
- `AudioFocusManager` takes audio focus whenever playback starts, from the app, the notification
  or a headset. It pauses on a permanent loss, and pauses and then resumes around a passing one
  like a call. When another app only asks to duck, like a navigation prompt, Android lowers the
  volume by itself, so the core's volume, which is the user's setting, never changes. `NPlayer`
  asks once the core plays rather than before: by then Media3 has put the service in the
  foreground, which Android 15 requires of an app in the background before it grants focus.
- `AudioBecomingNoisyManager` pauses when headphones are unplugged, also while paused for a call,
  so the music does not come back on the speaker after it. Without it the core would follow the
  device change and carry on through the speaker.

### Suspend

Once playback stops and the service leaves the foreground, the process is an ordinary cached
one:

- From Android 14, Android freezes a cached process 10 seconds after it becomes cached. No
  thread runs, the bus thread and the decoders included, until something wakes the process: an
  intent from a media button, the user reopening the app. Older versions do the same on devices
  that turn the freezer on, every Pixel among them.
- Android can kill a cached process at any time to reclaim memory. Nothing runs at that point:
  no `ShutdownRequested`, no destructors.

So whatever must survive has to be saved as it happens, which is what the core does.

### Resume

Within one process nothing is lost. The core keeps its state, and a new activity or a recreated
`PlaybackService` reads the current values from `CoreRepository`'s flows.

After the process died, the core reopens the play session from the library database when its
resume setting, `SetResume`, is on. It saves the session on pause, on every track change, at the
end, and every 10 seconds of playback. Those periodic saves ride on position reports, and the
core used to stop reporting positions while the app was hidden. A session killed in the
background then resumed from wherever the app was last on screen. Hidden, the core now still
reports positions every 5 seconds: no screen shows them, but the save needs them, and a killed
session resumes within about 10 seconds of where it was.

A headset or the notification can start playback with no process running. `MediaButtonReceiver`
starts `PlaybackService` as a foreground service, and the play press reaches `NPlayer`, which
leaves idle right away: Media3 then puts the service in the foreground, which Android requires
within seconds. The core resumes its saved session, or plays the whole library when nothing was
chosen yet; a Play that comes before the startup scan listed the library waits for the listing.
Media3 only calls `onPlaybackResumption` to start playback when the player is empty and accepts
new media items, which `NPlayer` never does, so its implementation only describes the session to
whoever asks.

### Not done yet

- Resume is off by default in the core's settings, so a killed process comes back with nothing
  queued until the Resume switch in Settings is on.
- From Android 15 an audio focus request fails unless the app is on top or runs a foreground
  service. Playback from the app is on top. Playback from a headset or the notification with
  the app in the background asks for focus once the core plays, after Media3 started the
  foreground service. That Android grants it then still has to be checked on a device.
- Android's resumption card after a reboot needs a `MediaLibraryService`. Android 15 also
  forbids starting a `mediaPlayback` foreground service from `BOOT_COMPLETED`, so that card is
  the only way back after a reboot.
- Web playlists stream without a Wi-Fi lock. Media3's `WifiLockManager` would keep Wi-Fi up
  while a remote track plays with the screen off.

## References

- [Background playback with a MediaSessionService](https://developer.android.com/media/media3/session/background-playback)
- [Media3 release notes](https://developer.android.com/jetpack/androidx/releases/media3)
- [Manage audio focus](https://developer.android.com/media/optimize/audio-focus)
- [Cached apps freezer](https://source.android.com/docs/core/perf/cached-apps-freezer)
