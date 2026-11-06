//! Hyprland socket event worker. Only one is spawned per process.
use std::{io::Read, os::unix::net::UnixStream, path::PathBuf, process::Command,
          sync::OnceLock, thread, time::Duration};
use crate::settings_store;
use p3w_platform::linux::{hyprland::HyprlandMonitorProvider, wallpaper::HyprlandWallpaperBackend};

static FOCUS_WORKER: OnceLock<()> = OnceLock::new();

fn active_window_monitor_id() -> Option<Option<i64>> {
    let out = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    if value["class"].as_str().unwrap_or("").is_empty() {
        Some(None)
    } else {
        value["monitor"].as_i64().map(Some)
    }
}

fn update_audio_focus_state() {
    // Desktop-only audio depends on global focus.
    if let Some(monitor_id) = active_window_monitor_id() {
        let _ = HyprlandWallpaperBackend::set_desktop_focused(monitor_id.is_none());
    }
}

pub(crate) fn update_occupancy_state() {
    // Query configuration and windows only when occupancy changes,
    // not on every window focus change.
    let cfg = settings_store::load();
    if cfg["focus_pause"].as_bool().unwrap_or(false) {
        match HyprlandMonitorProvider::new().occupied_monitor_names() {
            Ok(occupied) => {
                let _ = HyprlandWallpaperBackend::set_occupied_monitors(&occupied);
            }
            Err(error) => eprintln!("Could not update monitor occupancy: {error}"),
        }
    } else {
        let _ = HyprlandWallpaperBackend::clear_occupied_pause();
    }
}

pub(crate) fn update_focus_state() {
    update_audio_focus_state();
    update_occupancy_state();
}

pub(crate) fn start_focus_worker() {
    FOCUS_WORKER.get_or_init(|| {
        thread::spawn(|| loop {
            let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
            let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
            let socket = runtime
                .zip(signature)
                .map(|(r, s)| r.join("hypr").join(s).join(".socket2.sock"));
            let Some(socket) = socket else {
                thread::sleep(Duration::from_secs(3));
                continue;
            };
            let Ok(mut stream) = UnixStream::connect(socket) else {
                thread::sleep(Duration::from_secs(3));
                continue;
            };
            update_focus_state();
            let mut buffer = [0u8; 4096];
            let mut pending = String::new();
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        pending.push_str(&String::from_utf8_lossy(&buffer[..n]));
                        // A malformed/compositor socket must not grow memory without bound.
                        if pending.len() > 64 * 1024 {
                            pending.clear();
                            eprintln!("Hyprland event buffer exceeded 64 KiB; resynchronizing");
                            update_focus_state();
                            continue;
                        }
                        let mut audio_dirty = false;
                        let mut occupancy_dirty = false;
                        while let Some(pos) = pending.find('\n') {
                            let event = pending[..pos].to_string();
                            pending.drain(..=pos);

                            // Global focus affects audio; these events also help
                            // resynchronize occupancy if a window changes state.
                            if event.starts_with("activewindow>>")
                                || event.starts_with("activewindowv2>>")
                                || event.starts_with("focusedmon>>")
                                || event.starts_with("focusedmonv2>>")
                            {
                                audio_dirty = true;
                            }

                            // Only these events can change actual occupancy.
                            if event.starts_with("openwindow>>")
                                || event.starts_with("closewindow>>")
                                || event.starts_with("activewindow>>")
                                || event.starts_with("activewindowv2>>")
                                || event.starts_with("focusedmon>>")
                                || event.starts_with("focusedmonv2>>")
                                || event.starts_with("movewindow>>")
                                || event.starts_with("movewindowv2>>")
                                || event.starts_with("workspace>>")
                                || event.starts_with("workspacev2>>")
                                || event.starts_with("changefloatingmode>>")
                                || event.starts_with("fullscreen>>")
                                || event.starts_with("minimize>>")
                                || event.starts_with("pin>>")
                                || event.starts_with("monitoradded>>")
                                || event.starts_with("monitorremoved>>")
                                || event.starts_with("monitoraddedv2>>")
                                || event.starts_with("monitorremovedv2>>")
                                || event.starts_with("activespecial>>")
                                || event.starts_with("createworkspace>>")
                                || event.starts_with("destroyworkspace>>")
                            {
                                occupancy_dirty = true;
                            }
                        }

                        if audio_dirty || occupancy_dirty {
                            // Debounce bursts. Client and monitor queries
                            // are performed only when occupancy changes.
                            thread::sleep(Duration::from_millis(80));
                            if audio_dirty {
                                update_audio_focus_state();
                            }
                            if occupancy_dirty {
                                update_occupancy_state();
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            thread::sleep(Duration::from_secs(2));
        });
    });
}
