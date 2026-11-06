//! Shared Linux desktop/session detection. Choose a backend by the active
//! *desktop session*, never by the distribution or by a package manager.
use std::{env, fs, os::unix::fs::PermissionsExt, process::{Command, Stdio}};

use super::paths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Desktop {
    Hyprland, Sway, Gnome, Plasma, Xfce, Cinnamon, Mate, Lxqt, Other(String),
}
impl Desktop {
    pub fn label(&self) -> &str {
        match self {
            Self::Hyprland => "Hyprland", Self::Sway => "Sway", Self::Gnome => "GNOME",
            Self::Plasma => "KDE Plasma", Self::Xfce => "XFCE", Self::Cinnamon => "Cinnamon",
            Self::Mate => "MATE", Self::Lxqt => "LXQt", Self::Other(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType { Wayland, X11, Unknown }
impl SessionType {
    pub fn label(self) -> &'static str {
        match self { Self::Wayland => "Wayland", Self::X11 => "X11", Self::Unknown => "unknown" }
    }
}

#[derive(Debug, Clone)]
pub struct Distribution { pub id: String, pub id_like: Vec<String>, pub pretty_name: String }

impl Distribution {
    pub fn current() -> Self { Self::from_os_release(&fs::read_to_string("/etc/os-release").unwrap_or_default()) }
    pub fn from_os_release(source: &str) -> Self {
        fn unquote(value: &str) -> String {
            let value = value.trim();
            if value.len() >= 2 && ((value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\''))) {
                value[1..value.len()-1].replace("\\\"", "\"").replace("\\\\", "\\")
            } else { value.to_string() }
        }
        let mut d = Self { id: "linux".into(), id_like: Vec::new(), pretty_name: "Linux".into() };
        for line in source.lines() {
            if line.starts_with('#') { continue; }
            let Some((key, value)) = line.split_once('=') else { continue; };
            let value = unquote(value);
            match key {
                "ID" => d.id = value,
                "ID_LIKE" => d.id_like = value.split_whitespace().map(String::from).collect(),
                "PRETTY_NAME" => d.pretty_name = value,
                _ => {}
            }
        }
        d
    }
}

#[derive(Debug, Clone)]
pub struct DesktopSession {
    pub desktop: Desktop,
    pub session_type: SessionType,
    pub noctalia: bool,
    pub hyprland_active: bool,
}

fn desktop_tokens() -> Vec<String> {
    ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"]
        .iter().filter_map(|k| env::var(k).ok())
        .flat_map(|value| value.split(|c| c == ':' || c == ';' || c == ',').map(|v| v.trim().to_ascii_lowercase()).collect::<Vec<_>>())
        .filter(|token| !token.is_empty()).collect()
}
fn has_token(tokens: &[String], name: &str) -> bool { tokens.iter().any(|t| t == name) }

/// Exact desktop tokens avoid substring bugs (e.g. MATE vs ULTIMATE).
pub fn desktop_from_tokens(tokens: &[String], hyprland_signature: bool) -> Desktop {
    if hyprland_signature || has_token(tokens, "hyprland") { Desktop::Hyprland }
    else if has_token(tokens, "sway") { Desktop::Sway }
    else if has_token(tokens, "kde") || has_token(tokens, "plasma") { Desktop::Plasma }
    else if has_token(tokens, "gnome") || has_token(tokens, "gnome-classic") || has_token(tokens, "ubuntu") { Desktop::Gnome }
    else if has_token(tokens, "xfce") || has_token(tokens, "xfce4") { Desktop::Xfce }
    else if has_token(tokens, "cinnamon") || has_token(tokens, "x-cinnamon") { Desktop::Cinnamon }
    else if has_token(tokens, "mate") { Desktop::Mate }
    else if has_token(tokens, "lxqt") { Desktop::Lxqt }
    else { Desktop::Other(tokens.first().cloned().unwrap_or_else(|| "Generic desktop".into())) }
}

pub fn command_exists(name: &str) -> bool {
    if name.contains('/') { return false; }
    env::var_os("PATH").map(|p| env::split_paths(&p).any(|d| {
        fs::metadata(d.join(name)).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
    })).unwrap_or(false)
}
fn process_running(name: &str) -> bool {
    command_exists("pgrep") && Command::new("pgrep").args(["-x", name])
        .stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

impl DesktopSession {
    pub fn detect() -> Self {
        let tokens = desktop_tokens();
        let signature = env::var_os("HYPRLAND_INSTANCE_SIGNATURE").filter(|v| !v.is_empty()).is_some();
        let desktop = desktop_from_tokens(&tokens, signature);
        let session_type = match env::var("XDG_SESSION_TYPE").unwrap_or_default().to_ascii_lowercase().as_str() {
            "wayland" => SessionType::Wayland, "x11" => SessionType::X11,
            _ if env::var_os("WAYLAND_DISPLAY").is_some() => SessionType::Wayland,
            _ if env::var_os("DISPLAY").is_some() => SessionType::X11,
            _ => SessionType::Unknown,
        };
        // Noctalia is an overlay, not a distribution or compositor.
        let noctalia_config = env::var_os("NOCTALIA_CONFIG_HOME").map(std::path::PathBuf::from)
            .filter(|p| p.is_absolute()).or_else(paths::config_home);
        let noctalia_state = env::var_os("NOCTALIA_STATE_HOME").map(std::path::PathBuf::from)
            .filter(|p| p.is_absolute()).or_else(paths::state_home);
        let noctalia_configured = noctalia_config.map(|p| p.join("noctalia").is_dir()).unwrap_or(false)
            || noctalia_state.map(|p| p.join("noctalia/settings.toml").exists()).unwrap_or(false);
        let noctalia = has_token(&tokens, "noctalia")
            || (session_type == SessionType::Wayland && (process_running("noctalia") || process_running("noctalia-shell")
                || (noctalia_configured && process_running("quickshell"))));
        Self { hyprland_active: signature, desktop, session_type, noctalia }
    }
    pub fn description(&self) -> String {
        format!("{}{} · {}", self.desktop.label(), if self.noctalia { " + Noctalia" } else { "" }, self.session_type.label())
    }
    pub fn can_use_hyprland_backend(&self) -> bool {
        self.desktop == Desktop::Hyprland && self.hyprland_active && command_exists("hyprctl")
    }
    pub fn wallpaper_backend_error(&self) -> Option<String> {
        super::universal_wallpaper::UniversalWallpaperBackend::from_session(self.clone())
            .support(crate::wallpaper::WallpaperKind::Image).err()
    }
    pub fn can_apply_kind(&self, video: bool) -> Result<(), String> {
        super::universal_wallpaper::UniversalWallpaperBackend::from_session(self.clone())
            .support(if video { crate::wallpaper::WallpaperKind::Video } else { crate::wallpaper::WallpaperKind::Image })
    }

}

pub fn diagnostics() -> String {
    let distro = Distribution::current();
    let session = DesktopSession::detect();
    let backend = super::universal_wallpaper::UniversalWallpaperBackend::from_session(session.clone());
    let image = match backend.support(crate::wallpaper::WallpaperKind::Image) { Ok(()) => "available".into(), Err(e) => e };
    let video = match backend.support(crate::wallpaper::WallpaperKind::Video) { Ok(()) => "available".into(), Err(e) => e };
    format!("Persona 3 Wallpaper — Linux Diagnostics\nDistribution: {} (ID={})\nSession: {}\nBackend: {}\nImages: {}\nVideos: {}\nPer-monitor: {}\nPlayback/pause: {}\nFonts: {}",
        distro.pretty_name, distro.id, session.description(), backend.backend.description(), image, video,
        if backend.backend.per_monitor() { "yes" } else { "no (all monitors)" },
        if backend.playback_controls() { "yes" } else { "no (uses native settings)" },
        if command_exists("fc-list") { "Fontconfig available" } else { "fc-list not found" })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tokens(list: &[&str]) -> Vec<String> { list.iter().map(|s| s.to_string()).collect() }
    #[test]
    fn distro_ids_are_data_not_backend_selection() {
        let dist = Distribution::from_os_release("NAME=Fedora\nID=fedora\nID_LIKE=\"rhel centos\"\nPRETTY_NAME=\"Fedora Linux 43\"");
        assert_eq!(dist.id, "fedora");
        assert_eq!(dist.id_like, vec!["rhel", "centos"]);
        assert_eq!(dist.pretty_name, "Fedora Linux 43");
    }
    #[test]
    fn detects_hyprland_with_overlay_and_known_desktops() {
        assert_eq!(desktop_from_tokens(&tokens(&["hyprland", "noctalia"]), false), Desktop::Hyprland);
        assert_eq!(desktop_from_tokens(&tokens(&["kde"]), false), Desktop::Plasma);
        assert_eq!(desktop_from_tokens(&tokens(&["gnome"]), false), Desktop::Gnome);
        assert_eq!(desktop_from_tokens(&tokens(&["xfce"]), false), Desktop::Xfce);
        assert_eq!(desktop_from_tokens(&tokens(&["sway"]), false), Desktop::Sway);
        assert_eq!(desktop_from_tokens(&tokens(&["mate"]), false), Desktop::Mate);
        assert_eq!(desktop_from_tokens(&tokens(&["cinnamon"]), false), Desktop::Cinnamon);
        assert_eq!(desktop_from_tokens(&tokens(&["lxqt"]), false), Desktop::Lxqt);
        assert_eq!(desktop_from_tokens(&tokens(&["ubuntu", "kde"]), false), Desktop::Plasma);
        assert_eq!(desktop_from_tokens(&tokens(&["other"]), true), Desktop::Hyprland);
        assert!(matches!(desktop_from_tokens(&tokens(&["ultimate"]), false), Desktop::Other(_)));
    }
}
