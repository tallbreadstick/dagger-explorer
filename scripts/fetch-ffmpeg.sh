#!/usr/bin/env bash
# Downloads ffmpeg essentials/static builds into assets/ffmpeg/<platform>/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ASSETS="$ROOT/assets/ffmpeg"

mkdir -p "$ASSETS/linux-x86_64" "$ASSETS/linux-aarch64" "$ASSETS/windows-x86_64"

fetch_linux() {
  local arch="$1"
  local dir="$ASSETS/linux-$arch"
  local out="$dir/ffmpeg"
  [[ -x "$out" ]] && return 0

  local url tarball inner
  case "$arch" in
    x86_64)
      url="https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz"
      inner="ffmpeg-master-latest-linux64-gpl/bin/ffmpeg"
      ;;
    aarch64)
      url="https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linuxarm64-gpl.tar.xz"
      inner="ffmpeg-master-latest-linuxarm64-gpl/bin/ffmpeg"
      ;;
    *)
      echo "Unsupported Linux arch: $arch" >&2
      return 1
      ;;
  esac

  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  echo "Fetching Linux $arch ffmpeg..."
  curl -fsSL "$url" -o "$tmp/ffmpeg.tar.xz"
  tar -xJf "$tmp/ffmpeg.tar.xz" -C "$tmp"
  install -m 755 "$tmp/$inner" "$out"
  echo "Installed $out"
}

fetch_windows() {
  local dir="$ASSETS/windows-x86_64"
  local out="$dir/ffmpeg.exe"
  [[ -x "$out" ]] && return 0

  local url="https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  echo "Fetching Windows ffmpeg essentials..."
  curl -fsSL "$url" -o "$tmp/ffmpeg.zip"
  unzip -q "$tmp/ffmpeg.zip" -d "$tmp"
  local bin
  bin="$(find "$tmp" -name ffmpeg.exe | head -n1)"
  install -m 755 "$bin" "$out"
  echo "Installed $out"
}

case "$(uname -s)" in
  Linux)
    case "$(uname -m)" in
      x86_64) fetch_linux x86_64 ;;
      aarch64|arm64) fetch_linux aarch64 ;;
      *) echo "Unsupported machine: $(uname -m)" >&2; exit 1 ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*)
    fetch_windows
    ;;
  Darwin)
    echo "macOS: install ffmpeg via brew and ensure it is on PATH, or extend this script." >&2
    ;;
  *)
    echo "Unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

echo "Done."
