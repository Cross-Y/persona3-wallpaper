//! Desktop-entry autostart management (no shell interpolation).
use std::{path::{Path, PathBuf}, fs};
use p3w_platform::linux::paths;

fn autostart_path() -> Result<PathBuf, String> {
    paths::config_home().map(|p| p.join("autostart/persona3-wallpaper.desktop"))
        .ok_or_else(|| "Could not locate XDG_CONFIG_HOME / HOME".into())
}

/// Escape a desktop-entry Exec command according to freedesktop quoting rules.
/// This is NOT shell quoting: desktop launchers parse the Exec field themselves.
fn desktop_exec(exe: &Path) -> Result<String, String> {
    let name = exe.to_str().ok_or("The executable path is not valid UTF-8")?;
    if !exe.is_absolute() || name.chars().any(|c| c == '\n' || c == '\r' || c == '\0') {
        return Err("Invalid executable path for autostart".into());
    }
    let value = name.replace('%', "%%").replace('\\', "\\\\").replace('"', "\\\"")
        .replace('`', "\\`").replace('$', "\\$");
    Ok(format!("\"{value}\""))
}

/// An AppImage runs from a temporary mount: current_exe() is not a stable
/// path across launches. The AppImage runtime exposes the original path here.
fn stable_executable() -> Result<PathBuf, String> {
    if let Some(original) = std::env::var_os("APPIMAGE") {
        let path = PathBuf::from(original);
        if path.is_absolute() && path.is_file() { return Ok(path); }
    }
    std::env::current_exe().map_err(|e| format!("Could not locate the executable: {e}"))
}

pub(crate) fn set_autostart(enabled: bool) -> Result<(), String> {
    let path = autostart_path()?;
    if enabled {
        let exe = stable_executable()?;
        let exec = desktop_exec(&exe)?;
        let contents = format!("[Desktop Entry]\nType=Application\nName=Persona 3 Wallpaper\nComment=Restore desktop wallpapers\nExec={exec}\nIcon=persona3-wallpaper\nTerminal=false\nX-P3W-Managed=true\n");
        // Avoid unnecessary disk writes every time the app is started.
        if fs::read_to_string(&path).ok().as_deref() == Some(contents.as_str()) {
            return Ok(());
        }
        let parent = path.parent().ok_or("Invalid autostart path")?;
        fs::create_dir_all(parent).map_err(|e| format!("Could not create autostart entry: {e}"))?;
        fs::write(path, contents).map_err(|e| format!("Could not enable autostart: {e}"))
    } else if path.exists() {
        fs::remove_file(path).map_err(|e| format!("Could not disable autostart: {e}"))
    } else { Ok(()) }
}

