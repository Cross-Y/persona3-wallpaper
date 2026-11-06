//! Qt/QML bridge. Desktop integrations and worker logic live in focused modules.
use crate::{cursor_manager, font_manager, settings_store, theme_manager};
use crate::autostart::set_autostart;
use crate::desktop_options::{available_cursor_themes, available_fonts};
use crate::focus_monitor::{start_focus_worker, update_focus_state, update_occupancy_state};
use crate::resource_monitor::current_resource_usage;
use crate::wallpaper_restore::restore_saved_wallpapers;
use cxx_qt_lib::QString;
use p3w_platform::linux::{environment::{self, DesktopSession}, universal_wallpaper::UniversalWallpaperBackend};
use p3w_platform::WallpaperKind;
use std::pin::Pin;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = super::QString;
    }
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type SettingsController = super::SettingsControllerRust;
        #[qinvokable]
        fn initialize(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn dynamic_theme_status(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn set_dynamic_colors(self: Pin<&mut SettingsController>, enabled: bool) -> QString;
        #[qinvokable]
        fn set_dynamic_theme_target(self: Pin<&mut SettingsController>, target: QString, enabled: bool) -> QString;
        #[qinvokable]
        fn refresh_dynamic_colors(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn set_palette_color(self: Pin<&mut SettingsController>, role: QString, hex: QString) -> QString;
        #[qinvokable]
        fn resource_usage(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn wallpaper_support(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn wallpaper_capabilities(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn linux_diagnostics(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn set_paused(self: Pin<&mut SettingsController>, paused: bool) -> QString;
        #[qinvokable]
        fn set_restore_on_start(self: Pin<&mut SettingsController>, enabled: bool) -> QString;
        #[qinvokable]
        fn set_focus_pause(self: Pin<&mut SettingsController>, enabled: bool) -> QString;
        #[qinvokable]
        fn set_start_on_login(self: Pin<&mut SettingsController>, enabled: bool) -> QString;
        #[qinvokable]
        fn set_background_mode(self: Pin<&mut SettingsController>, enabled: bool) -> QString;
        #[qinvokable]
        fn set_ui_sounds(self: Pin<&mut SettingsController>, enabled: bool) -> QString;
        #[qinvokable]
        fn set_ui_sound_volume(self: Pin<&mut SettingsController>, volume: f64) -> QString;
        #[qinvokable]
        fn set_auto_video_optimize(self: Pin<&mut SettingsController>, enabled: bool) -> QString;
        #[qinvokable]
        fn video_optimization_status(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn list_fonts(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn font_environment(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn list_cursor_themes(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn set_font_family(self: Pin<&mut SettingsController>, family: QString) -> QString;
        #[qinvokable]
        fn set_cursor_theme(self: Pin<&mut SettingsController>, theme: QString) -> QString;
        #[qinvokable]
        fn import_cursor_archive(self: Pin<&mut SettingsController>, archive_url: QString) -> QString;
        #[qinvokable]
        fn set_audio(
            self: Pin<&mut SettingsController>,
            enabled: bool,
            desktop_only: bool,
        ) -> QString;
        #[qinvokable]
        fn stop_wallpapers(self: Pin<&mut SettingsController>) -> QString;
        #[qinvokable]
        fn restore_wallpapers(self: Pin<&mut SettingsController>) -> QString;
    }
}
#[derive(Default)]
pub struct SettingsControllerRust;

impl qobject::SettingsController {
    pub fn dynamic_theme_status(self: Pin<&mut Self>) -> QString {
        QString::from(theme_manager::status().to_string())
    }
    pub fn set_dynamic_colors(self: Pin<&mut Self>, enabled: bool) -> QString {
        match theme_manager::set_auto(enabled) { Ok(info) => QString::from(format!("OK:{info}")), Err(e) => QString::from(format!("ERROR:{e}")) }
    }
    pub fn set_dynamic_theme_target(self: Pin<&mut Self>, target: QString, enabled: bool) -> QString {
        match theme_manager::set_target(&target.to_string(), enabled) { Ok(info) => QString::from(format!("OK:{info}")), Err(e) => QString::from(format!("ERROR:{e}")) }
    }
    pub fn refresh_dynamic_colors(self: Pin<&mut Self>) -> QString {
        match theme_manager::refresh_saved() { Ok(info) => QString::from(format!("OK:{info}")), Err(e) => QString::from(format!("ERROR:{e}")) }
    }
    pub fn set_palette_color(self: Pin<&mut Self>, role: QString, hex: QString) -> QString {
        match theme_manager::set_palette_color(&role.to_string(), &hex.to_string()) {
            Ok(info)=>QString::from(format!("OK:{info}")),
            Err(error)=>QString::from(format!("ERROR:{error}"))
        }
    }
    pub fn wallpaper_support(self: Pin<&mut Self>) -> QString {
        QString::from(UniversalWallpaperBackend::detect().status())
    }
    pub fn wallpaper_capabilities(self: Pin<&mut Self>) -> QString {
        let backend = UniversalWallpaperBackend::detect();
        QString::from(serde_json::json!({
            "backend": backend.backend_id(),
            "images": backend.support(WallpaperKind::Image).is_ok(),
            "videos": backend.support(WallpaperKind::Video).is_ok(),
            "playback_controls": backend.playback_controls(),
            "per_monitor": backend.backend.per_monitor()
        }).to_string())
    }
    pub fn linux_diagnostics(self: Pin<&mut Self>) -> QString {
        QString::from(environment::diagnostics())
    }
    pub fn resource_usage(self: Pin<&mut Self>) -> QString {
        QString::from(current_resource_usage())
    }
    pub fn list_fonts(self: Pin<&mut Self>) -> QString {
        QString::from(serde_json::to_string(&available_fonts()).unwrap_or_else(|_| "[]".into()))
    }
    pub fn font_environment(self: Pin<&mut Self>) -> QString {
        QString::from(font_manager::describe_environment())
    }
    pub fn list_cursor_themes(self: Pin<&mut Self>) -> QString {
        QString::from(serde_json::to_string(&available_cursor_themes()).unwrap_or_else(|_| "[]".into()))
    }
    pub fn set_font_family(self: Pin<&mut Self>, family: QString) -> QString {
        let family = family.to_string();
        if !available_fonts().iter().any(|f| f == &family) { return QString::from("ERROR:Font is not installed."); }
        let applied = match font_manager::apply_font(&family) {
            Ok(backends) => backends,
            Err(e) => return QString::from(format!("ERROR:{e}")),
        };
        match settings_store::set_string("font_family", &family) {
            Ok(()) => QString::from(format!("OK:{}", applied.join(", "))),
            Err(e) => QString::from(format!("ERROR:{e}")),
        }
    }
    pub fn import_cursor_archive(self: Pin<&mut Self>, archive_url: QString) -> QString {
        match cursor_manager::import_cursor_archive(&archive_url.to_string()) {
            Ok(themes) => QString::from(format!("OK:{}", themes.join("|"))),
            Err(error) => QString::from(format!("ERROR:{error}")),
        }
    }
    pub fn set_cursor_theme(self: Pin<&mut Self>, theme: QString) -> QString {
        let theme = theme.to_string();
        if !available_cursor_themes().iter().any(|t| t == &theme) { return QString::from("ERROR:Cursor theme is not installed."); }
        let applied = match cursor_manager::apply_cursor_theme(&theme, 24) {
            Ok(backends) => backends,
            Err(e) => return QString::from(format!("ERROR:{e}")),
        };
        match settings_store::set_string("cursor_theme", &theme) {
            Ok(()) => QString::from(format!("OK:{}", applied.join(", "))),
            Err(e) => QString::from(format!("ERROR:{e}")),
        }
    }
    pub fn initialize(self: Pin<&mut Self>) -> QString {
        let cfg = settings_store::load();
        // Refresh autostart after moving from `target/debug` to an installed
        // binary, or when the user updates an AppImage in place.
        if cfg["start_on_login"].as_bool().unwrap_or(false) {
            if let Err(error) = set_autostart(true) {
                eprintln!("Could not refresh login autostart: {error}");
            }
        }
        p3w_platform::linux::video_optimizer::set_enabled(cfg["auto_video_optimize"].as_bool().unwrap_or(true));
        let sound = cfg["sound_enabled"].as_bool().unwrap_or(false);
        let desktop_only = cfg["audio_desktop_only"].as_bool().unwrap_or(false);
        let universal = UniversalWallpaperBackend::detect();
        if universal.playback_controls() {
            let _ = universal.set_audio(sound, desktop_only);
            // Focus/occupancy monitoring currently uses Hyprland's event socket only.
            if DesktopSession::detect().can_use_hyprland_backend() { start_focus_worker(); }
            if cfg["paused"].as_bool().unwrap_or(false) {
                let _ = universal.pause_all();
                let _ = restore_saved_wallpapers();
            } else if cfg["restore_on_start"].as_bool().unwrap_or(true) {
                let _ = restore_saved_wallpapers();
            }
            if DesktopSession::detect().can_use_hyprland_backend() { update_focus_state(); }
        } else if cfg["restore_on_start"].as_bool().unwrap_or(true) {
            let _ = restore_saved_wallpapers();
        }
        // Desktop cursor settings persist on their own; only re-apply an explicitly
        // selected cursor on Hyprland where hyprctl's runtime change is ephemeral.
        if DesktopSession::detect().can_use_hyprland_backend() && settings_store::has_saved_key("cursor_theme") {
            if let Some(saved) = cfg["cursor_theme"].as_str() {
                let _ = cursor_manager::apply_cursor_theme(saved, 24);
            }
        }
        QString::from(cfg.to_string())
    }
    pub fn video_optimization_status(self: Pin<&mut Self>) -> QString {
        QString::from(p3w_platform::linux::video_optimizer::status_text())
    }
    pub fn set_auto_video_optimize(self: Pin<&mut Self>, enabled: bool) -> QString {
        match settings_store::set_bool("auto_video_optimize", enabled) {
            Ok(()) => {
                p3w_platform::linux::video_optimizer::set_enabled(enabled);
                // Reapply existing selections: disabling returns to the original
                // media; enabling picks cached versions or queues conversions.
                if let Err(error) = restore_saved_wallpapers() { eprintln!("Could not refresh wallpaper optimization: {error}"); }
                QString::from("OK")
            }
            Err(error) => QString::from(format!("ERROR:{error}"))
        }
    }
    pub fn set_paused(self: Pin<&mut Self>, paused: bool) -> QString {
        let universal = UniversalWallpaperBackend::detect();
        if !universal.playback_controls() { return QString::from("ERROR:This desktop manages animated wallpapers through a native extension."); }
        let old = settings_store::load()["paused"].as_bool().unwrap_or(false);
        if let Err(e) = settings_store::set_bool("paused", paused) {
            return QString::from(format!("ERROR: {e}"));
        }
        let result = if paused {
            universal.pause_all()
        } else {
            if let Err(e) = restore_saved_wallpapers() {
                let _ = settings_store::set_bool("paused", old);
                return QString::from(format!("ERROR:{e}"));
            }
            universal.resume_all()
        };
        match result {
            Ok(()) => QString::from("OK"),
            Err(e) => {
                let _ = settings_store::set_bool("paused", old);
                QString::from(format!("ERROR: {e}"))
            }
        }
    }
    pub fn set_restore_on_start(self: Pin<&mut Self>, enabled: bool) -> QString {
        match settings_store::set_bool("restore_on_start", enabled) {
            Ok(()) => QString::from("OK"),
            Err(e) => QString::from(format!("ERROR: {e}")),
        }
    }
    pub fn set_focus_pause(self: Pin<&mut Self>, enabled: bool) -> QString {
        if !DesktopSession::detect().can_use_hyprland_backend() {
            return QString::from("ERROR:Pausing when windows are visible currently requires Hyprland.");
        }
        if let Err(e) = settings_store::set_bool("focus_pause", enabled) {
            return QString::from(format!("ERROR: {e}"));
        }
        if DesktopSession::detect().can_use_hyprland_backend() { update_occupancy_state(); }
        QString::from("OK")
    }
    pub fn set_start_on_login(self: Pin<&mut Self>, enabled: bool) -> QString {
        let previous=settings_store::load()["start_on_login"].as_bool().unwrap_or(false);
        match set_autostart(enabled) {
            Err(e)=>QString::from(format!("ERROR: {e}")),
            Ok(())=>match settings_store::set_bool("start_on_login",enabled) {
                Ok(())=>QString::from("OK"),
                Err(e)=>{
                    let _=set_autostart(previous);
                    QString::from(format!("ERROR: {e}"))
                }
            }
        }
    }

    pub fn set_background_mode(self: Pin<&mut Self>, enabled: bool) -> QString {
        match settings_store::set_bool("background_mode", enabled) {
            Ok(()) => QString::from("OK"),
            Err(e) => QString::from(format!("ERROR: {e}")),
        }
    }
    pub fn set_ui_sounds(self: Pin<&mut Self>, enabled: bool) -> QString {
        match settings_store::set_bool("ui_sounds_enabled", enabled) {
            Ok(()) => QString::from("OK"),
            Err(e) => QString::from(format!("ERROR: {e}")),
        }
    }
    pub fn set_ui_sound_volume(self: Pin<&mut Self>, volume: f64) -> QString {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return QString::from("ERROR: Invalid interface sound volume.");
        }
        let mut cfg = settings_store::load();
        cfg["ui_sound_volume"] = serde_json::json!(volume);
        match settings_store::save(&cfg) {
            Ok(()) => QString::from("OK"),
            Err(e) => QString::from(format!("ERROR: {e}")),
        }
    }
    pub fn set_audio(self: Pin<&mut Self>, enabled: bool, desktop_only: bool) -> QString {
        if !UniversalWallpaperBackend::detect().playback_controls() { return QString::from("ERROR:This desktop has no animated wallpaper player controlled by the application."); }
        if desktop_only && !DesktopSession::detect().can_use_hyprland_backend() {
            return QString::from("ERROR:Desktop-only audio requires Hyprland focus events.");
        }
        let previous=settings_store::load();
        let old_enabled=previous["sound_enabled"].as_bool().unwrap_or(false);
        let old_desktop_only=previous["audio_desktop_only"].as_bool().unwrap_or(false);
        if let Err(e) = settings_store::set_audio(enabled, desktop_only) {
            return QString::from(format!("ERROR: {e}"));
        }
        match UniversalWallpaperBackend::detect().set_audio(enabled, desktop_only) {
            Ok(()) => QString::from("OK"),
            Err(e) => {
                let _=settings_store::set_audio(old_enabled,old_desktop_only);
                QString::from(format!("ERROR: {e}"))
            }
        }
    }

    pub fn stop_wallpapers(self: Pin<&mut Self>) -> QString {
        if !UniversalWallpaperBackend::detect().playback_controls() { return QString::from("ERROR:This desktop has no animated wallpaper player controlled by the application."); }
        match UniversalWallpaperBackend::detect().stop_all() {
            Ok(()) => QString::from("OK"),
            Err(e) => QString::from(format!("ERROR: {e}")),
        }
    }

    pub fn restore_wallpapers(self: Pin<&mut Self>) -> QString {
        match restore_saved_wallpapers() {
            Ok(()) => QString::from("OK"),
            Err(e) => QString::from(format!("ERROR:{e}")),
        }
    }
}
