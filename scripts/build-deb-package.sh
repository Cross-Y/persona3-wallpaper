#!/usr/bin/env bash
# Build on Debian/Ubuntu, where dpkg-shlibdeps can resolve native Qt ABI deps.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
if [[ -r /etc/os-release ]]; then
  . /etc/os-release
else
  echo 'Missing /etc/os-release.' >&2; exit 1
fi
case "${ID:-}" in
  debian|ubuntu|linuxmint|pop) ;;
  *) echo 'Build .deb on Debian/Ubuntu or a derivative; native Qt libraries are distro-specific.' >&2; exit 1 ;;
esac
for tool in dpkg-deb dpkg-shlibdeps dpkg python3; do
  command -v "$tool" >/dev/null || { echo "Missing $tool" >&2; exit 1; }
done
if [[ "${1:-}" == '--skip-build' ]]; then
    [[ $# -eq 1 ]] || exit 2
else
    [[ $# -eq 0 ]] || exit 2
    "$ROOT/scripts/build-release.sh"
fi
BIN="${P3W_BINARY:-$ROOT/target/release/persona3-wallpaper}"
[[ -f "$BIN" && -x "$BIN" ]] || { echo "Missing executable: $BIN" >&2; exit 1; }
VERSION="$(awk -F'"' '/^version = /{print $2; exit}' "$ROOT/Cargo.toml")"
ARCH="$(dpkg --print-architecture)"
mkdir -p "$ROOT/dist"
STAGE="$(mktemp -d "$ROOT/dist/.deb-stage.XXXXXXXX")"
trap 'rm -rf -- "$STAGE"' EXIT
install -Dm755 "$BIN" "$STAGE/usr/bin/persona3-wallpaper"
install -Dm644 "$ROOT/packaging/desktop/persona3-wallpaper.desktop" "$STAGE/usr/share/applications/persona3-wallpaper.desktop"
install -Dm644 "$ROOT/packaging/icons/persona3-wallpaper.png" "$STAGE/usr/share/icons/hicolor/256x256/apps/persona3-wallpaper.png"
mkdir -p "$STAGE/DEBIAN" "$STAGE/debian"
printf 'Source: persona3-wallpaper\nSection: utils\nPriority: optional\nMaintainer: %s\nStandards-Version: 4.6.2\n\nPackage: persona3-wallpaper\nArchitecture: any\nDescription: Linux wallpaper manager\n' \
  "${P3W_MAINTAINER:-Local beta builder <local@localhost>}" > "$STAGE/debian/control"
SHLIBS="$(cd "$STAGE" && dpkg-shlibdeps -O -e"$STAGE/usr/bin/persona3-wallpaper" | sed -n 's/^shlibs:Depends=//p')"
[[ -n "$SHLIBS" ]] || { echo 'Cannot resolve native library dependencies; .deb not generated.' >&2; exit 1; }
cat > "$STAGE/DEBIAN/control" <<EOF
Package: persona3-wallpaper
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Maintainer: ${P3W_MAINTAINER:-Local beta builder <local@localhost>}
Depends: $SHLIBS, qml6-module-qtquick, qml6-module-qtquick-controls, qml6-module-qtquick-layouts, qml6-module-qtquick-dialogs, qml6-module-qt-labs-platform, qml6-module-qtmultimedia, qt6-wayland, ffmpeg
Description: Animated and static Linux wallpaper manager
 Manage wallpapers on supported Linux desktop environments with a Qt interface.
EOF
rm -rf "$STAGE/debian"
dpkg-deb --build --root-owner-group "$STAGE" "$ROOT/dist/persona3-wallpaper_${VERSION}_${ARCH}.deb"
