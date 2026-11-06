//! Dispatch wallpaper operations by desktop, not by distribution.
//! No shell interpolation, no root privileges and no rewriting desktop config files.
use std::{os::unix::ffi::OsStrExt, path::Path, process::Command};
use crate::{monitor::{Monitor, MonitorProvider}, wallpaper::{WallpaperApplier, WallpaperKind}};
use super::{animated_video::{GenericVideoBackend, Mode}, environment::{self, Desktop, DesktopSession, SessionType}, paths, video_optimizer::{self, DisplaySize}, hyprland::HyprlandMonitorProvider,
    sway::SwayMonitorProvider, wallpaper::HyprlandWallpaperBackend};

pub const ALL_DISPLAYS: &str = "all";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend { Hyprland, Sway, Gnome, Plasma, Xfce, Cinnamon, Mate, Lxqt, Unsupported }

impl Backend {
    pub fn id(self) -> &'static str {
        match self {
            Self::Hyprland => "hyprland", Self::Sway => "sway", Self::Gnome => "gnome",
            Self::Plasma => "plasma", Self::Xfce => "xfce", Self::Cinnamon => "cinnamon",
            Self::Mate => "mate", Self::Lxqt => "lxqt", Self::Unsupported => "unsupported",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Hyprland => "Hyprland · swaybg/mpvpaper", Self::Sway => "Sway · swaybg/mpvpaper",
            Self::Gnome => "GNOME · GSettings", Self::Plasma => "KDE Plasma · PlasmaShell D-Bus",
            Self::Xfce => "XFCE · xfconf-query", Self::Cinnamon => "Cinnamon · GSettings",
            Self::Mate => "MATE · GSettings", Self::Lxqt => "LXQt · PCManFM-Qt",
            Self::Unsupported => "No compatible adapter",
        }
    }
    pub fn from_desktop(desktop: &Desktop) -> Self {
        match desktop {
            Desktop::Hyprland => Self::Hyprland, Desktop::Sway => Self::Sway,
            Desktop::Gnome => Self::Gnome, Desktop::Plasma => Self::Plasma,
            Desktop::Xfce => Self::Xfce, Desktop::Cinnamon => Self::Cinnamon,
            Desktop::Mate => Self::Mate, Desktop::Lxqt => Self::Lxqt,
            Desktop::Other(_) => Self::Unsupported,
        }
    }
    pub fn controls_playback(self) -> bool { matches!(self, Self::Hyprland | Self::Sway) }
    pub fn per_monitor(self) -> bool { self.controls_playback() }
    pub fn supports_video(self) -> bool { self != Self::Unsupported } // Session-specific support is checked by UniversalWallpaperBackend.
    fn required_tool(self, kind: WallpaperKind) -> Option<&'static str> {
        match self {
            Self::Hyprland | Self::Sway => Some(if kind == WallpaperKind::Video { "mpvpaper" } else { "swaybg" }),
            Self::Gnome | Self::Cinnamon | Self::Mate => Some("gsettings"),
            Self::Plasma => None, // Any supported D-Bus CLI works.
            Self::Xfce => Some("xfconf-query"), Self::Lxqt => Some("pcmanfm-qt"),
            Self::Unsupported => None,
        }
    }
}

/// Runtime-specific route for animated wallpapers. GNOME/Plasma require a
/// compositor-integrated extension; mpvpaper cannot draw behind GNOME Shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VideoRoute { ExistingWayland, Generic(Mode), GnomeExtension, PlasmaPlugin }

const GNOME_VIDEO_SCHEMA: &str = "org.gnome.shell.extensions.wallpaperengine";
const GNOME_VIDEO_UUID: &str = "wallpaperengine@waylandwe";
const PLASMA_VIDEO_PLUGIN: &str = "luisbocanegra.smart.video.wallpaper.reborn";

fn installed_plasma_plugin() -> bool {
    std::iter::once(paths::data_home()).flatten().chain(paths::data_dirs())
        .any(|root| root.join("plasma/wallpapers").join(PLASMA_VIDEO_PLUGIN).join("metadata.json").is_file())
}
fn gnome_extension_enabled() -> bool {
    if !environment::command_exists("gnome-extensions") { return false; }
    Command::new("gnome-extensions").args(["list", "--enabled"]).output().ok()
        .filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|s| s.trim() == GNOME_VIDEO_UUID))
        .unwrap_or(false)
}
fn gnome_extension_schema_dir() -> Option<std::path::PathBuf> {
    std::iter::once(paths::data_home()).flatten().chain(paths::data_dirs())
        .map(|root| root.join("gnome-shell/extensions").join(GNOME_VIDEO_UUID).join("schemas"))
        .find(|dir| dir.join("gschemas.compiled").is_file())
}
fn gnome_gsettings_command() -> Option<Command> {
    if !environment::command_exists("gsettings") { return None; }
    let mut command = Command::new("gsettings");
    // Extensions commonly ship their schema privately instead of installing it
    // in the system-wide GSettings database.
    if let Some(dir) = gnome_extension_schema_dir() { command.arg("--schemadir").arg(dir); }
    Some(command)
}
fn gnome_schema_ready() -> bool {
    let Some(mut cmd) = gnome_gsettings_command() else { return false; };
    let Ok(result) = cmd.args(["list-keys", GNOME_VIDEO_SCHEMA]).output() else { return false; };
    result.status.success() && ["video-path", "enabled"].iter().all(|key|
        String::from_utf8_lossy(&result.stdout).lines().any(|line| line.trim() == *key))
}
fn gnome_setting(key: &str, value: &str) -> Result<(), String> {
    let mut cmd = gnome_gsettings_command().ok_or("gsettings is unavailable")?;
    let result = cmd.args(["set", GNOME_VIDEO_SCHEMA, key, value]).output()
        .map_err(|e| format!("GNOME: {e}"))?;
    if result.status.success() { Ok(()) }
    else { Err(format!("GNOME: could not apply {key}: {}", String::from_utf8_lossy(&result.stderr).trim())) }
}
#[derive(Debug, Clone)]
pub struct UniversalWallpaperBackend { pub backend: Backend, session: DesktopSession }
impl UniversalWallpaperBackend {
    pub fn detect() -> Self { Self::from_session(DesktopSession::detect()) }
    pub fn from_session(session: DesktopSession) -> Self {
        Self { backend: Backend::from_desktop(&session.desktop), session }
    }
    pub fn backend_id(&self) -> &'static str { self.backend.id() }
    fn video_route(&self) -> Result<VideoRoute, String> {
        match self.backend {
            Backend::Hyprland | Backend::Sway => Ok(VideoRoute::ExistingWayland),
            Backend::Plasma => Ok(VideoRoute::PlasmaPlugin),
            Backend::Gnome if self.session.session_type == SessionType::Wayland => Ok(VideoRoute::GnomeExtension),
            Backend::Gnome | Backend::Xfce | Backend::Cinnamon | Backend::Mate | Backend::Lxqt => match self.session.session_type {
                SessionType::X11 => Ok(VideoRoute::Generic(Mode::X11)),
                SessionType::Wayland => Ok(VideoRoute::Generic(Mode::WaylandLayer)),
                SessionType::Unknown => Err("Cannot choose a player without detecting X11 or Wayland.".into()),
            },
            Backend::Unsupported => Err("No animated wallpaper adapter is available for this desktop.".into()),
        }
    }
    /// Physical playback size for real outputs, never assume all screens are 1080p.
    /// Generic X11 desktops use the largest attached display because their
    /// wallpaper backend cannot address displays individually.
    pub fn video_display_size(&self, monitor: &str) -> Option<DisplaySize> {
        if self.backend.per_monitor() {
            let display = self.list_monitors().ok()?.into_iter().find(|d| d.name == monitor)?;
            return Some(DisplaySize { width: display.width, height: display.height, refresh: display.refresh_rate.round() as u32 });
        }
        if self.session.session_type == SessionType::X11 {
            let output = Command::new("xrandr").arg("--query").output().ok()?;
            if output.status.success() { return video_optimizer::largest_x11_display(&String::from_utf8_lossy(&output.stdout)); }
        }
        // Unknown Wayland global monitor sizes: preserve original rather than
        // guessing and degrading a multi-monitor or HiDPI desktop.
        None
    }
    /// The optimizer invokes this only with the global apply lock held. Avoid
    /// scheduling another conversion or altering persistent wallpaper choices.
    pub(super) fn activate_optimized(&self, cached: &Path, monitor: &str) -> Result<(), String> {
        self.apply_video_direct(cached, monitor)
    }
    /// Explicit user choice: play the original even when auto-optimization is on.
    pub fn apply_original_video(&self, path: &Path, monitor: &str) -> Result<(), String> {
        self.support(WallpaperKind::Video)?;
        if !path.is_file() { return Err("Video does not exist".into()); }
        self.validate_target(monitor)?;
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        let _lock = video_optimizer::apply_lock().lock().map_err(|_| "Wallpaper lock unavailable")?;
        video_optimizer::invalidate(self.backend_id(), monitor);
        self.apply_video_direct(&path, monitor)
    }

    /// Explicit user choice: only switch once the cached conversion is ready.
    pub fn apply_prepared_video(&self, path: &Path, monitor: &str) -> Result<(), String> {
        self.support(WallpaperKind::Video)?;
        self.validate_target(monitor)?;
        let size = self.video_display_size(monitor).ok_or("Cannot detect monitor resolution")?;
        let cache = video_optimizer::prepared_path(path, size).ok_or("Optimized copy is not ready")?;
        let _lock = video_optimizer::apply_lock().lock().map_err(|_| "Wallpaper lock unavailable")?;
        video_optimizer::invalidate(self.backend_id(), monitor);
        self.apply_video_direct(&cache, monitor)
    }
    fn apply_video_direct(&self, path: &Path, monitor: &str) -> Result<(), String> {
        match self.video_route()? {
            VideoRoute::ExistingWayland => HyprlandWallpaperBackend::new().apply(path, monitor, WallpaperKind::Video),
            VideoRoute::Generic(mode) => GenericVideoBackend::apply(path, mode),
            VideoRoute::GnomeExtension => apply_gnome_video(path),
            VideoRoute::PlasmaPlugin => apply_plasma_video(path),
        }
    }
    fn generic_player(&self) -> bool { matches!(self.video_route(), Ok(VideoRoute::Generic(_))) }
    pub fn playback_controls(&self) -> bool { (self.backend.controls_playback() || self.generic_player()) && self.support(WallpaperKind::Video).is_ok() }
    pub fn pause_all(&self) -> Result<(), String> {
        if self.generic_player() { GenericVideoBackend::pause_all() }
        else if self.backend.controls_playback() { HyprlandWallpaperBackend::pause_all() }
        else { Err("Native playback is managed by the desktop or its extension.".into()) }
    }
    pub fn resume_all(&self) -> Result<(), String> {
        if self.generic_player() { GenericVideoBackend::resume_all() }
        else if self.backend.controls_playback() { HyprlandWallpaperBackend::resume_all() }
        else { Err("Native playback is managed by the desktop or its extension.".into()) }
    }
    pub fn is_paused(&self) -> bool {
        if self.generic_player() { GenericVideoBackend::is_paused() }
        else if self.backend.controls_playback() { HyprlandWallpaperBackend::is_paused().unwrap_or(false) }
        else { false }
    }
    pub fn stop_all(&self) -> Result<(), String> {
        let _apply = video_optimizer::apply_lock().lock().map_err(|_| "Wallpaper apply lock unavailable")?;
        video_optimizer::invalidate_all();
        if self.generic_player() { GenericVideoBackend::stop_all() }
        else if self.backend.controls_playback() { HyprlandWallpaperBackend::stop_all() }
        else { Err("This wallpaper is managed by a native extension; stop it from its preferences.".into()) }
    }
    pub fn set_audio(&self, enabled: bool, desktop_only: bool) -> Result<(), String> {
        if self.generic_player() {
            if desktop_only { return Err("Desktop-only audio requires Hyprland.".into()); }
            GenericVideoBackend::set_audio(enabled)
        } else if self.backend.controls_playback() { HyprlandWallpaperBackend::set_audio_settings(enabled, desktop_only) }
        else { Err("Audio is configured in the desktop extension.".into()) }
    }
    /// On application exit, only stop processes spawned by this app. Native
    /// GNOME/Plasma plugins are managed by the compositor and remain active.
    pub fn on_app_exit(&self) -> Result<(), String> {
        let _apply = video_optimizer::apply_lock().lock().map_err(|_| "Wallpaper apply lock unavailable")?;
        video_optimizer::invalidate_all();
        if self.generic_player() { GenericVideoBackend::stop_all() }
        else if self.backend.controls_playback() { HyprlandWallpaperBackend::pause_all() }
        else { Ok(()) }
    }
    pub fn shutdown(&self) -> Result<(), String> {
        let _apply = video_optimizer::apply_lock().lock().map_err(|_| "Wallpaper apply lock unavailable")?;
        video_optimizer::invalidate_all();
        if self.generic_player() { GenericVideoBackend::stop_all() }
        else if self.backend.controls_playback() { HyprlandWallpaperBackend::shutdown() }
        else { Ok(()) }
    }
    pub fn support(&self, kind: WallpaperKind) -> Result<(), String> {
        if self.backend == Backend::Unsupported {
            return Err(format!("No wallpaper adapter is available for {}. Desktop settings were not changed.", self.session.desktop.label()));
        }
        if kind == WallpaperKind::Video {
            match self.video_route()? {
                VideoRoute::ExistingWayland => {
                    if !environment::command_exists("mpvpaper") {
                        return Err("Install mpvpaper to play animated wallpapers on Hyprland/Sway.".into());
                    }
                },
                VideoRoute::Generic(Mode::X11) => {
                    if !environment::command_exists("xwinwrap") || !environment::command_exists("mpv") {
                        return Err("For animated wallpapers on X11, install 'xwinwrap' and 'mpv'. xwinwrap may cover desktop icons; see the documentation.".into());
                    }
                },
                VideoRoute::Generic(Mode::WaylandLayer) => {
                    if !environment::command_exists("mpvpaper") {
                        return Err("For Wayland animations, install 'mpvpaper' (requires layer-shell support from your compositor).".into());
                    }
                },
                VideoRoute::GnomeExtension => {
                    if !gnome_extension_enabled() || !gnome_schema_ready() || !environment::command_exists("mpv") {
                        return Err(format!("GNOME Wayland requires the {GNOME_VIDEO_UUID} extension enabled, its GSettings schema, and 'mpv'. See docs/ANIMATED_LINUX.md."));
                    }
                },
                VideoRoute::PlasmaPlugin => {
                    if !installed_plasma_plugin() || !environment::command_exists("gdbus") {
                        return Err(format!("KDE Plasma requires the '{PLASMA_VIDEO_PLUGIN}' plugin and 'gdbus' for videos. See docs/ANIMATED_LINUX.md."));
                    }
                },
            }
        }
        match self.backend {
            Backend::Hyprland if !self.session.can_use_hyprland_backend() =>
                return Err("Start Persona 3 Wallpaper from an active Hyprland session with hyprctl and HYPRLAND_INSTANCE_SIGNATURE available.".into()),
            Backend::Sway if self.session.session_type != SessionType::Wayland || std::env::var_os("SWAYSOCK").is_none() || !environment::command_exists("swaymsg") =>
                return Err("Sway requires an active Wayland session, SWAYSOCK, and swaymsg.".into()),
            Backend::Plasma if !["qdbus6", "qdbus-qt6", "qdbus", "gdbus"].iter().any(|t| environment::command_exists(t)) =>
                return Err("Plasma requires qdbus6, qdbus-qt6, qdbus, or gdbus for D-Bus desktop control.".into()),
            _ => {}
        }
        if kind == WallpaperKind::Image {
            if let Some(tool) = self.backend.required_tool(kind) {
                if !environment::command_exists(tool) {
                    return Err(format!("Missing '{tool}' for {}. Install it from your distribution's repository.", self.backend.description()));
                }
            }
        }
        Ok(())
    }
    pub fn status(&self) -> String {
        let image = self.support(WallpaperKind::Image);
        let video = self.support(WallpaperKind::Video);
        match (image, video) {
            (Ok(()), Ok(())) => format!("Backend: {} · images and videos available.", self.backend.description()),
            (Ok(()), Err(reason)) => format!("Backend: {} · static images available. Videos: {reason}", self.backend.description()),
            (Err(e), Ok(())) => format!("Backend: {} · images: {e}. Animated videos available.", self.backend.description()),
            (Err(e), Err(v)) => format!("Backend: {} · images: {e}. Videos: {v}", self.backend.description()),
        }
    }
    pub fn list_monitors(&self) -> Result<Vec<Monitor>, String> {
        self.support(WallpaperKind::Image)?;
        match self.backend {
            Backend::Hyprland => HyprlandMonitorProvider::new().monitors(),
            Backend::Sway => SwayMonitorProvider.monitors(),
            _ => Ok(vec![Monitor {
                id: 0, name: ALL_DISPLAYS.into(), description: "All monitors (global desktop settings)".into(),
                width: 0, height: 0, refresh_rate: 0.0, x: 0, y: 0, focused: true,
            }]),
        }
    }
    fn validate_target(&self, monitor: &str) -> Result<(), String> {
        if self.backend.per_monitor() {
            if !self.list_monitors()?.iter().any(|m| m.name == monitor) {
                return Err(format!("Display '{monitor}' is not connected"));
            }
        } else if monitor != ALL_DISPLAYS {
            return Err("This desktop only exposes the 'all' target (all monitors).".into());
        }
        Ok(())
    }
}
impl WallpaperApplier for UniversalWallpaperBackend {
    fn apply(&self, path: &Path, monitor: &str, kind: WallpaperKind) -> Result<(), String> {
        self.support(kind)?;
        if !path.is_file() { return Err(format!("File does not exist: {}", path.display())); }
        let path = path.canonicalize().map_err(|e| format!("Could not resolve path: {e}"))?;
        self.validate_target(monitor)?;
        // Also serializes the background cache upgrade with normal user actions:
        // an old conversion must never replace a more recent wallpaper choice.
        let _apply = video_optimizer::apply_lock().lock().map_err(|_| "Wallpaper apply lock unavailable")?;
        if kind == WallpaperKind::Video {
            let route = self.video_route()?;
            let selectable = matches!(route, VideoRoute::ExistingWayland | VideoRoute::Generic(_));
            let optimized = if selectable {
                video_optimizer::select(&path, self.backend_id(), monitor, self.video_display_size(monitor))
            } else {
                video_optimizer::invalidate(self.backend_id(), monitor);
                path.clone()
            };
            let result = self.apply_video_direct(&optimized, monitor);
            if result.is_err() && optimized != path {
                // An externally damaged cached file must not prevent wallpaper
                // playback. It will be regenerated on a subsequent selection.
                let _ = std::fs::remove_file(&optimized);
                return self.apply_video_direct(&path, monitor);
            }
            if result.is_err() { video_optimizer::invalidate(self.backend_id(), monitor); }
            return result;
        }
        video_optimizer::invalidate(self.backend_id(), monitor);
        // If a generic process is covering the desktop, release it before
        // delegating to the native static-image configuration.
        if self.generic_player() { GenericVideoBackend::stop_all()?; }
        match self.backend {
            Backend::Hyprland | Backend::Sway => HyprlandWallpaperBackend::new().apply(&path, monitor, kind),
            Backend::Gnome => {
                // GNOME's compositor extension otherwise keeps covering the static background.
                if self.session.session_type == SessionType::Wayland && gnome_schema_ready() {
                    gnome_setting("enabled", "false")?;
                }
                apply_gsettings_uri("org.gnome.desktop.background", &path, true)
            },
            Backend::Cinnamon => apply_gsettings_uri("org.cinnamon.desktop.background", &path, false),
            Backend::Mate => run_checked("gsettings", &["set", "org.mate.background", "picture-filename", path.to_str().ok_or("Path is not valid UTF-8")?]),
            Backend::Plasma => apply_plasma(&path),
            Backend::Xfce => apply_xfce(&path),
            Backend::Lxqt => run_checked("pcmanfm-qt", &["--set-wallpaper", path.to_str().ok_or("Path is not valid UTF-8")?]),
            Backend::Unsupported => Err("No compatible backend is available".into()),
        }
    }
}

fn apply_gnome_video(path: &Path) -> Result<(), String> {
    let path = path.to_str().ok_or("GNOME: video path is not valid UTF-8")?;
    // Extension's documented GSettings API, not GNOME picture-uri (which is static).
    gnome_setting("video-path", path)?;
    gnome_setting("enabled", "true")
}

fn apply_plasma_video(path: &Path) -> Result<(), String> {
    let urls = serde_json::json!([{"filename": file_uri(path), "enabled": true, "duration": 0,
        "customDuration": 0, "playbackRate": 0, "loop": false}]).to_string();
    // The plugin stores a JSON STRING under VideoUrls, not a JSON array value.
    // Plasma's evaluateScript updates every desktop containment, matching the
    // app's "all" monitor target. Encode JS strings using serde_json.
    let url_arg = serde_json::to_string(&urls).map_err(|e| e.to_string())?;
    let plugin_arg = serde_json::to_string(PLASMA_VIDEO_PLUGIN).map_err(|e| e.to_string())?;
    let script = format!("var ds = desktops(); for (var i=0; i<ds.length; ++i) {{ var d=ds[i]; d.wallpaperPlugin={plugin_arg}; d.currentConfigGroup=['Wallpaper', {plugin_arg}, 'General']; d.writeConfig('VideoUrls', {url_arg}); }}");
    // gdbus also works without KDE-specific qdbus executables.
    run_checked("gdbus", &["call", "--session", "--dest", "org.kde.plasmashell",
        "--object-path", "/PlasmaShell", "--method", "org.kde.PlasmaShell.evaluateScript", &script])
}

fn run_checked(binary: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(binary).args(args).output().map_err(|e| format!("Could not run {binary}: {e}"))?;
    if output.status.success() { Ok(()) } else {
        let message = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!("{binary} returned an error: {}", if message.trim().is_empty() { stdout.trim() } else { message.trim() }))
    }
}

/// Percent-escape file paths for GSettings/Plasma file:// URIs; never run a shell.
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~') { uri.push(b as char); }
        else { uri.push_str(&format!("%{b:02X}")); }
    }
    uri
}
fn has_gsettings_key(schema: &str, key: &str) -> bool {
    let Ok(output) = Command::new("gsettings").args(["list-keys", schema]).output() else { return false; };
    output.status.success() && String::from_utf8_lossy(&output.stdout).lines().any(|line| line.trim() == key)
}
fn apply_gsettings_uri(schema: &str, path: &Path, dark_variant: bool) -> Result<(), String> {
    let uri = file_uri(path);
    run_checked("gsettings", &["set", schema, "picture-uri", &uri])?;
    if let Ok(output) = Command::new("gsettings").args(["get", schema, "picture-options"]).output() {
        if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "'none'" {
            run_checked("gsettings", &["set", schema, "picture-options", "zoom"])?;
        }
    }
    if dark_variant && has_gsettings_key(schema, "picture-uri-dark") {
        run_checked("gsettings", &["set", schema, "picture-uri-dark", &uri])?;
    }
    Ok(())
}
fn apply_xfce(path: &Path) -> Result<(), String> {
    let output = Command::new("xfconf-query").args(["-c", "xfce4-desktop", "-l"]).output()
        .map_err(|e| format!("Could not query xfce4-desktop: {e}"))?;
    if !output.status.success() { return Err("XFCE could not list xfce4-desktop properties".into()); }
    let props: Vec<String> = String::from_utf8_lossy(&output.stdout).lines()
        .filter(|name| name.starts_with("/backdrop/") && (name.ends_with("/last-image") || name.ends_with("/image-path")))
        .map(str::to_owned).collect();
    if props.is_empty() {
        return Err("XFCE reported no wallpaper properties; open Desktop Settings once and try again.".into());
    }
    let value = path.to_str().ok_or("Path is not valid UTF-8")?;
    let mut failures = Vec::new();
    for prop in &props {
        if let Err(e) = run_checked("xfconf-query", &["-c", "xfce4-desktop", "-p", prop, "-s", value]) { failures.push(e); }
    }
    if failures.is_empty() { Ok(()) } else { Err(format!("XFCE: {} properties could not be updated: {}", failures.len(), failures.join("; "))) }
}
fn apply_plasma(path: &Path) -> Result<(), String> {
    let uri = serde_json::to_string(&file_uri(path)).map_err(|e| e.to_string())?;
    let js = format!("var items = desktops(); for (var i = 0; i < items.length; ++i) {{ var d = items[i]; d.wallpaperPlugin = 'org.kde.image'; d.currentConfigGroup = ['Wallpaper', 'org.kde.image', 'General']; d.writeConfig('Image', {uri}); }}");
    let mut errors = Vec::new();
    for cli in ["qdbus6", "qdbus-qt6", "qdbus", "gdbus"] {
        if !environment::command_exists(cli) { continue; }
        let result = if cli == "gdbus" {
            run_checked(cli, &["call", "--session", "--dest", "org.kde.plasmashell", "--object-path", "/PlasmaShell", "--method", "org.kde.PlasmaShell.evaluateScript", &js])
        } else {
            run_checked(cli, &["org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell.evaluateScript", &js])
        };
        match result { Ok(()) => return Ok(()), Err(e) => errors.push(e) }
    }
    Err(format!("PlasmaShell D-Bus could not apply wallpaper: {}", errors.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picks_adapters_without_distribution() {
        assert_eq!(Backend::from_desktop(&Desktop::Gnome), Backend::Gnome);
        assert_eq!(Backend::from_desktop(&Desktop::Plasma), Backend::Plasma);
        assert_eq!(Backend::from_desktop(&Desktop::Sway), Backend::Sway);
        assert_eq!(Backend::from_desktop(&Desktop::Other("river".into())), Backend::Unsupported);
        assert!(Backend::Xfce.supports_video());
        assert!(Backend::Hyprland.supports_video());
    }
    #[test]
    fn animated_routes_are_selected_by_session_not_distro() {
        let session = |desktop, session_type| DesktopSession { desktop, session_type, noctalia: false, hyprland_active: false };
        assert_eq!(UniversalWallpaperBackend::from_session(session(Desktop::Xfce, SessionType::X11)).video_route().unwrap(), VideoRoute::Generic(Mode::X11));
        assert_eq!(UniversalWallpaperBackend::from_session(session(Desktop::Cinnamon, SessionType::Wayland)).video_route().unwrap(), VideoRoute::Generic(Mode::WaylandLayer));
        assert_eq!(UniversalWallpaperBackend::from_session(session(Desktop::Gnome, SessionType::Wayland)).video_route().unwrap(), VideoRoute::GnomeExtension);
        assert_eq!(UniversalWallpaperBackend::from_session(session(Desktop::Gnome, SessionType::X11)).video_route().unwrap(), VideoRoute::Generic(Mode::X11));
        assert_eq!(UniversalWallpaperBackend::from_session(session(Desktop::Plasma, SessionType::Wayland)).video_route().unwrap(), VideoRoute::PlasmaPlugin);
    }
    #[test]
    fn file_uri_is_escaped_for_spaces_and_odd_characters() {
        assert_eq!(file_uri(Path::new("/tmp/a b#'ñ.png")), "file:///tmp/a%20b%23%27%C3%B1.png");
    }
}
