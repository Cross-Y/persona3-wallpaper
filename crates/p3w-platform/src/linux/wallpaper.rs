use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::linux::{file_identity::file_identity, environment::{Desktop, DesktopSession}, hyprland::HyprlandMonitorProvider, sway::SwayMonitorProvider, paths};
use crate::monitor::MonitorProvider;
use crate::wallpaper::{WallpaperApplier, WallpaperKind};

#[derive(Clone)]
struct WallpaperSpec {
    path: PathBuf,
    kind: WallpaperKind,
    version: Option<(u64, std::time::SystemTime)>,
}

struct ActiveWallpaper {
    child: Child,
    ipc_socket: Option<PathBuf>,
    kind: WallpaperKind,
    /// Last pause state successfully sent through mpv IPC. None means not yet synchronized.
    paused: Option<bool>,
}

#[derive(Default)]
struct WallpaperRegistry {
    active: HashMap<String, ActiveWallpaper>,
    desired: HashMap<String, WallpaperSpec>,
    /// Manual pause from the application's main toggle.
    paused: bool,
    /// Monitors occupied by one or more visible windows, regardless of focus.
    occupied_paused: HashSet<String>,
    sound_enabled: bool,
    audio_desktop_only: bool,
    desktop_focused: bool,
}

static WALLPAPERS: OnceLock<Mutex<WallpaperRegistry>> = OnceLock::new();
fn registry() -> &'static Mutex<WallpaperRegistry> {
    WALLPAPERS.get_or_init(|| {
        Mutex::new(WallpaperRegistry {
            sound_enabled: false,
            desktop_focused: true,
            ..Default::default()
        })
    })
}

fn runtime_directory() -> Result<PathBuf, String> {
    let runtime = paths::runtime_dir().ok_or("XDG_RUNTIME_DIR must be set to an absolute path")?;
    let dir = runtime.join("persona3-wallpaper");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Could not create runtime directory: {e}"))?;
    Ok(dir)
}

fn socket_for_monitor(monitor: &str) -> Result<PathBuf, String> {
    let safe_name: String = monitor
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(runtime_directory()?.join(format!("mpv-{safe_name}.sock")))
}

fn effective_sound(state: &WallpaperRegistry) -> bool {
    state.sound_enabled && (!state.audio_desktop_only || state.desktop_focused)
}

fn should_pause(state: &WallpaperRegistry, monitor: &str) -> bool {
    state.paused || state.occupied_paused.contains(monitor)
}

fn spawn_wallpaper(
    path: &Path,
    monitor: &str,
    kind: WallpaperKind,
    sound: bool,
) -> Result<ActiveWallpaper, String> {
    let mut ipc_socket = None;
    let mut command = match kind {
        WallpaperKind::Image => {
            let mut c = Command::new("swaybg");
            c.args(["-o", monitor, "-i"]).arg(path).args(["-m", "fill"]);
            c
        }
        WallpaperKind::Video => {
            let socket = socket_for_monitor(monitor)?;
            let _ = std::fs::remove_file(&socket);
            // Use hardware decoding when available (NVDEC/VAAPI),
            // fewer CPU threads, and late-frame dropping to reduce stutter
            // while playing multiple wallpapers simultaneously.
            let options = format!(
                "--loop-file=inf --input-ipc-server={} --mute={} --hwdec=auto-safe --vd-lavc-threads=2 --framedrop=vo --osc=no --osd-level=0",
                socket.display(),
                if sound { "no" } else { "yes" }
            );
            let mut c = Command::new("mpvpaper");
            c.args(["-o", options.as_str(), monitor]).arg(path);
            ipc_socket = Some(socket);
            c
        }
    };

    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| match kind {
            WallpaperKind::Image => format!("Could not start swaybg: {e}"),
            WallpaperKind::Video => format!("Could not start mpvpaper: {e}"),
        })?;

    Ok(ActiveWallpaper {
        child,
        ipc_socket,
        kind,
        paused: None,
    })
}

fn set_monitor_paused(active: &mut ActiveWallpaper, paused: bool) {
    if active.kind != WallpaperKind::Video || active.paused == Some(paused) {
        return;
    }

    let Some(socket) = &active.ipc_socket else {
        return;
    };

    let message = serde_json::json!({"command": ["set_property", "pause", paused]});

    // mpvpaper can need a short moment to create its IPC socket after spawn.
    // Retry briefly, but never block the event worker for almost half a second.
    for _ in 0..8 {
        if let Ok(mut stream) = UnixStream::connect(socket) {
            if writeln!(stream, "{message}").is_ok() {
                active.paused = Some(paused);
                return;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
}


fn stop_child(active: &mut ActiveWallpaper) {
    let _ = active.child.kill();
    let _ = active.child.wait();
    if let Some(socket) = &active.ipc_socket {
        let _ = std::fs::remove_file(socket);
    }
}

fn apply_pause_states(state: &mut WallpaperRegistry) {
    // Calculate targets first so we can mutably update each player without
    // repeatedly borrowing the registry or sending duplicate IPC commands.
    let targets: Vec<(String, bool)> = state
        .active
        .keys()
        .map(|monitor| (monitor.clone(), should_pause(state, monitor)))
        .collect();

    for (monitor, paused) in targets {
        if let Some(active) = state.active.get_mut(&monitor) {
            set_monitor_paused(active, paused);
        }
    }
}

fn stop_active(state: &mut WallpaperRegistry) {
    for (_, mut active) in state.active.drain() {
        stop_child(&mut active);
    }
}

pub struct HyprlandWallpaperBackend;
impl HyprlandWallpaperBackend {
    pub fn new() -> Self {
        Self
    }

    /// Pause animation everywhere but keep each wallpaper process and frame visible.
    pub fn pause_all() -> Result<(), String> {
        let mut state = registry()
            .lock()
            .map_err(|_| "Wallpaper registry unavailable".to_string())?;
        state.paused = true;
        apply_pause_states(&mut state);
        Ok(())
    }

    pub fn resume_all() -> Result<(), String> {
        let mut state = registry()
            .lock()
            .map_err(|_| "Wallpaper registry unavailable".to_string())?;
        state.paused = false;
        apply_pause_states(&mut state);
        Ok(())
    }

    /// True only for the manual global pause switch, not per-monitor focus pauses.
    pub fn is_paused() -> Result<bool, String> {
        registry()
            .lock()
            .map(|s| s.paused)
            .map_err(|_| "Wallpaper registry unavailable".into())
    }

    /// Update all monitors with visible windows.
    /// Pause every occupied output, not just the one containing the focused window.
    pub fn set_occupied_monitors(occupied_names: &HashSet<String>) -> Result<(), String> {
        let mut state = registry()
            .lock()
            .map_err(|_| "Wallpaper registry unavailable".to_string())?;
        state.occupied_paused = occupied_names.clone();
        apply_pause_states(&mut state);
        Ok(())
    }

    pub fn clear_occupied_pause() -> Result<(), String> {
        let mut state = registry()
            .lock()
            .map_err(|_| "Wallpaper registry unavailable".to_string())?;
        state.occupied_paused.clear();
        apply_pause_states(&mut state);
        Ok(())
    }

    /// Stop every wallpaper process while keeping the saved/desired selections.
    /// The UI can restore them later without losing the selected wallpaper.
    pub fn stop_all() -> Result<(), String> {
        let mut state = registry()
            .lock()
            .map_err(|_| "Wallpaper registry unavailable".to_string())?;
        stop_active(&mut state);
        Ok(())
    }

    pub fn set_audio_settings(enabled: bool, desktop_only: bool) -> Result<(), String> {
        let mut state = registry()
            .lock()
            .map_err(|_| "Wallpaper registry unavailable".to_string())?;
        let before = effective_sound(&state);
        state.sound_enabled = enabled;
        state.audio_desktop_only = desktop_only;
        if before != effective_sound(&state) {
            let mute = !effective_sound(&state);
            for active in state.active.values() {
                if active.kind == WallpaperKind::Video {
                    if let Some(socket) = &active.ipc_socket {
                        let message =
                            serde_json::json!({"command": ["set_property", "mute", mute]});
                        if let Ok(mut stream) = UnixStream::connect(socket) {
                            let _ = writeln!(stream, "{message}");
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn set_desktop_focused(focused: bool) -> Result<(), String> {
        let mut state = registry()
            .lock()
            .map_err(|_| "Wallpaper registry unavailable".to_string())?;
        let before = effective_sound(&state);
        state.desktop_focused = focused;
        if before != effective_sound(&state) {
            let mute = !effective_sound(&state);
            for active in state.active.values() {
                if active.kind == WallpaperKind::Video {
                    if let Some(socket) = &active.ipc_socket {
                        let message =
                            serde_json::json!({"command": ["set_property", "mute", mute]});
                        if let Ok(mut stream) = UnixStream::connect(socket) {
                            let _ = writeln!(stream, "{message}");
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Stop children only when the application itself exits.
    pub fn shutdown() -> Result<(), String> {
        let mut state = registry()
            .lock()
            .map_err(|_| "Wallpaper registry unavailable".to_string())?;
        stop_active(&mut state);
        Ok(())
    }
}
impl Default for HyprlandWallpaperBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WallpaperApplier for HyprlandWallpaperBackend {
    fn apply(&self, path: &Path, monitor: &str, kind: WallpaperKind) -> Result<(), String> {
        if !path.is_file() {
            return Err(format!("Wallpaper file does not exist: {}", path.display()));
        }
        if monitor.trim().is_empty() {
            return Err("No monitor was selected".into());
        }
        let monitors = match DesktopSession::detect().desktop {
            Desktop::Hyprland => HyprlandMonitorProvider::new().monitors()?,
            Desktop::Sway => SwayMonitorProvider.monitors()?,
            _ => return Err("The swaybg/mpvpaper player only supports Hyprland and Sway".into()),
        };
        if !monitors.iter().any(|m| m.name == monitor) {
            return Err(format!("Monitor '{monitor}' is not connected"));
        }
        let mut state = registry()
            .lock()
            .map_err(|_| "Wallpaper registry unavailable".to_string())?;
        let spec = WallpaperSpec {
            path: path.to_path_buf(),
            kind,
            version: file_identity(path),
        };
        let unchanged = spec.version.is_some() && state.desired.get(monitor)
            .map(|old| old.path == spec.path && old.kind == spec.kind && old.version == spec.version)
            .unwrap_or(false);
        if unchanged {
            if let Some(active) = state.active.get_mut(monitor) {
                // Avoid a costly mpvpaper/swaybg restart for identical live media.
                if matches!(active.child.try_wait(), Ok(None)) { return Ok(()); }
            }
        }
        state.desired.insert(monitor.to_string(), spec.clone());
        if let Some(mut old) = state.active.remove(monitor) {
            stop_child(&mut old);
        }
        let active = spawn_wallpaper(&spec.path, monitor, spec.kind, effective_sound(&state))?;
        state.active.insert(monitor.to_string(), active);
        apply_pause_states(&mut state);
        Ok(())
    }
}
