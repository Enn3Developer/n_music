# N Music

Simple music player written in Rust + Slint

## Contribute

### Translations

If your language isn't fully supported by N Music, you can add a language by creating a file in `n_player/assets/lang`.
The file must be a JSON file and its name should be like this: `en_English.json`; `en` is the denominator of the
language, `English` is the name of the language in that language.

You can copy the english file and rename it correctly and start translating, then to check if everything works correctly
you can compile and run a debug build, the language will automatically be added to the supported languages during
compilation.
