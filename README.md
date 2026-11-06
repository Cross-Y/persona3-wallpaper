# Persona 3 Wallpaper (Linux beta source)

A Rust + Qt/QML desktop wallpaper manager for Linux, with animated and static
wallpapers, per-desktop backend selection, a Wallhaven catalog, optional desktop
recoloring, and a themed interface.

## Current functionality

- Animated and static wallpaper playback through desktop-aware backends.
- Multiple display selection where the backend supports it.
- Configurable playback controls, session restoration, and autostart.
- Synchronized settings and editable palette (Primary, Secondary, Tertiary,
  and Quaternary). Color integrations are opt-in.
- Custom fonts and cursor themes; external downloads and imports are validated.
- Five replaceable window, tab, hover, and select sound effects.
- System tray with on-demand RSS, PSS, and CPU sampling while hidden.

## Requirements

- Linux (graphical testing is still required for each supported desktop).
- Rust/Cargo and the Qt 6 development libraries for building the CXX-Qt app.
- For menu sounds, the optional Qt Multimedia QML module.
- FFmpeg for video previews and dynamic color sampling.
- A wallpaper backend suitable for your desktop; see
  [Desktop compatibility](docs/LINUX_COMPATIBILITY.md).

## Development build

```bash
python3 -m unittest discover -s tests -v
cargo check --workspace
cargo test --workspace
cargo run -p persona3-wallpaper
```

This ZIP contains **source code**, not a redistributable installer. The Python
suite mainly checks source structure, scripts, and integrations; passing it
is not a substitute for `cargo check` or real Qt/desktop testing.

## Settings and files

- User settings: `~/.config/persona3-wallpaper/settings.json` or `XDG_CONFIG_HOME`.
- Imported wallpapers: `~/.local/share/persona3-wallpaper/wallpapers/` or `XDG_DATA_HOME`.
- Optional interface audio: `app/assets/ui/` (replace files, then rebuild).
- More details: [Architecture](docs/ARCHITECTURE.md),
  [Dynamic colors](docs/DYNAMIC_COLORS.md), and
  [Interface audio](docs/MENU_SOUND_EFFECTS.md).

## Stability work in this revision

The settings Qt bridge now delegates to small modules for autostart, desktop
choices, Hyprland focus events, wallpaper restoration, and process statistics.
Color processing is split into palette calculation and media frame extraction.
Persisted settings use an atomic temporary-file rename, and catalog HTTP
connections are reused. Failed downloads are cleaned up; previews detect
source modifications. Automatic colors are cached for unchanged wallpapers.

Known remaining validation tasks: Qt graphical smoke tests, long-run RSS/PSS
sampling, and hands-on desktop compatibility checks. Network catalog searches,
file imports, and some FFmpeg operations still call synchronous Rust/QML paths
and may briefly block the interface. These should be profiled on a real Linux
session before a public beta.

## Licensing and asset rights

Source code uses `GPL-3.0-or-later` (see [`LICENSE`](LICENSE) and Cargo metadata). The title,
interface artwork, and replacement sound files may have **different owners**.
Do **not** publish or redistribute proprietary game illustrations or sound
recordings with the source or binary unless you have permission or a valid
license. Replace them with original or properly licensed media for public
releases. Neither this document nor the code audit verifies asset ownership.

## Automatic video optimization (experimental)

When a monitor is smaller than the source video, the engine can transcode a
cached H.264 copy (for example 4K → 1080p), using FFmpeg in the background.
The source video remains untouched. Toggle this feature in **Settings →
Performance → Optimize video for monitor resolution**. See
[`docs/VIDEO_OPTIMIZATION.md`](docs/VIDEO_OPTIMIZATION.md) for its scope,
dependencies, cache limits, and the real-desktop verification procedure.


## Installable Linux beta builds

The repository now includes a Linux desktop launcher, a per-user installer,
and packaging scripts for Arch/CachyOS, Debian/Ubuntu, and an experimental
AppImage. See [Installation & packaging](docs/INSTALL_AND_PACKAGING.md).

On your build machine, run `bash scripts/install-local.sh`, then search for
**Persona 3 Wallpaper** in your application launcher. You can quit completely
and reopen it from the launcher. Packagers must still build and test each
platform's binaries; this source archive does not include a compiled executable.

## Pre-play optimization and image framing

Selecting an oversized video in Library offers **Optimize, then apply** or
**Play original**. Choosing to optimize keeps the current wallpaper visible
until FFmpeg finishes writing the cached copy; repeat selections reuse it.
Static images offer **Crop to fill**, **Fit entire image**, **Stretch to screen**,
with draggable crop position, zoom and persisted settings. For a native desktop
with unknown monitor dimensions, the app retains the desktop's default behavior.
See [Pre-play and image framing](docs/PREPLAY_AND_STATIC_FRAMING.md).


### Easy Linux install / dependency preflight

- **Prebuilt beta package:** `bash scripts/install-package.sh /path/to/package.deb`
  (or `.pkg.tar.zst` / `.AppImage`). For `.deb` and Arch packages, the system
  package manager resolves missing declared dependencies.
- **From source:** `bash scripts/install-local.sh` checks for required build
  tools / Qt modules and **asks for permission** before using the configured
  system package manager. Check without installing:
  `bash scripts/ensure-build-deps.sh --check`.
- **Already compiled:** `bash scripts/install-local.sh --skip-build` installs
  the executable without checking or installing Rust build dependencies.

See `docs/INSTALL_AND_PACKAGING.md`. Unsupported desktop wallpaper backends
(e.g. mpvpaper/AUR or compositor plugins) are not installed automatically.
