#!/usr/bin/env bash
# Does not touch user wallpapers, settings, themes, or cached optimizations.
set -euo pipefail
[[ -n "${HOME:-}" && "$HOME" == /* ]] || { echo 'HOME must be absolute.' >&2; exit 1; }
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}"
[[ "$DATA_DIR" == /* && "$CONFIG_DIR" == /* ]] || { echo 'XDG paths must be absolute.' >&2; exit 1; }
rm -f -- "$HOME/.local/bin/persona3-wallpaper" \
  "$HOME/.local/bin/persona3-wallpaper.AppImage" \
  "$DATA_DIR/applications/persona3-wallpaper.desktop" \
  "$DATA_DIR/icons/hicolor/256x256/apps/persona3-wallpaper.png"
# Remove only the app-owned autostart entry (do not disturb other startup apps).
AUTO="$CONFIG_DIR/autostart/persona3-wallpaper.desktop"
if [[ -f "$AUTO" ]] && grep -qF 'X-P3W-Managed=true' "$AUTO"; then
    rm -f -- "$AUTO"
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$DATA_DIR/applications" 2>/dev/null || true
fi
printf 'Removed per-user launcher, executable and icon. Settings and wallpapers preserved.\n'
