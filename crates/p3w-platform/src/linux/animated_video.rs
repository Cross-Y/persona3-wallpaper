//! Non-Hyprland video engines: xwinwrap+mpv on X11 and mpvpaper on
//! layer-shell Wayland compositors. External tools are optional dependencies.
//! The application owns only its own process group and IPC socket.
use std::{io::Write, os::unix::{net::UnixStream, process::CommandExt}, path::{Path, PathBuf},
    process::{Child, Command, Stdio}, sync::{Mutex, OnceLock}, thread, time::Duration};
use super::{file_identity::file_identity, paths};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode { X11, WaylandLayer }

struct Player { child: Child, socket: PathBuf, paused: bool }
struct State {
    active: Option<Player>,
    current: Option<(PathBuf, Mode, Option<(u64, std::time::SystemTime)>)>,
    sound_enabled: bool,
    paused: bool,
}
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State { active: None, current: None, sound_enabled: false, paused: false }))
}

fn socket_path() -> Result<PathBuf, String> {
    let root = paths::runtime_dir().ok_or("XDG_RUNTIME_DIR is not configured")?.join("persona3-wallpaper");
    std::fs::create_dir_all(&root).map_err(|e| format!("Could not create IPC directory: {e}"))?;
    Ok(root.join("mpv-generic.sock"))
}
fn mpv_ipc(socket: &Path, property: &str, value: bool) -> bool {
    let Ok(mut stream) = UnixStream::connect(socket) else { return false; };
    let request = serde_json::json!({"command": ["set_property", property, value]});
    writeln!(stream, "{request}").is_ok()
}
fn stop(player: &mut Player) {
    // A group is required: on X11 mpv is a descendant of xwinwrap.
    let pgid = player.child.id() as i32;
    unsafe { libc::kill(-pgid, libc::SIGTERM); }
    for _ in 0..10 {
        if player.child.try_wait().ok().flatten().is_some() { break; }
        thread::sleep(Duration::from_millis(25));
    }
    if player.child.try_wait().ok().flatten().is_none() {
        unsafe { libc::kill(-pgid, libc::SIGKILL); }
        let _ = player.child.kill();
    }
    let _ = player.child.wait();
    let _ = std::fs::remove_file(&player.socket);
}
fn spawn(path: &Path, mode: Mode, sound: bool) -> Result<Player, String> {
    let socket = socket_path()?;
    let _ = std::fs::remove_file(&socket);
    let mut command = match mode {
        Mode::X11 => {
            let mut c = Command::new("xwinwrap");
            c.args(["-fs", "-ni", "-s", "-nf", "-b", "-un", "-fdt", "-ov", "--",
                // Fixed shell fragment. The WID placeholder and path are POSITIONAL arguments,
                // never interpolated into shell source (spaces and quotes are safe).
                "sh", "-c", "wid=$1; shift; exec mpv \"--wid=$wid\" \"$@\"", "p3w-xwinwrap", "WID"]);
            c
        }
        Mode::WaylandLayer => {
            let mut c = Command::new("mpvpaper");
            let options = format!("--loop-file=inf --input-ipc-server={} --mute={} --hwdec=auto-safe --vd-lavc-threads=2 --framedrop=vo --osc=no --osd-level=0", socket.display(), if sound { "no" } else { "yes" });
            c.args(["-o", &options, "*"]).arg(path);
            c
        }
    };
    if mode == Mode::X11 {
        command.args(["--loop-file=inf", "--hwdec=auto-safe", "--vd-lavc-threads=2", "--framedrop=vo", "--osc=no", "--osd-level=0"]);
        command.arg(format!("--input-ipc-server={}", socket.display()));
        command.arg(if sound { "--mute=no" } else { "--mute=yes" }).arg(path);
    }
    // Put external wallpaper in its own process group; never kill unrelated players.
    command.process_group(0).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = command.spawn().map_err(|e| format!("Could not start the player ({mode:?}): {e}"))?;
    // Detect common early failures instead of claiming success when a command exits immediately.
    for _ in 0..30 {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let _ = std::fs::remove_file(&socket);
            return Err(format!("The player exited on startup ({mode:?}, code {status}). Check dependencies and compositor compatibility."));
        }
        if socket.exists() { break; }
        thread::sleep(Duration::from_millis(80));
    }
    if !socket.exists() {
        let mut incomplete = Player { child, socket, paused: false };
        stop(&mut incomplete);
        return Err("The player did not create its IPC socket; check mpvpaper/xwinwrap logs and desktop compatibility.".into());
    }
    Ok(Player { child, socket, paused: false })
}

pub struct GenericVideoBackend;
impl GenericVideoBackend {
    pub fn apply(path: &Path, mode: Mode) -> Result<(), String> {
        let mut guard = state().lock().map_err(|_| "Video manager is unavailable".to_string())?;
        let version = file_identity(path);
        let unchanged = version.is_some() && guard.current.as_ref()
            .map(|(old_path, old_mode, old_version)| old_path.as_path() == path && *old_mode == mode && *old_version == version)
            .unwrap_or(false);
        if unchanged {
            if let Some(active) = guard.active.as_mut() {
                if matches!(active.child.try_wait(), Ok(None)) { return Ok(()); }
            }
        }
        if let Some(mut previous) = guard.active.take() { stop(&mut previous); }
        guard.current = None;
        let mut player = spawn(path, mode, guard.sound_enabled)?;
        if guard.paused { for _ in 0..8 { if mpv_ipc(&player.socket, "pause", true) { break; } thread::sleep(Duration::from_millis(25)); } player.paused = true; }
        guard.active = Some(player);
        guard.current = Some((path.to_path_buf(), mode, version));
        Ok(())
    }
    pub fn pause_all() -> Result<(), String> { Self::set_pause(true) }
    pub fn resume_all() -> Result<(), String> { Self::set_pause(false) }
    fn set_pause(paused: bool) -> Result<(), String> {
        let mut guard = state().lock().map_err(|_| "Video manager is unavailable".to_string())?;
        guard.paused = paused;
        if let Some(player) = guard.active.as_mut() { if mpv_ipc(&player.socket, "pause", paused) { player.paused = paused; } }
        Ok(())
    }
    pub fn is_paused() -> bool { state().lock().map(|s| s.paused).unwrap_or(false) }
    pub fn set_audio(enabled: bool) -> Result<(), String> {
        let mut guard = state().lock().map_err(|_| "Video manager is unavailable".to_string())?;
        guard.sound_enabled = enabled;
        if let Some(player) = guard.active.as_mut() { let _ = mpv_ipc(&player.socket, "mute", !enabled); }
        Ok(())
    }
    pub fn stop_all() -> Result<(), String> {
        let mut guard = state().lock().map_err(|_| "Video manager is unavailable".to_string())?;
        if let Some(mut player) = guard.active.take() { stop(&mut player); }
        guard.current = None;
        Ok(())
    }
}
