//! Desktop-aware Xcursor theme manager.
//!
//! Applies cursor themes through the active compositor/desktop where possible,
//! and writes standard per-user GTK/Xcursor settings as a portable fallback.
//! No system-wide files are modified.

use std::{fs, path::PathBuf, process::Command};
use p3w_platform::linux::{environment::{self, Desktop, DesktopSession}, paths};

fn home() -> Result<PathBuf, String> { paths::home().ok_or_else(|| "Could not locate HOME".into()) }
fn config_home() -> Result<PathBuf, String> { paths::config_home().ok_or_else(|| "Could not locate XDG_CONFIG_HOME / HOME".into()) }
fn command_exists(name: &str) -> bool { environment::command_exists(name) }

fn set_ini(path: &PathBuf, section: &str, key: &str, value: &str) -> Result<(), String> {
    let mut lines = fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let header = format!("[{section}]");
    if let Some(start) = lines.iter().position(|line| line.trim() == header) {
        let end = lines
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, line)| {
                let value = line.trim();
                value.starts_with('[') && value.ends_with(']')
            })
            .map(|(index, _)| index)
            .unwrap_or(lines.len());
        if let Some(index) = (start + 1..end).find(|&index| {
            lines[index]
                .split_once('=')
                .map(|(existing, _)| existing.trim() == key)
                .unwrap_or(false)
        }) {
            lines[index] = format!("{key}={value}");
        } else {
            lines.insert(end, format!("{key}={value}"));
        }
    } else {
        if !lines.is_empty() && !lines.last().unwrap().is_empty() {
            lines.push(String::new());
        }
        lines.push(header);
        lines.push(format!("{key}={value}"));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, lines.join("\n"))
        .map_err(|error| format!("Could not write {}: {error}", path.display()))
}

fn run_ok(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn gsettings_set(schema: &str, key: &str, value: &str) -> bool {
    command_exists("gsettings") && run_ok("gsettings", &["set", schema, key, value])
}

fn write_standard_settings(theme: &str, size: u32) -> Result<Vec<String>, String> {
    let mut applied = Vec::new();
    let config = config_home()?;

    // GTK3 and GTK4 read these per-user settings in common desktop sessions.
    for version in ["gtk-3.0", "gtk-4.0"] {
        let path = config.join(version).join("settings.ini");
        let name_ok = set_ini(&path, "Settings", "gtk-cursor-theme-name", theme).is_ok();
        let size_ok = set_ini(&path, "Settings", "gtk-cursor-theme-size", &size.to_string()).is_ok();
        if name_ok && size_ok {
            applied.push(version.to_string());
        }
    }

    // Xcursor's conventional fallback theme. This is user-scoped and works
    // for X11 clients and toolkits that consult ~/.icons/default.
    let default_theme = home()?.join(".icons/default/index.theme");
    if set_ini(&default_theme, "Icon Theme", "Inherits", theme).is_ok() {
        applied.push("Xcursor (~/.icons/default)".into());
    }

    // Xresources is still honored by a number of X11 applications.
    let xresources = home()?.join(".Xresources");
    let mut contents = fs::read_to_string(&xresources).unwrap_or_default();
    for (key, value) in [("Xcursor.theme", theme.to_string()), ("Xcursor.size", size.to_string())] {
        let prefix = format!("{key}:");
        let replacement = format!("{key}: {value}");
        if let Some(line) = contents.lines().position(|line| line.trim_start().starts_with(&prefix)) {
            let mut lines = contents.lines().map(str::to_owned).collect::<Vec<_>>();
            lines[line] = replacement;
            contents = lines.join("\n");
        } else {
            if !contents.is_empty() && !contents.ends_with('\n') { contents.push('\n'); }
            contents.push_str(&replacement);
            contents.push('\n');
        }
    }
    if let Some(parent) = xresources.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&xresources, contents).map_err(|error| error.to_string())?;
    applied.push("Xresources".into());

    Ok(applied)
}

fn apply_hyprland(theme: &str, size: u32) -> bool {
    DesktopSession::detect().can_use_hyprland_backend()
        && run_ok("hyprctl", &["setcursor", theme, &size.to_string()])
}

fn apply_sway(theme: &str, size: u32) -> bool {
    DesktopSession::detect().desktop == Desktop::Sway
        && command_exists("swaymsg")
        && run_ok("swaymsg", &["seat", "seat0", "xcursor_theme", theme, &size.to_string()])
}

fn apply_kde(theme: &str) -> bool {
    if DesktopSession::detect().desktop != Desktop::Plasma {
        return false;
    }
    let Ok(config) = config_home() else { return false; };
    let path = config.join("kcminputrc");
    let configured = set_ini(&path, "Mouse", "cursorTheme", theme).is_ok();
    if configured {
        for bus in ["qdbus6", "qdbus"] {
            if command_exists(bus) && run_ok(bus, &["org.kde.KWin", "/KWin", "reconfigure"]) {
                break;
            }
        }
    }
    configured
}

fn apply_xfce(theme: &str, size: u32) -> bool {
    if DesktopSession::detect().desktop != Desktop::Xfce || !command_exists("xfconf-query") {
        return false;
    }
    let name_ok = run_ok("xfconf-query", &["-c", "xsettings", "-p", "/Gtk/CursorThemeName", "-n", "-t", "string", "-s", theme])
        || run_ok("xfconf-query", &["-c", "xsettings", "-p", "/Gtk/CursorThemeName", "-s", theme]);
    let size_text = size.to_string();
    let size_ok = run_ok("xfconf-query", &["-c", "xsettings", "-p", "/Gtk/CursorThemeSize", "-n", "-t", "int", "-s", &size_text])
        || run_ok("xfconf-query", &["-c", "xsettings", "-p", "/Gtk/CursorThemeSize", "-s", &size_text]);
    name_ok || size_ok
}

fn apply_lxqt(theme: &str, size: u32) -> bool {
    if DesktopSession::detect().desktop != Desktop::Lxqt { return false; }
    let Ok(config) = config_home() else { return false; };
    let path = config.join("lxqt/lxqt.conf");
    set_ini(&path, "Mouse", "cursor_theme", theme).is_ok()
        && set_ini(&path, "Mouse", "cursor_size", &size.to_string()).is_ok()
}

pub fn apply_cursor_theme(theme: &str, size: u32) -> Result<Vec<String>, String> {
    if theme.trim().is_empty() || theme.contains('/') || theme.contains('\0') || theme.contains('\n') {
        return Err("Invalid cursor theme name".into());
    }
    if size == 0 || size > 256 {
        return Err("Invalid cursor size".into());
    }

    let session = DesktopSession::detect();
    let mut applied = Vec::new();

    if apply_hyprland(theme, size) { applied.push("Hyprland".into()); }
    if apply_sway(theme, size) { applied.push("Sway".into()); }

    if session.desktop == Desktop::Gnome {
        if gsettings_set("org.gnome.desktop.interface", "cursor-theme", theme) {
            applied.push("GNOME/GSettings".into());
        }
    }
    if session.desktop == Desktop::Cinnamon {
        if gsettings_set("org.cinnamon.desktop.interface", "cursor-theme", theme)
            || gsettings_set("org.gnome.desktop.interface", "cursor-theme", theme)
        {
            applied.push("Cinnamon/GSettings".into());
        }
    }
    if session.desktop == Desktop::Mate {
        if gsettings_set("org.mate.peripherals-mouse", "cursor-theme", theme)
            || gsettings_set("org.gnome.desktop.interface", "cursor-theme", theme)
        {
            applied.push("MATE/GSettings".into());
        }
    }
    if apply_kde(theme) { applied.push("KDE Plasma".into()); }
    if apply_xfce(theme, size) { applied.push("XFCE/Xfconf".into()); }
    if apply_lxqt(theme, size) { applied.push("LXQt".into()); }

    let mut standard = write_standard_settings(theme, size)?;
    applied.append(&mut standard);

    // Reload Xresources when running an X11 session; failure is non-fatal.
    if session.session_type == environment::SessionType::X11 && command_exists("xrdb") {
        if let Some(home) = paths::home() {
            let _ = run_ok("xrdb", &["-merge", home.join(".Xresources").to_string_lossy().as_ref()]);
        }
    }

    applied.sort();
    applied.dedup();
    if applied.is_empty() {
        Err("Could not apply the cursor theme with any available backend".into())
    } else {
        Ok(applied)
    }
}

/// Import cursor packages into the user's XDG icon theme directory.
///
/// Python 3 is used only for the archive formats supported by its standard
/// library (ZIP / TAR + gzip, xz, bzip2). The helper is embedded at compile
/// time, so it also works when the app is installed outside its source tree.
/// The helper extracts into a temporary user-owned directory, validates Xcursor
/// data, resolves internal aliases and never overwrites an existing theme.
pub fn import_cursor_archive(archive_url: &str) -> Result<Vec<String>, String> {
    let icons = paths::data_home()
        .ok_or_else(|| "Could not locate XDG_DATA_HOME / HOME".to_string())?
        .join("icons");
    let output = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(include_str!("cursor_archive_helper.py"))
        .arg(archive_url)
        .arg(&icons)
        .output()
        .map_err(|error| format!("Python 3 is required for importing cursor archives: {error}"))?;
    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| {
            let detail = String::from_utf8_lossy(&output.stderr);
            format!("Cursor importer failed: {}", detail.trim())
        })?;
    if !output.status.success() || response["ok"] != true {
        return Err(response["error"].as_str().unwrap_or("Could not import cursor archive").to_string());
    }
    let themes = response["themes"].as_array()
        .ok_or_else(|| "Cursor importer returned an invalid result".to_string())?
        .iter().filter_map(|name| name.as_str().map(str::to_owned)).collect::<Vec<_>>();
    if themes.is_empty() { return Err("No cursor themes were installed".into()); }
    Ok(themes)
}
