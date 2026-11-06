#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
[[ "$(uname -m)" == 'x86_64' ]] || { echo 'The current Arch PKGBUILD targets x86_64 only.' >&2; exit 1; }
command -v makepkg >/dev/null || { echo 'Install pacman-contrib/base-devel (makepkg) on Arch first.' >&2; exit 1; }
if [[ "${1:-}" == '--skip-build' ]]; then
    [[ $# -eq 1 ]] || exit 2
else
    [[ $# -eq 0 ]] || exit 2
    "$ROOT/scripts/build-release.sh"
fi
BIN="${P3W_BINARY:-$ROOT/target/release/persona3-wallpaper}"
[[ -f "$BIN" && -x "$BIN" ]] || { echo "Missing release binary: $BIN" >&2; exit 1; }
mkdir -p "$ROOT/dist"
STAGE="$(mktemp -d "$ROOT/dist/.arch-stage.XXXXXXXX")"
trap 'rm -rf -- "$STAGE"' EXIT
cp "$ROOT/packaging/arch/PKGBUILD" "$STAGE/PKGBUILD"
cp "$BIN" "$STAGE/persona3-wallpaper"
cp "$ROOT/packaging/desktop/persona3-wallpaper.desktop" "$STAGE/persona3-wallpaper.desktop"
cp "$ROOT/packaging/icons/persona3-wallpaper.png" "$STAGE/persona3-wallpaper.png"
# Generate local file checksums: never distribute placeholder SKIP checksums.
mapfile -t SUMS < <(cd "$STAGE" && sha256sum persona3-wallpaper persona3-wallpaper.desktop persona3-wallpaper.png | cut -d' ' -f1)
python3 - "$STAGE/PKGBUILD" "${SUMS[@]}" <<'PY2'
from pathlib import Path
import sys
p=Path(sys.argv[1]);s=p.read_text();s=s.replace("sha256sums=('SKIP' 'SKIP' 'SKIP')", "sha256sums=('"+"' '".join(sys.argv[2:])+"')")
p.write_text(s)
PY2
(cd "$STAGE" && makepkg --force --cleanbuild --noconfirm)
find "$STAGE" -maxdepth 1 -name '*.pkg.tar.zst' -exec cp -v {} "$ROOT/dist/" \;
printf '\nArch package placed in: %s/dist/\n' "$ROOT"
