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

Gradle needs a UTF-8 locale, `LANG=C.UTF-8` for example: one translation in `assets/lang` has a
file name that is not ASCII.

## The interim screens

Until the new design lands, the app shows the Slint app's two screens, ported to Compose with its
palette, icons and translations:

- The main screen lists the library in play order, with search, a button that scrolls to the
  playing track, the scan's progress, and the control panel.
- Settings has the theme, the music folder, a rescan, the language, and the version, credits and
  license.

The theme and the language stay in the core's settings file, in the `android.ui` section the
Slint app used, so the choices made there carry over. The translations in `assets/lang` follow the
desktop app's format, and keys a translation lacks show in English.

## How Kotlin reaches the core

- `n_music_ffi` exports the core with UniFFI. `Core.start` runs the core services on their own
  bus thread, `Core.send` emits the message a `Command` stands for, and `Core.nextEvent` suspends
  until the core announces something. `tracks` and `count` read the library directly, off the
  bus, like the desktop's worker.
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

## Background playback

### What runs where

The core lives in the process, not in an activity. `NMusicApplication.onCreate` starts it, and
that runs on every process start, whether Android started the process for the UI, for
`PlaybackService` or for a media button. The Slint app ran the core inside `android_main`, so it
lived and died with the activity.

What keeps the process alive is `PlaybackService`, a Media3 `MediaSessionService`:

- Media3 runs it in the foreground, with the media notification, while `NPlayer` reports
  playing.
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
  volume by itself, so the core's volume, which is the user's setting, never changes.
- `AudioBecomingNoisyManager` pauses when headphones are unplugged. Without it the core would
  follow the device change and carry on through the speaker.

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
starts `PlaybackService`, the play press reaches `NPlayer`, and the core resumes its saved
session, or plays the whole library when nothing was chosen yet. Media3 only calls
`onPlaybackResumption` to start playback when the player is empty and accepts new media items,
which `NPlayer` never does, so its implementation only describes the session to whoever asks.

### Not done yet

- Resume is off by default in the core's settings, and this app has no settings screen yet. The
  core also has no event that reports the setting, which a toggle would need.
- From Android 15 an audio focus request fails unless the app is on top or runs a foreground
  service. Playback from the app is on top. Playback from a headset with the app in the
  background asks for focus while Media3 brings the service to the foreground, and whether
  Android accepts it at that moment still has to be checked on a device.
- Android's resumption card after a reboot needs a `MediaLibraryService`. Android 15 also
  forbids starting a `mediaPlayback` foreground service from `BOOT_COMPLETED`, so that card is
  the only way back after a reboot.
- The app registers only the Storage Access Framework provider. Web streams would need the
  `INTERNET` permission, the core's `WebProvider` and a Wi-Fi lock while a remote track plays,
  which Media3's `WifiLockManager` covers.

## References

- [Background playback with a MediaSessionService](https://developer.android.com/media/media3/session/background-playback)
- [Media3 release notes](https://developer.android.com/jetpack/androidx/releases/media3)
- [Manage audio focus](https://developer.android.com/media/optimize/audio-focus)
- [Cached apps freezer](https://source.android.com/docs/core/perf/cached-apps-freezer)
