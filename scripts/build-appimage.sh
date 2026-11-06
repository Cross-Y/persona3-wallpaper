#!/usr/bin/env bash
# Experimental: bundle Qt/QML through linuxdeploy + its Qt plugin.
# Build on an older supported glibc-based distro, not Arch, for broad compatibility.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
[[ "$(uname -m)" == x86_64 ]] || { echo 'Only x86_64 configured so far.' >&2; exit 1; }
for tool in "${LINUXDEPLOY:-linuxdeploy}" "${LINUXDEPLOY_QT:-linuxdeploy-plugin-qt}"; do
  command -v "$tool" >/dev/null || { echo "Missing $tool (provide in PATH or via LINUXDEPLOY/LINUXDEPLOY_QT)" >&2; exit 1; }
done
if [[ "${1:-}" == '--skip-build' ]]; then
    [[ $# -eq 1 ]] || exit 2
else
    [[ $# -eq 0 ]] || exit 2
    "$ROOT/scripts/build-release.sh"
fi
BIN="${P3W_BINARY:-$ROOT/target/release/persona3-wallpaper}"
[[ -f "$BIN" && -x "$BIN" ]] || { echo "Missing executable: $BIN" >&2; exit 1; }
mkdir -p "$ROOT/dist"
STAGE="$(mktemp -d "$ROOT/dist/.appimage-stage.XXXXXXXX")"
trap 'rm -rf -- "$STAGE"' EXIT
APPDIR="$STAGE/AppDir"
install -Dm755 "$BIN" "$APPDIR/usr/bin/persona3-wallpaper"
install -Dm644 "$ROOT/packaging/desktop/persona3-wallpaper.desktop" "$APPDIR/usr/share/applications/persona3-wallpaper.desktop"
install -Dm644 "$ROOT/packaging/icons/persona3-wallpaper.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/persona3-wallpaper.png"
# Plugin mode: executable is in PATH under its conventional plugin name.
PLUGIN="${LINUXDEPLOY_QT:-linuxdeploy-plugin-qt}"
if [[ "$PLUGIN" == */* ]]; then
  mkdir -p "$STAGE/plugins"
  ln -s -- "$(realpath "$PLUGIN")" "$STAGE/plugins/linuxdeploy-plugin-qt"
  export PATH="$STAGE/plugins:$PATH"
fi
export QML_SOURCES_PATHS="$ROOT/app/qml"
export EXTRA_QT_MODULES='multimedia'
export EXTRA_PLATFORM_PLUGINS='libqwayland-egl.so;libqwayland-generic.so'
export VERSION="$(awk -F'"' '/^version = /{print $2; exit}' "$ROOT/Cargo.toml")"
(
  cd "$ROOT/dist"
  "${LINUXDEPLOY:-linuxdeploy}" --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/persona3-wallpaper" \
    --desktop-file "$APPDIR/usr/share/applications/persona3-wallpaper.desktop" \
    --icon-file "$APPDIR/usr/share/icons/hicolor/256x256/apps/persona3-wallpaper.png" \
    --plugin qt --output appimage
)
