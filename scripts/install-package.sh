#!/usr/bin/env bash
# One-command installer for a prebuilt local package. apt and pacman resolve
# native package dependencies; AppImages are registered per-user.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
[[ $# -eq 1 && -f "$1" ]] || { echo 'Usage: bash scripts/install-package.sh /path/to/package.deb|.pkg.tar.zst|.AppImage' >&2; exit 2; }
[[ "$(id -u)" -ne 0 ]] || { echo 'Run as a regular user. The package manager will request sudo only when needed.' >&2; exit 1; }
PKG="$(realpath -- "$1")"
case "$PKG" in
    *.deb)
        command -v apt >/dev/null 2>&1 || { echo 'This .deb requires a Debian/Ubuntu system with apt.' >&2; exit 1; }
        echo 'APT will show and resolve the package dependencies before asking you to confirm.'
        sudo apt install -- "$PKG"
        ;;
    *.pkg.tar.zst)
        command -v pacman >/dev/null 2>&1 || { echo 'This package requires Arch Linux / CachyOS with pacman.' >&2; exit 1; }
        echo 'Pacman will show and resolve package dependencies before asking you to confirm.'
        sudo pacman -U -- "$PKG"
        ;;
    *.AppImage)
        "$ROOT/scripts/install-appimage-local.sh" "$PKG"
        echo 'The AppImage was added to your application menu. Some external wallpaper backends may still be needed.'
        ;;
    *) echo 'Unsupported package type. Use a .deb, .pkg.tar.zst or .AppImage.' >&2; exit 2 ;;
esac
