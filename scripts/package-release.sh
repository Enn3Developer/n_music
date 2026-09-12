#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
runtime=${1:?Usage: bash scripts/package-release.sh linux-x64|win-x64|osx-arm64}
version=$(python3 -c 'import tomllib; print(tomllib.load(open("n_player/Cargo.toml", "rb"))["package"]["version"])')
executable=n_player
options=()
case "$runtime" in
  linux-x64) target=x86_64-unknown-linux-gnu ;;
  win-x64)
    target=x86_64-pc-windows-msvc
    executable=n_player.exe
    options+=(--icon n_player/assets/icons/icon.ico --framework vcredist143-x64)
    ;;
  osx-arm64) target=aarch64-apple-darwin ;;
  *) echo "Unsupported runtime: $runtime" >&2; exit 1 ;;
esac

binary="target/$target/release/$executable"
if [[ ! -f "$binary" ]]; then
  echo "Build first: cargo build --locked --release --package n_player --bin n_player --target $target" >&2
  exit 1
fi

channel=$runtime
if [[ "${version%%+*}" == *-* ]]; then
  channel="$runtime-preview"
fi
output="target/velopack/$runtime"
# Recreate only this runtime's staging and output directories, so a local rebuild
# cannot publish old packages or list assets absent from the new GitHub release.
stage="target/velopack-stage/$runtime"
rm -rf "$stage" "$output"
mkdir -p "$stage" "$output"
pack_dir="$stage/app"
mkdir -p "$pack_dir"

if [[ "$runtime" == linux-x64 ]]; then
  pack_dir="$stage/NMusic.AppDir"
  mkdir -p "$pack_dir/usr/bin" "$pack_dir/usr/lib"
  cp "$binary" "$pack_dir/usr/bin/$executable"
  cp LICENSE "$pack_dir/usr/bin/LICENSE"
  cp n_player/assets/icons/icon.png "$pack_dir/NMusic.png"
  cp n_player/assets/icons/icon.png "$pack_dir/.DirIcon"
  cat > "$pack_dir/NMusic.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=N Music
Exec=n_player
Icon=NMusic
Categories=AudioVideo;Audio;Player;
X-AppImage-Version=$version
EOF
  cat > "$pack_dir/AppRun" <<'EOF'
#!/bin/sh
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export LD_LIBRARY_PATH="$app_dir/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$app_dir/usr/bin/n_player" "$@"
EOF
  chmod +x "$pack_dir/AppRun" "$pack_dir/usr/bin/$executable"

  # ldd includes transitive dependencies. Keep glibc and its loader on the host;
  # bundle the audio, font and window-system libraries linked by this build.
  ldd "$binary" > "$stage/ldd.txt"
  if grep -q 'not found' "$stage/ldd.txt"; then
    cat "$stage/ldd.txt" >&2
    exit 1
  fi
  while read -r name path; do
    name=${name##*/}
    case "$name" in
      libc.so.*|libm.so.*|libdl.so.*|libpthread.so.*|librt.so.*|libresolv.so.*|libutil.so.*|ld-linux*) continue ;;
    esac
    cp -L "$path" "$pack_dir/usr/lib/$name"
  done < <(awk '$2 == "=>" && $3 ~ /^\// { print $1, $3 }' "$stage/ldd.txt")
else
  cp "$binary" "$pack_dir/$executable"
  cp LICENSE "$pack_dir/LICENSE"
fi

if [[ "$runtime" == osx-arm64 ]]; then
  iconset="$stage/NMusic.iconset"
  mkdir -p "$iconset"
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" n_player/assets/icons/icon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" n_player/assets/icons/icon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$iconset" -o "$stage/NMusic.icns"
  options+=(--icon "$stage/NMusic.icns" --bundleId com.enn3developer.n-music)
fi

dotnet vpk pack --packId NMusic --packTitle "N Music" \
  --packAuthors Enn3Developer --packVersion "$version" \
  --packDir "$pack_dir" --mainExe "$executable" \
  --runtime "$runtime" --channel "$channel" \
  --outputDir "$output" --delta None "${options[@]}" --yes --skip-updates
