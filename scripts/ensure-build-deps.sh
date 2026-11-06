#!/usr/bin/env bash
# Interactive dependency preflight for people compiling from source.
# Never downloads packages outside the distro's configured repositories.
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: bash scripts/ensure-build-deps.sh [--check | --yes]
  (no flags)  Check missing packages and ask before installing them.
  --check     Print missing packages and the suggested command; do not install.
  --yes       Explicitly authorize installing missing packages without a prompt.

This script is for building from source. Users of .deb/.pkg.tar.zst packages
should install with apt/pacman; their package managers handle dependencies.
USAGE
}
MODE=prompt
case "${1:-}" in
    '') [[ $# -eq 0 ]] || { usage >&2; exit 2; } ;;
    --check) [[ $# -eq 1 ]] || { usage >&2; exit 2; }; MODE=check ;;
    --yes) [[ $# -eq 1 ]] || { usage >&2; exit 2; }; MODE=yes ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
esac

if [[ "$(id -u)" -eq 0 && "$MODE" != check ]]; then
    echo 'Run as a regular user, not root. Sudo is only used for packages.' >&2
    exit 1
fi

OS_RELEASE="${P3W_OS_RELEASE_FILE:-/etc/os-release}"
[[ -r "$OS_RELEASE" ]] || { echo "Cannot read $OS_RELEASE" >&2; exit 1; }
# A system-managed os-release file (tests may override the path).
# shellcheck disable=SC1090
source "$OS_RELEASE"
DIST_ID="${ID:-}"
DIST_LIKE=" ${ID_LIKE:-} "
PM=''
if [[ "$DIST_ID" == arch || "$DIST_ID" == cachyos || "$DIST_ID" == endeavouros || "$DIST_LIKE" == *' arch '* ]]; then
    PM=pacman
elif [[ "$DIST_ID" == debian || "$DIST_ID" == ubuntu || "$DIST_ID" == linuxmint || "$DIST_ID" == pop || "$DIST_LIKE" == *' debian '* || "$DIST_LIKE" == *' ubuntu '* ]]; then
    PM=apt
elif [[ "$DIST_ID" == fedora || "$DIST_LIKE" == *' fedora '* ]]; then
    PM=dnf
else
    echo "Unsupported distribution for automatic build dependency installation: ${PRETTY_NAME:-$DIST_ID}" >&2
    echo 'Install Rust, a C++ build toolchain, Qt 6 development/QML modules, FFmpeg and Python using your package manager.' >&2
    exit 1
fi

case "$PM" in
    pacman) command -v pacman >/dev/null 2>&1 || { echo 'pacman was not found.' >&2; exit 1; } ;;
    apt)
        command -v apt-get >/dev/null 2>&1 || { echo 'apt-get was not found.' >&2; exit 1; }
        command -v dpkg-query >/dev/null 2>&1 || { echo 'dpkg-query was not found.' >&2; exit 1; }
        ;;
    dnf)
        command -v dnf >/dev/null 2>&1 || { echo 'dnf was not found.' >&2; exit 1; }
        command -v rpm >/dev/null 2>&1 || { echo 'rpm was not found.' >&2; exit 1; }
        ;;
esac

package_installed() {
    case "$PM" in
        pacman) pacman -Qq -- "$1" >/dev/null 2>&1 ;;
        apt) [[ "$(dpkg-query -W -f='${Status}' -- "$1" 2>/dev/null || :)" == 'install ok installed' ]] ;;
        dnf) rpm -q --quiet -- "$1" >/dev/null 2>&1 ;;
    esac
}

# Keep tools supplied by rustup instead of installing a second Rust toolchain.
# Check individual packages otherwise: pkg-config alone cannot guarantee that
# the platform's QML import plugins are present.
MISSING=()
add_if_missing() { package_installed "$1" || MISSING+=("$1"); }
case "$PM" in
    pacman)
        for pkg in base-devel cmake pkgconf qt6-base qt6-declarative qt6-multimedia qt6-wayland ffmpeg python; do
            add_if_missing "$pkg"
        done
        if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
            add_if_missing rust
        fi
        ;;
    apt)
        for pkg in build-essential cmake pkg-config qt6-base-dev qt6-declarative-dev qt6-multimedia-dev qt6-wayland \
                   qml6-module-qtquick qml6-module-qtquick-controls qml6-module-qtquick-layouts \
                   qml6-module-qtquick-dialogs qml6-module-qt-labs-platform qml6-module-qtmultimedia \
                   ffmpeg python3; do
            add_if_missing "$pkg"
        done
        command -v cargo >/dev/null 2>&1 || add_if_missing cargo
        command -v rustc >/dev/null 2>&1 || add_if_missing rustc
        ;;
    dnf)
        for pkg in gcc-c++ make cmake pkgconf-pkg-config qt6-qtbase-devel qt6-qtdeclarative-devel \
                   qt6-qtmultimedia-devel qt6-qtwayland ffmpeg-free python3; do
            add_if_missing "$pkg"
        done
        command -v cargo >/dev/null 2>&1 || add_if_missing cargo
        command -v rustc >/dev/null 2>&1 || add_if_missing rust
        ;;
esac

if [[ ${#MISSING[@]} -eq 0 ]]; then
    echo 'All required source-build packages are already installed.'
    exit 0
fi

printf 'Missing build dependencies (%s):\n' "${#MISSING[@]}"
printf '  - %s\n' "${MISSING[@]}"

case "$PM" in
    pacman) INSTALL_CMD=(pacman -S --needed -- "${MISSING[@]}") ;;
    apt) INSTALL_CMD=(apt-get install -- "${MISSING[@]}") ;;
    dnf) INSTALL_CMD=(dnf install -- "${MISSING[@]}") ;;
esac
printf '\nProposed installation command:\n  sudo'
printf ' %q' "${INSTALL_CMD[@]}"
printf '\n\n'

if [[ "$MODE" == check ]]; then
    echo 'Check only: no system changes were made.'
    exit 2
fi
if [[ "$MODE" == prompt ]]; then
    if [[ ! -t 0 ]]; then
        echo 'Non-interactive terminal: no packages installed. Run manually or use --yes to authorize.' >&2
        exit 2
    fi
    read -r -p 'Install these packages from your configured repositories? [y/N] ' answer
    case "$answer" in
        y|Y|yes|YES|Yes|s|S|si|SI|sí|Sí) ;;
        *) echo 'Dependency installation declined. Your system was not changed.'; exit 2 ;;
    esac
fi

command -v sudo >/dev/null 2>&1 || { echo 'sudo is required to install system packages.' >&2; exit 1; }
sudo "${INSTALL_CMD[@]}" || { echo 'Dependency installation failed; no build was started.' >&2; exit 1; }
for pkg in "${MISSING[@]}"; do
    if ! package_installed "$pkg"; then
        echo "Package manager returned success, but $pkg is still missing. Check repositories." >&2
        exit 1
    fi
done
printf '\nBuild dependencies are installed. Proceeding with compilation.\n'
if [[ "$PM" == dnf ]]; then
    echo 'Note: Fedora ffmpeg-free may lack patented encoders; video conversion might need additional codecs.'
fi
