# Persona 3 Wallpaper

**An animated and static wallpaper manager for Linux, built with Rust and Qt/QML.**

Persona 3 Wallpaper is a Linux desktop application with a themed interface, desktop-aware wallpaper backends, a Wallhaven browser, and optional desktop appearance integrations.

> **Beta software:** This project is still being tested across Linux desktop environments. Some features depend on your compositor or desktop, and installation from source compiles the application on your machine.

## Quick installation (Linux)

Clone the repository and run the included per-user installer:

```bash
git clone https://github.com/Cross-Y/persona3-wallpaper.git
cd persona3-wallpaper
bash scripts/install-local.sh
```

The installer is designed to check for the required build tools and Qt modules, ask before installing missing system dependencies, compile the application, and add **Persona 3 Wallpaper** to your application launcher.

After installation, search for **Persona 3 Wallpaper** in your desktop's application menu.

**Before installing:** You need a supported Linux desktop session and a suitable wallpaper backend. Some backends (such as `mpvpaper` or desktop-specific plugins) are **not installed automatically**. Support varies by desktop environment; see [Compatibility and limitations](#compatibility-and-limitations).

### Check dependencies without installing

```bash
bash scripts/ensure-build-deps.sh --check
```

### Install an already compiled local build

```bash
bash scripts/install-local.sh --skip-build
```

This option expects the application to have been built already.

### Uninstall

From the repository directory:

```bash
bash scripts/uninstall-local.sh
```

The local uninstaller is intended to remove the installed launcher and application files while keeping your personal settings.

## Features

- **Animated and static wallpapers** with desktop-aware backends.
- **Multiple-monitor selection** where supported by the backend.
- **Wallpaper library and Wallhaven catalog** for finding and managing wallpapers.
- **Playback controls, autostart, and session restoration.**
- Synchronized settings and editable palette (Primary, Secondary, Tertiary, and Quaternary), with optional desktop recoloring.
- **Custom fonts and cursor themes**, including validated imports.
- **Replaceable UI sound effects** for windows, tabs, hover, and selection.
- **System tray resource statistics**, including on-demand RSS, PSS, and CPU sampling.
- **Experimental video optimization** that can cache a lower-resolution H.264 copy before applying a wallpaper.
- **Static-image framing** with fit, stretch, and crop controls.

## Requirements

- Linux with a graphical desktop session.
- Rust and Cargo, a C/C++ build toolchain, and Qt 6 development libraries to compile from source.
- FFmpeg for video processing, previews, and dynamic color sampling.
- A compatible wallpaper backend for your desktop environment.
- Optional: the Qt Multimedia QML module for menu sounds.

The local installer can help with some build dependencies, but it does not guarantee that every wallpaper backend or desktop integration is available on your distribution.

## Compatibility and limitations

Wallpaper playback and desktop customization are backend-dependent. The application includes integrations for multiple Linux desktop environments, but **real-world testing is still required** across distributions, Wayland/X11 sessions, and desktop versions.

Known beta limitations:

- Qt graphical smoke tests and long-running memory tests are still needed.
- Network catalog searches, file imports, and some FFmpeg operations may briefly block the interface.
- Video optimization and desktop recoloring are experimental.
- Packaging scripts exist, but prebuilt packages should not be considered verified until they have been built and tested on their target systems.

If you encounter a problem, please include your Linux distribution, desktop environment/compositor, display server (Wayland or X11), and relevant terminal output when reporting it.

## Prebuilt packages (work in progress)

The repository contains packaging scripts for Arch/CachyOS (`.pkg.tar.zst`), Debian/Ubuntu (`.deb`), and an experimental AppImage. These are **not yet guaranteed to be available as downloadable releases**.

Once verified builds are published, they will be listed in the repository's **Releases** section. Until then, use the source installation instructions above.

## Development

Run these commands from the repository root:

```bash
python3 -m unittest discover -s tests -v
cargo check --workspace
cargo test --workspace
cargo run -p persona3-wallpaper
```

The Python tests mainly check source structure and integration assumptions. They do not replace a successful Rust/Qt build or testing in a real graphical Linux session.

## Settings and data

- Settings: `~/.config/persona3-wallpaper/settings.json` (or `XDG_CONFIG_HOME`).
- Imported wallpapers: `~/.local/share/persona3-wallpaper/wallpapers/` (or `XDG_DATA_HOME`).
- UI sounds: `app/assets/ui/` (replace the files and rebuild).

## License and third-party assets

The source code is licensed under **GPL-3.0-or-later**; see [LICENSE](LICENSE).

**Important:** Persona 3 and its related artwork and audio belong to their respective rights holders. The source-code license does **not** grant permission to redistribute third-party illustrations, icons, music, or sound recordings. Before publishing source archives or binaries publicly, replace any unlicensed proprietary assets with original or properly licensed alternatives.

This project is an independent fan-made application and is not affiliated with or endorsed by Atlus or SEGA.
