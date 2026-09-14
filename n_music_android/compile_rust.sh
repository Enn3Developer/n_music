#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
source ./env.sh
cargo ndk -t arm64-v8a -o android/build/rustJniLibs -P 30 build --locked --package n_music_android --lib --no-default-features
