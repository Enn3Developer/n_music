#!/usr/bin/env bash
# Builds the core (n_music_ffi) as a static library for the macOS app and generates its Swift
# bindings. Xcode runs it before compiling the app; it also runs on its own:
#
#   n_music_macos/scripts/build-core.sh [debug|macos]
#
# The Cargo profile follows the Xcode configuration: Debug builds with `dev` (fast to build),
# Release with `macos` (fat LTO, one codegen unit; see the workspace Cargo.toml).
# N_MUSIC_RUST_PROFILE=macos makes a Debug build use the optimized core too.
#
# Xcode links the library from $N_MUSIC_CORE_LIB_DIR (project.yml: one folder per
# configuration, whatever the profile): run from Xcode, the script puts the library it just
# built there, so the app always links the core the bindings were generated from.
#
# It builds for each architecture in $ARCHS (Xcode sets it; this Mac's own outside Xcode) with
# the CPU baseline of that architecture's oldest supported Mac, and lipos them into
# target/macos/<profile>/libn_music_ffi.a. The bindings land in n_music_macos/Generated.
set -euo pipefail

macos_dir="$(cd "$(dirname "$0")/.." && pwd)"
root="$(cd "$macos_dir/.." && pwd)"
cd "$root"

# Xcode starts build phases with a minimal PATH.
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

# The argument, else the override, else the configuration's (Xcode's RUST_PROFILE setting).
profile=${1:-${N_MUSIC_RUST_PROFILE:-${RUST_PROFILE:-}}}
if [[ -z "$profile" ]]; then
  case "${CONFIGURATION:-Debug}" in
    Release) profile=macos ;;
    *) profile=debug ;;
  esac
fi
case "$profile" in
  debug | dev) profile=debug; cargo_profile=(--profile dev) ;;
  macos) cargo_profile=(--profile macos) ;;
  *) echo "Unknown profile $profile: debug or macos" >&2; exit 1 ;;
esac

# The app's minimum: C dependencies (Opus, mozjpeg, SQLite) are compiled for it too.
export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-14.0}
# SDKROOT stays as Xcode set it: ring's and the other C dependencies' build scripts need it.

libraries=()
for arch in ${ARCHS:-$(uname -m)}; do
  case "$arch" in
    arm64)
      target=aarch64-apple-darwin
      # Every Mac with Apple silicon has at least an M1's instructions.
      cpu=apple-m1
      ;;
    x86_64)
      target=x86_64-apple-darwin
      # macOS 14 runs on no Intel Mac older than the Haswell generation's x86-64-v3.
      cpu=x86-64-v3
      ;;
    *) echo "Unsupported architecture $arch" >&2; exit 1 ;;
  esac
  rustup target list --installed 2>/dev/null | grep -qx "$target" || rustup target add "$target"
  # With --target, RUSTFLAGS reach only the target's code: build scripts and proc macros run
  # on this machine and are left alone.
  RUSTFLAGS="${RUSTFLAGS:-} -C target-cpu=$cpu" \
    cargo rustc --package n_music_ffi --lib --crate-type staticlib \
    --target "$target" "${cargo_profile[@]}"
  libraries+=("target/$target/$profile/libn_music_ffi.a")
done

out="target/macos/$profile"
mkdir -p "$out"
if [[ ${#libraries[@]} -eq 1 ]]; then
  # A copy rather than a link, so Xcode sees it change.
  cp -f "${libraries[0]}" "$out/libn_music_ffi.a.tmp"
else
  lipo -create "${libraries[@]}" -output "$out/libn_music_ffi.a.tmp"
fi
if ! cmp -s "$out/libn_music_ffi.a.tmp" "$out/libn_music_ffi.a"; then
  mv -f "$out/libn_music_ffi.a.tmp" "$out/libn_music_ffi.a"
else
  rm -f "$out/libn_music_ffi.a.tmp"
fi

# What Xcode links: this very library, whichever profile built it.
if [[ -n "${N_MUSIC_CORE_LIB_DIR:-}" ]]; then
  mkdir -p "$N_MUSIC_CORE_LIB_DIR"
  if ! cmp -s "$out/libn_music_ffi.a" "$N_MUSIC_CORE_LIB_DIR/libn_music_ffi.a"; then
    cp -f "$out/libn_music_ffi.a" "$N_MUSIC_CORE_LIB_DIR/libn_music_ffi.a.tmp"
    mv -f "$N_MUSIC_CORE_LIB_DIR/libn_music_ffi.a.tmp" "$N_MUSIC_CORE_LIB_DIR/libn_music_ffi.a"
  fi
fi

# The bindings follow what the library exports, read from its own metadata (library mode),
# with the workspace's own uniffi-bindgen so the versions always match.
generated="$macos_dir/Generated"
staging="target/macos/swift-$profile"
rm -rf "$staging"
cargo run --quiet --package n_music_uniffi_bindgen --bin uniffi-bindgen -- generate \
  --library "${libraries[0]}" --language swift --out-dir "$staging" --no-format
mkdir -p "$generated"
# Rewritten only when they changed, so Swift doesn't recompile everything every build.
sync() {
  if ! cmp -s "$1" "$2"; then
    cp -f "$1" "$2"
  fi
}
sync "$staging/n_music_ffi.swift" "$generated/NMusicCore.swift"
sync "$staging/n_music_ffiFFI.h" "$generated/n_music_ffiFFI.h"
sync "$staging/n_music_ffiFFI.modulemap" "$generated/module.modulemap"
echo "Built $out/libn_music_ffi.a ($profile) and its Swift bindings${N_MUSIC_CORE_LIB_DIR:+, linked from $N_MUSIC_CORE_LIB_DIR}"
