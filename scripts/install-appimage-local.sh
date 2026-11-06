#!/usr/bin/env bash
# Register a downloaded AppImage in the user's XDG launcher without root.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
[[ $# -eq 1 && -f "$1" ]] || { echo 'Usage: install-appimage-local.sh PATH-TO-APPIMAGE' >&2; exit 2; }
SOURCE="$1"
[[ -n "${HOME:-}" && "$HOME" == /* ]] || { echo 'HOME must be absolute.' >&2; exit 1; }
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
[[ "$DATA_DIR" == /* ]] || { echo 'XDG_DATA_HOME must be absolute.' >&2; exit 1; }
BIN_DIR="$HOME/.local/bin"
APP_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/256x256/apps"
mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
TMP_BIN="$(mktemp "$BIN_DIR/.persona3-wallpaper-appimage.XXXXXXXX")"
TMP_DESKTOP="$(mktemp "$APP_DIR/.persona3-wallpaper.XXXXXXXX.desktop")"
trap 'rm -f -- "$TMP_BIN" "$TMP_DESKTOP"' EXIT
install -m 755 -- "$SOURCE" "$TMP_BIN"
python3 "$ROOT/packaging/write_desktop.py" \
  "$ROOT/packaging/desktop/persona3-wallpaper.desktop" "$TMP_DESKTOP" \
  "$BIN_DIR/persona3-wallpaper.AppImage"
if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$TMP_DESKTOP"
fi
install -m 644 "$ROOT/packaging/icons/persona3-wallpaper.png" "$ICON_DIR/persona3-wallpaper.png"
mv -f -- "$TMP_BIN" "$BIN_DIR/persona3-wallpaper.AppImage"
chmod 644 "$TMP_DESKTOP"
mv -f -- "$TMP_DESKTOP" "$APP_DIR/persona3-wallpaper.desktop"
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APP_DIR" 2>/dev/null || true
fi
printf 'Installed AppImage launcher: %s\n' "$APP_DIR/persona3-wallpaper.desktop"
