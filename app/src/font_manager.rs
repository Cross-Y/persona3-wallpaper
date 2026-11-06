//! Desktop-aware Linux font configuration for Persona 3 Wallpaper.
//!
//! This module writes user-scoped settings only. It never edits /etc and never
//! invokes a shell. Desktop-specific settings are best-effort; Fontconfig is
//! the portable fallback for applications that honor it.

use std::{collections::BTreeSet, fs, path::PathBuf, process::Command};
use p3w_platform::linux::{environment::{self, Desktop, DesktopSession}, paths};

pub fn describe_environment() -> String {
    let session = DesktopSession::detect();
    format!("Desktop: {}. Distribution: {}. Compatible settings will be used with Fontconfig as a fallback; some apps may need a restart.",
        session.description(), environment::Distribution::current().pretty_name)
}
fn config_home() -> Result<PathBuf, String> { paths::config_home().ok_or_else(|| "Could not locate XDG_CONFIG_HOME / HOME".into()) }
fn command_exists(name: &str) -> bool { environment::command_exists(name) }

fn xml_escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}
fn set_ini(path: &PathBuf, section: &str, key: &str, value: &str) -> Result<(), String> {
    let mut lines = fs::read_to_string(path).unwrap_or_default().lines().map(str::to_owned).collect::<Vec<_>>();
    let header = format!("[{section}]");
    if let Some(start) = lines.iter().position(|l| l.trim() == header) {
        let end = lines.iter().enumerate().skip(start + 1).find(|(_, l)| l.trim().starts_with('[') && l.trim().ends_with(']')).map(|(i, _)| i).unwrap_or(lines.len());
        if let Some(i) = (start + 1..end).find(|&i| lines[i].split_once('=').map(|(k, _)| k.trim() == key).unwrap_or(false)) {
            lines[i] = format!("{key}={value}");
        } else { lines.insert(end, format!("{key}={value}")); }
    } else {
        if !lines.is_empty() && !lines.last().unwrap().is_empty() { lines.push(String::new()); }
        lines.push(header); lines.push(format!("{key}={value}"));
    }
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    fs::write(path, lines.join("\n")).map_err(|e| format!("Could not write {}: {e}", path.display()))
}
fn set_toml_string(path: &PathBuf, section: &str, key: &str, value: &str) -> Result<(), String> {
    let mut lines = fs::read_to_string(path).unwrap_or_default().lines().map(str::to_owned).collect::<Vec<_>>();
    let header = format!("[{section}]");
    let quoted = serde_json::to_string(value).map_err(|e| e.to_string())?;
    let replacement = format!("{key} = {quoted}");
    if let Some(start) = lines.iter().position(|l| l.trim() == header) {
        let end = lines.iter().enumerate().skip(start + 1).find(|(_, l)| { let t = l.trim(); t.starts_with('[') && t.ends_with(']') }).map(|(i, _)| i).unwrap_or(lines.len());
        if let Some(i) = (start + 1..end).find(|&i| lines[i].split_once('=').map(|(k, _)| k.trim() == key).unwrap_or(false)) { lines[i] = replacement; }
        else { lines.insert(end, replacement); }
    } else {
        if !lines.is_empty() && !lines.last().unwrap().is_empty() { lines.push(String::new()); }
        lines.push(header); lines.push(replacement);
    }
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    fs::write(path, lines.join("\n")).map_err(|e| format!("Could not update TOML configuration: {e}"))
}
fn apply_noctalia(family: &str) -> Result<(), String> {
    let base = std::env::var_os("NOCTALIA_STATE_HOME").map(PathBuf::from).filter(|p| p.is_absolute())
        .or_else(paths::state_home).ok_or("Could not locate XDG_STATE_HOME / HOME")?;
    let path = base.join("noctalia/settings.toml");
    set_toml_string(&path, "shell", "font_family", family)?;
    // IPC is optional: the TOML override is watched and hot-reloaded by Noctalia v5+.
    for binary in ["noctalia", "noctalia-shell"] {
        if command_exists(binary) {
            let value = serde_json::to_string(family).unwrap_or_else(|_| format!("\"{family}\""));
            let _ = Command::new(binary).args(["ipc", "call", "config-set", "shell.font_family", &value]).status();
            let _ = Command::new(binary).args(["ipc", "call", "config-reload"]).status();
            break;
        }
    }
    Ok(())
}
fn apply_gsettings(schema: &str, key: &str, value: &str) -> bool {
    if !command_exists("gsettings") { return false; }
    let schemas = Command::new("gsettings").arg("list-schemas").output().ok();
    let exists = schemas.map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l.trim() == schema)).unwrap_or(false);
    if !exists { return false; }
    Command::new("gsettings").args(["set", schema, key, value]).status().map(|s| s.success()).unwrap_or(false)
}
fn apply_fontconfig(family: &str) -> Result<(), String> {
    let path = config_home()?.join("fontconfig/conf.d/99-persona3-wallpaper-fonts.conf");
    let name = xml_escape(family);
    let xml = format!("<?xml version=\"1.0\"?>\n<!DOCTYPE fontconfig SYSTEM \"urn:fontconfig:fonts.dtd\">\n<fontconfig>\n  <alias><family>sans-serif</family><prefer><family>{name}</family></prefer></alias>\n  <alias><family>sans</family><prefer><family>{name}</family></prefer></alias>\n  <alias><family>system-ui</family><prefer><family>{name}</family></prefer></alias>\n</fontconfig>\n");
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    fs::write(&path, xml).map_err(|e| format!("Could not write Fontconfig settings: {e}"))?;
    if command_exists("fc-cache") { let _ = Command::new("fc-cache").arg("-f").status(); }
    Ok(())
}

pub fn apply_font(family: &str) -> Result<Vec<String>, String> {
    if family.trim().is_empty() || family.chars().any(|c| c == '\n' || c == '\r' || c == '\0') { return Err("Invalid font name".into()); }
    let session = DesktopSession::detect();
    let desktops = vec![session.desktop.clone()];
    let mut applied = BTreeSet::new();
    let mut errors = Vec::new();
    let gtk_font = format!("{family} 10");

    if session.noctalia {
        match apply_noctalia(family) { Ok(()) => { applied.insert("Noctalia".to_string()); }, Err(e) => errors.push(e) }
    }
    for desktop in &desktops {
        match desktop {
            Desktop::Gnome => {
                let mut ok = false;
                for (key, value) in [("font-name", gtk_font.as_str()), ("document-font-name", gtk_font.as_str())] {
                    ok |= apply_gsettings("org.gnome.desktop.interface", key, value);
                }
                if ok { applied.insert("GNOME/GSettings".into()); }
            }
            Desktop::Cinnamon => {
                let mut ok = false;
                for key in ["font-name", "document-font-name"] { ok |= apply_gsettings("org.cinnamon.desktop.interface", key, &gtk_font); }
                if ok { applied.insert("Cinnamon/GSettings".into()); }
            }
            Desktop::Mate => {
                let mut ok = false;
                for key in ["font-name", "document-font-name"] { ok |= apply_gsettings("org.mate.interface", key, &gtk_font); }
                if ok { applied.insert("MATE/GSettings".into()); }
            }
            Desktop::Xfce => {
                if command_exists("xfconf-query") {
                    let exists = Command::new("xfconf-query").args(["-c", "xsettings", "-p", "/Gtk/FontName"]).output().map(|o| o.status.success()).unwrap_or(false);
                    let mut args = vec!["-c", "xsettings", "-p", "/Gtk/FontName"];
                    if !exists { args.extend(["-n", "-t", "string"]); }
                    args.extend(["-s", gtk_font.as_str()]);
                    let ok = Command::new("xfconf-query").args(args).status().map(|s| s.success()).unwrap_or(false);
                    if ok { applied.insert("XFCE/Xfconf".into()); }
                }
            }
            Desktop::Plasma => {
                let path = config_home()?.join("kdeglobals");
                let kde_font = format!("{family},10,-1,5,50,0,0,0,0,0");
                let mut ok = true;
                for key in ["font", "menuFont", "toolBarFont", "activeFont", "smallestReadableFont"] { if set_ini(&path, "General", key, &kde_font).is_err() { ok = false; } }
                if ok { applied.insert("KDE Plasma (kdeglobals)".into()); }
            }
            Desktop::Lxqt => {
                let path = config_home()?.join("lxqt/lxqt.conf");
                let qt_font = format!("{family},10,-1,5,50,0,0,0,0,0");
                if set_ini(&path, "Qt", "font", &qt_font).is_ok() { applied.insert("LXQt".into()); }
            }
            Desktop::Hyprland | Desktop::Sway | Desktop::Other(_) => {}
        }
    }

    // GTK settings.ini is a safe per-user fallback for GTK3 and some GTK4 setups.
    for version in ["gtk-3.0", "gtk-4.0"] {
        let path = config_home()?.join(version).join("settings.ini");
        if set_ini(&path, "Settings", "gtk-font-name", &gtk_font).is_ok() { applied.insert(format!("{version}")); }
    }

    // Qt control-center files are updated only when they already exist; Qt will
    // use them only if the session is configured to load qt5ct/qt6ct.
    let config = config_home()?;
    let qt_font = format!("{family},10,-1,5,50,0,0,0,0,0");
    for path in [config.join("qt5ct/qt5ct.conf"), config.join("qt6ct/qt6ct.conf")] {
        if path.exists() && set_ini(&path, "Fonts", "general", &qt_font).is_ok() { applied.insert("qt5ct/qt6ct".into()); }
    }

    // Portable fallback for applications that resolve their generic UI family through Fontconfig.
    match apply_fontconfig(family) { Ok(()) => { applied.insert("Fontconfig".into()); }, Err(e) => errors.push(e) }

    if applied.is_empty() { return Err(if errors.is_empty() { "No compatible backend was found to apply this font".into() } else { errors.join("; ") }); }
    Ok(applied.into_iter().collect())
}
