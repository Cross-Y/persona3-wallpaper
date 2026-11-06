//! Installed fonts and Xcursor themes for the appearance selector.
use std::{collections::BTreeSet, process::Command, sync::{Mutex, OnceLock}, time::{Duration, Instant}};
use p3w_platform::linux::paths;

// Fontconfig is relatively expensive; avoid invoking fc-list again when the
// user opens the dropdown and immediately saves a selection.
static FONT_CACHE: OnceLock<Mutex<Option<(Instant, Vec<String>)>>> = OnceLock::new();
pub(crate) fn available_fonts() -> Vec<String> {
    let cache=FONT_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard)=cache.lock() {
        if let Some((timestamp, list))=guard.as_ref() {
            if timestamp.elapsed() < Duration::from_secs(30) { return list.clone(); }
        }
    }
    let output = Command::new("fc-list").args(["--format", "%{family}\n"]).output();
    let Ok(output) = output else { return Vec::new(); };
    if !output.status.success() { return Vec::new(); }
    let mut fonts = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        for family in line.split(',') {
            let family = family.trim();
            if !family.is_empty() { fonts.insert(family.to_string()); }
        }
    }
    let result: Vec<String> = fonts.into_iter().collect();
    if let Ok(mut guard)=cache.lock() { *guard=Some((Instant::now(), result.clone())); }
    result
}

pub(crate) fn available_cursor_themes() -> Vec<String> {
    let mut themes = BTreeSet::new();
    for root in paths::icon_theme_dirs() {
        let Ok(entries) = std::fs::read_dir(root) else { continue; };
        for entry in entries.flatten() {
            let path = entry.path();
            // An icon theme without cursors is not a selectable cursor theme.
            if path.join("cursors").is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    themes.insert(name.to_string());
                }
            }
        }
    }
    if themes.is_empty() { themes.insert("Adwaita".into()); }
    themes.into_iter().collect()
}

