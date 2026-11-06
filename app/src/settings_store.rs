use serde_json::{json, Map, Value};
use std::{fs::{self, OpenOptions}, io::Write, os::unix::fs::OpenOptionsExt,
    path::PathBuf, sync::{Mutex, atomic::{AtomicU64, Ordering}}};

// Serialize in-process updates; write a complete file then atomically rename it.
static SETTINGS_UPDATE: Mutex<()> = Mutex::new(());
static TEMP_SERIAL: AtomicU64 = AtomicU64::new(0);

use p3w_platform::linux::paths;

fn settings_path() -> Option<PathBuf> {
    paths::config_home().map(|base| base.join("persona3-wallpaper/settings.json"))
}
pub fn has_saved_key(key: &str) -> bool {
    settings_path().and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|data| serde_json::from_str::<Value>(&data).ok())
        .and_then(|value| value.as_object().map(|object| object.contains_key(key)))
        .unwrap_or(false)
}

pub fn load() -> Value {
    let mut v = json!({"paused":false,"restore_on_start":true,"focus_pause":true,"start_on_login":false,"background_mode":true,"sound_enabled":false,"audio_desktop_only":false,"ui_sounds_enabled":true,"ui_sound_volume":0.55,"auto_video_optimize":true,"font_family":"Inter","cursor_theme":"Adwaita","theme_auto_enabled":false,"theme_gtk_enabled":true,"theme_noctalia_enabled":true,"theme_desktop_enabled":true,"theme_qt_enabled":false,"theme_waybar_enabled":false,"theme_rofi_enabled":false,"theme_wofi_enabled":false,"theme_last_accent":"#559DD7","theme_manual_colors":{},"wallpapers":{}});
    let Some(path) = settings_path() else {
        return v;
    };
    if let Ok(contents) = std::fs::read_to_string(path) {
        if let Ok(saved) = serde_json::from_str::<Value>(&contents) {
            if let (Some(dst), Some(src)) = (v.as_object_mut(), saved.as_object()) {
                for (k, val) in src {
                    dst.insert(k.clone(), val.clone());
                }
            }
        }
    }
    if !v["wallpapers"].is_object() {
        v["wallpapers"] = Value::Object(Map::new());
    }
    v
}
fn save_unlocked(v: &Value) -> Result<(), String> {
    let path = settings_path().ok_or("Could not locate configuration directory")?;
    let parent = path.parent().ok_or("Invalid settings path")?;
    fs::create_dir_all(parent).map_err(|e| format!("Could not create config directory: {e}"))?;
    let data = serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?;
    let serial = TEMP_SERIAL.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(".settings-{}-{serial}.tmp", std::process::id()));
    let write_result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600)
            .open(&temp).map_err(|e| format!("Could not create settings temp file: {e}"))?;
        file.write_all(&data).map_err(|e| format!("Could not write settings: {e}"))?;
        file.sync_all().map_err(|e| format!("Could not sync settings: {e}"))?;
        fs::rename(&temp, &path).map_err(|e| format!("Could not replace settings: {e}"))
    })();
    if write_result.is_err() { let _ = fs::remove_file(&temp); }
    write_result
}

pub fn save(v: &Value) -> Result<(), String> {
    let _lock = SETTINGS_UPDATE.lock().map_err(|_| "Settings lock unavailable")?;
    save_unlocked(v)
}

// Read-modify-write is one critical section, so concurrent menu actions cannot
// accidentally overwrite each other's preferences using an old snapshot.
fn update(change: impl FnOnce(&mut Value)) -> Result<(), String> {
    let _lock = SETTINGS_UPDATE.lock().map_err(|_| "Settings lock unavailable")?;
    let mut value = load();
    change(&mut value);
    save_unlocked(&value)
}

pub fn set_bool(key: &str, value: bool) -> Result<(), String> {
    update(|v| v[key] = Value::Bool(value))
}
pub fn set_string(key: &str, value: &str) -> Result<(), String> {
    update(|v| v[key] = json!(value))
}
pub fn set_audio(enabled: bool, desktop_only: bool) -> Result<(), String> {
    update(|v| {
        v["sound_enabled"] = json!(enabled);
        v["audio_desktop_only"] = json!(desktop_only);
    })
}
pub fn remember_wallpaper_for(backend: &str, path: &str, monitor: &str, is_video: bool) -> Result<(), String> {
    update(|v| {
        if !v["wallpapers_v2"].is_object() { v["wallpapers_v2"] = Value::Object(Map::new()); }
        if !v["wallpapers_v2"][backend].is_object() { v["wallpapers_v2"][backend] = Value::Object(Map::new()); }
        v["wallpapers_v2"][backend][monitor] = json!({"path":path, "is_video":is_video});
        v["theme_last_wallpaper"] = json!(path);
        v["theme_last_video"] = json!(is_video);
    })
}
pub fn remember_video_playback_choice(backend: &str, path: &str, monitor: &str, choice: &str) -> Result<(), String> {
    if !["original", "optimized"].contains(&choice) { return Err("Invalid video playback choice".into()); }
    update(|v| {
        if !v["wallpapers_v2"].is_object() { v["wallpapers_v2"] = Value::Object(Map::new()); }
        if !v["wallpapers_v2"][backend].is_object() { v["wallpapers_v2"][backend] = Value::Object(Map::new()); }
        v["wallpapers_v2"][backend][monitor] = json!({"path":path, "is_video":true, "video_choice":choice});
        v["theme_last_wallpaper"] = json!(path);
        v["theme_last_video"] = json!(true);
    })
}

pub fn remember_wallpaper_with_layout(backend: &str, path: &str, monitor: &str,
    layout: &crate::image_optimizer::ImageLayout) -> Result<(), String> {
    update(|v| {
        if !v["wallpapers_v2"].is_object() { v["wallpapers_v2"] = Value::Object(Map::new()); }
        if !v["wallpapers_v2"][backend].is_object() { v["wallpapers_v2"][backend] = Value::Object(Map::new()); }
        v["wallpapers_v2"][backend][monitor] = json!({"path":path, "is_video":false, "image_layout":layout.to_value()});
        v["theme_last_wallpaper"] = json!(path);
        v["theme_last_video"] = json!(false);
    })
}

pub struct SavedWallpaper {
    pub monitor: String, pub path: PathBuf, pub is_video: bool,
    pub layout: Option<crate::image_optimizer::ImageLayout>,
    pub video_choice: Option<String>,
}
pub fn saved_wallpaper_entries_for(backend: &str) -> Vec<SavedWallpaper> {
    let v = load();
    let mut items = Map::new();
    if backend == "hyprland" {
        if let Some(legacy) = v["wallpapers"].as_object() {
            for (name, wallpaper) in legacy { items.insert(name.clone(), wallpaper.clone()); }
        }
    }
    if let Some(scoped) = v["wallpapers_v2"][backend].as_object() {
        for (name, wallpaper) in scoped { items.insert(name.clone(), wallpaper.clone()); }
    }
    items.into_iter().filter_map(|(monitor, w)| Some(SavedWallpaper {
        monitor, path: PathBuf::from(w["path"].as_str()?),
        is_video: w["is_video"].as_bool().unwrap_or(false),
        layout: crate::image_optimizer::ImageLayout::from_value(&w["image_layout"]),
        video_choice: w["video_choice"].as_str().map(str::to_owned),
    })).collect()
}

pub fn saved_wallpapers_for(backend: &str) -> Vec<(String, PathBuf, bool)> {
    let v = load();
    let mut items = Map::new();
    // Keep existing selections from the previous Hyprland-only format, but let
    // explicit v2 selections override those for the same monitor.
    if backend == "hyprland" {
        if let Some(legacy) = v["wallpapers"].as_object() {
            for (name, wallpaper) in legacy { items.insert(name.clone(), wallpaper.clone()); }
        }
    }
    if let Some(scoped) = v["wallpapers_v2"][backend].as_object() {
        for (name, wallpaper) in scoped { items.insert(name.clone(), wallpaper.clone()); }
    }
    items.into_iter().filter_map(|(monitor, w)| Some((monitor,
        PathBuf::from(w["path"].as_str()?), w["is_video"].as_bool().unwrap_or(false)))).collect()
}
