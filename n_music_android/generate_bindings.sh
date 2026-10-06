#!/usr/bin/env bash
# Writes the Kotlin side of n_music_ffi, read from the library compile_rust.sh just built, where
# Gradle compiles it with the app.
set -euo pipefail
cd "$(dirname "$0")"
out=android/build/generated/uniffi
rm -rf "$out"
cargo run --locked --quiet --package n_music_uniffi_bindgen --bin uniffi-bindgen -- generate \
    --library android/build/rustJniLibs/arm64-v8a/libn_music_ffi.so \
    --language kotlin --no-format --out-dir "$out"
