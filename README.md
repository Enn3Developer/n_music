# N Music

Cross-platform music player written in Rust + Slint

<img src="readme_preview.png" height="450" alt="App Preview"/>

## Features

- Cover art
- Support media control (Only Linux and Android)
- Extremely fast and resource efficient
- Locale support

## Coming

- Android builds:
    - [x] Kotlin code
    - [x] Notifications
    - [x] Working dev build
    - [x] Automatic CI for releases

- Streaming:
    - [ ] Simple web streaming
    - [ ] Youtube streaming
    - [ ] Deezer streaming

- QoL:
    - [ ] Playlists
    - [x] Media control for Windows, Mac and Linux
    - [ ] Auto updater (desktop only; opt-out)

## Contribute

### Building

Run in debug mode:

```shell
cargo run --package n_music_desktop
```

Build in release mode:

```shell
cargo build --release --package n_music_desktop
```

### Translations

If your language isn't fully supported by N Music, you can add a language by creating a file in
`n_music_desktop/assets/lang` (and its copy in `n_music_android/assets/lang`).
The file must be a JSON file and its name should be like this: `it_Italiano.json`; `it` is the denominator of the
language, `Italiano` is the name of the language in that language (i.e. how it should be displayed in the app).

You can copy the english file and rename it correctly and start translating, then to check if everything works correctly
you can compile and run a debug build, the language will automatically be added to the supported languages during
compilation.
