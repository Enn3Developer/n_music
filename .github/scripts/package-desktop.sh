#!/usr/bin/env bash
# Packages the desktop app for one runtime, in two steps the release workflow runs in turn:
#   stage  copies the app built by `cargo build --release` and the Qt runtime it needs into
#          target/velopack-stage/<runtime>, and names the file that starts the staged app
#          (`launcher` in GITHUB_OUTPUT, when set);
#   pack   turns the staged app into Velopack packages in target/velopack/<runtime>.
# It follows scripts/package-release.sh and adds what the Qt interface needs. QT_ROOT_DIR is
# the Qt the app was built with. In GitHub Actions, pack first fetches the channel's previous
# release from GitHub, so Velopack adds a delta package that updates from it.
set -euo pipefail

cd "$(dirname "$0")/../.."
usage="Usage: bash .github/scripts/package-desktop.sh stage|pack linux-x64|win-x64|osx-arm64"
step=${1:-}
runtime=${2:-}
version=$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["workspace"]["package"]["version"])')
pack_id=com.enn3developer.n_music
executable=n_music_desktop
case "$runtime" in
  linux-x64)
    target=x86_64-unknown-linux-gnu
    pack_dir=NMusic.AppDir
    ;;
  win-x64)
    target=x86_64-pc-windows-msvc
    executable=n_music_desktop.exe
    pack_dir=app
    ;;
  osx-arm64)
    target=aarch64-apple-darwin
    pack_dir=$pack_id.app
    ;;
  *) echo "$usage" >&2; exit 1 ;;
esac

channel=$runtime
if [[ "${version%%+*}" == *-* ]]; then
  channel="$runtime-preview"
fi
stage="target/velopack-stage/$runtime"
pack_dir="$stage/$pack_dir"

stage_linux() {
  mkdir -p "$pack_dir/usr/bin"
  cp "$binary" "$pack_dir/usr/bin/$executable"
  cp LICENSE "$pack_dir/usr/bin/LICENSE"
  cp n_music_desktop/assets/icons/icon.png "$pack_dir/$pack_id.png"
  cp n_music_desktop/assets/icons/icon.png "$pack_dir/.DirIcon"
  cat > "$pack_dir/$pack_id.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=N Music
Exec=n_music_desktop
Icon=$pack_id
StartupWMClass=$pack_id
Categories=AudioVideo;Audio;Player;
X-AppImage-Version=$version
EOF
  cat > "$pack_dir/AppRun" <<'EOF'
#!/bin/sh
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export LD_LIBRARY_PATH="$app_dir/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$app_dir/usr/bin/n_music_desktop" "$@"
EOF
  chmod +x "$pack_dir/AppRun" "$pack_dir/usr/bin/$executable"
  python3 .github/scripts/deploy-qt-linux.py "$pack_dir/usr" "$QT_ROOT_DIR" n_music_desktop/qml "$executable"
  launcher="$pack_dir/AppRun"
}

stage_windows() {
  mkdir -p "$pack_dir"
  cp "$binary" "$pack_dir/$executable"
  cp LICENSE "$pack_dir/LICENSE"
  # Velopack's installer brings the Visual C++ runtime, which Qt needs as well.
  "$QT_ROOT_DIR/bin/windeployqt.exe" --release --qmldir n_music_desktop/qml --no-translations \
    --no-compiler-runtime --no-system-d3d-compiler --no-system-dxc-compiler --no-opengl-sw \
    "$pack_dir/$executable"
  launcher="$pack_dir/$executable"
}

stage_macos() {
  mkdir -p "$stage/app"
  cp "$binary" "$stage/app/$executable"
  iconset="$stage/NMusic.iconset"
  mkdir -p "$iconset"
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" n_music_desktop/assets/icons/icon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" n_music_desktop/assets/icons/icon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$iconset" -o "$stage/NMusic.icns"
  # The .app bundle `vpk pack` made before, which macdeployqt can then complete.
  dotnet vpk bundle --packId "$pack_id" --packTitle "N Music" --packAuthors Enn3Developer \
    --packVersion "$version" --packDir "$stage/app" --mainExe "$executable" \
    --icon "$stage/NMusic.icns" --bundleId com.enn3developer.n-music \
    --outputDir "$stage" --yes --skip-updates
  cp LICENSE "$pack_dir/Contents/Resources/LICENSE"
  # Copying Qt's frameworks into the bundle breaks Qt's own signatures, and Apple silicon runs
  # no code with a broken one, so macdeployqt signs the bundle again, ad hoc.
  "$QT_ROOT_DIR/bin/macdeployqt" "$pack_dir" -qmldir=n_music_desktop/qml -codesign=-
  launcher="$pack_dir/Contents/MacOS/$executable"
}

stage() {
  : "${QT_ROOT_DIR:?Set QT_ROOT_DIR to the Qt installation the app was built with}"
  binary="target/$target/release/$executable"
  if [[ ! -f "$binary" ]]; then
    echo "Build first: cargo build --locked --release --package n_music_desktop --bin n_music_desktop --target $target" >&2
    exit 1
  fi
  # Recreate only this runtime's staging directory, so a local rebuild cannot package old files.
  rm -rf "$stage"
  mkdir -p "$stage"
  case "$runtime" in
    linux-x64) stage_linux ;;
    win-x64) stage_windows ;;
    osx-arm64) stage_macos ;;
  esac
  echo "Staged $launcher"
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "launcher=$launcher" >> "$GITHUB_OUTPUT"
  fi
}

pack() {
  if [[ ! -d "$pack_dir" ]]; then
    echo "Stage first: bash .github/scripts/package-desktop.sh stage $runtime" >&2
    exit 1
  fi
  options=()
  if [[ "$runtime" == win-x64 ]]; then
    options+=(--icon n_music_desktop/assets/icons/icon.ico --framework vcredist143-x64)
  fi
  # vpk pack writes into work; output gets only this release's files, so a local rebuild
  # cannot publish old packages or list assets absent from the new GitHub release.
  work="target/velopack-work/$runtime"
  output="target/velopack/$runtime"
  rm -rf "$work" "$output"
  mkdir -p "$work"
  if [[ -n "${GITHUB_REPOSITORY:-}" ]]; then
    python3 .github/scripts/velopack-release.py previous "$GITHUB_REPOSITORY" "$channel" "$pack_id" "$version" "$work"
  fi
  dotnet vpk pack --packId "$pack_id" --packTitle "N Music" \
    --packAuthors Enn3Developer --packVersion "$version" \
    --packDir "$pack_dir" --mainExe "$executable" \
    --runtime "$runtime" --channel "$channel" \
    --outputDir "$work" ${options[@]+"${options[@]}"} --yes --skip-updates
  python3 .github/scripts/velopack-release.py collect "$work" "$channel" "$output"
}

case "$step" in
  stage) stage ;;
  pack) pack ;;
  *) echo "$usage" >&2; exit 1 ;;
esac
