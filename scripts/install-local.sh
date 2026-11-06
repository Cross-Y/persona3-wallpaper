#!/usr/bin/env bash
# Per-user installation: ~/.local/bin + XDG applications + icon theme.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
if [[ "${1:-}" == '--skip-build' ]]; then
    [[ $# -eq 1 ]] || { echo 'Usage: install-local.sh [--skip-build]' >&2; exit 2; }
else
    [[ $# -eq 0 ]] || { echo 'Usage: install-local.sh [--skip-build]' >&2; exit 2; }
    # Only source builds need Rust, Qt development headers and build tools.
    # Never ask for root privileges when installing an already-built binary.
    "$ROOT/scripts/ensure-build-deps.sh"
    "$ROOT/scripts/build-release.sh"
fi
SOURCE_BIN="${P3W_BINARY:-$ROOT/target/release/persona3-wallpaper}"
[[ -f "$SOURCE_BIN" && -x "$SOURCE_BIN" ]] || { echo "Missing executable: $SOURCE_BIN" >&2; exit 1; }
[[ -n "${HOME:-}" && "$HOME" == /* ]] || { echo 'HOME must be an absolute path.' >&2; exit 1; }
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
[[ "$DATA_DIR" == /* ]] || { echo 'XDG_DATA_HOME must be absolute.' >&2; exit 1; }
BIN_DIR="$HOME/.local/bin"
APP_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/256x256/apps"
mkdir -p -- "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
TMP_BIN="$(mktemp "$BIN_DIR/.persona3-wallpaper.XXXXXXXX")"
TMP_DESKTOP="$(mktemp "$APP_DIR/.persona3-wallpaper.XXXXXXXX.desktop")"
trap 'rm -f -- "$TMP_BIN" "$TMP_DESKTOP"' EXIT
install -m 755 -- "$SOURCE_BIN" "$TMP_BIN"
python3 "$ROOT/packaging/write_desktop.py" \
  "$ROOT/packaging/desktop/persona3-wallpaper.desktop" "$TMP_DESKTOP" "$BIN_DIR/persona3-wallpaper"
if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$TMP_DESKTOP"
fi
# Install the icon before exposing the launcher, to avoid a missing-image race.
install -m 644 "$ROOT/packaging/icons/persona3-wallpaper.png" "$ICON_DIR/persona3-wallpaper.png"
mv -f -- "$TMP_BIN" "$BIN_DIR/persona3-wallpaper"
chmod 644 "$TMP_DESKTOP"
mv -f -- "$TMP_DESKTOP" "$APP_DIR/persona3-wallpaper.desktop"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi
printf '\nInstalled for current user.\n  Binary: %s\n  Launcher: %s\n' \
  "$BIN_DIR/persona3-wallpaper" "$APP_DIR/persona3-wallpaper.desktop"
printf 'Search for "Persona 3 Wallpaper" in your application launcher.\n'
printf 'To remove this local installation: bash scripts/uninstall-local.sh\n'
