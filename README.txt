PERSONA 3 WALLPAPER — LEGACY AUTOMATIC PERFORMANCE V2 NOTES
==========================================================

This file describes an earlier development snapshot and is retained for
historical reference. For the current version, use README.md and docs/.

Earlier features included:
- Manual wallpaper engine pause.
- Focus-aware playback: pause wallpapers when Hyprland reports an active
  window and resume them on the desktop.
- App autostart via ~/.config/autostart/persona3-wallpaper.desktop.
- Restore wallpapers at startup.
- Enable/disable video sound.
- Always-on audio or audio only when the desktop is focused.
- Saved preferences in ~/.config/persona3-wallpaper/settings.json.

BUILD FROM THE CURRENT SOURCE TREE
----------------------------------
cargo fmt --all
cargo check --workspace
cargo run -p persona3-wallpaper

EARLIER MODIFIED SOURCE FILES
-----------------------------
app/src/settings_controller.rs
app/src/settings_store.rs
app/src/main.rs
app/src/cxxqt_object.rs
app/build.rs
app/qml/Main.qml
app/qml/components/Sidebar.qml
app/qml/views/SettingsView.qml
crates/p3w-platform/src/linux/wallpaper.rs

NOTES
-----
- Focus tracking uses Hyprland's event socket (.socket2.sock), requiring
  HYPRLAND_INSTANCE_SIGNATURE and XDG_RUNTIME_DIR in the application environment.
- The activewindow event identifies desktop focus when no active window class
  exists.
- Autostart uses the currently running executable path. When running from
  target/debug, its desktop entry will point there; update autostart after
  moving or installing the app.
- Changing video audio settings restarts mpvpaper; static images have no sound.
- External processes not owned by Persona 3 Wallpaper are not modified.
- Per-monitor independent audio profiles are not supported by the older engine.
- For animated wallpaper requirements on other Linux desktops, see
  docs/ANIMATED_LINUX.md and run bash scripts/check-animated-linux.sh.
- GNOME and Plasma require optional third-party plugins/extensions; xwinwrap/mpv
  on X11 remains experimental and may cover desktop icons.
- Compile and verify everything on your Linux machine before distribution.
