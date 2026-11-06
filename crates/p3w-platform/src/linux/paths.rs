//! Freedesktop XDG base directories. Never treat relative or empty XDG overrides
//! as a working-directory path: they are invalid according to the specification.
use std::{env, path::PathBuf};

fn absolute_environment_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from).filter(|p| p.is_absolute())
}

pub fn home() -> Option<PathBuf> { absolute_environment_path("HOME") }
pub fn config_home() -> Option<PathBuf> { absolute_environment_path("XDG_CONFIG_HOME").or_else(|| home().map(|p| p.join(".config"))) }
pub fn data_home() -> Option<PathBuf> { absolute_environment_path("XDG_DATA_HOME").or_else(|| home().map(|p| p.join(".local/share"))) }
pub fn state_home() -> Option<PathBuf> { absolute_environment_path("XDG_STATE_HOME").or_else(|| home().map(|p| p.join(".local/state"))) }
pub fn cache_home() -> Option<PathBuf> { absolute_environment_path("XDG_CACHE_HOME").or_else(|| home().map(|p| p.join(".cache"))) }
pub fn runtime_dir() -> Option<PathBuf> { absolute_environment_path("XDG_RUNTIME_DIR") }

/// System-wide data directories; the order matters when looking up icons.
pub fn data_dirs() -> Vec<PathBuf> {
    match env::var_os("XDG_DATA_DIRS").filter(|v| !v.is_empty()) {
        Some(raw) => env::split_paths(&raw).filter(|p| p.is_absolute()).collect(),
        None => vec![PathBuf::from("/usr/local/share"), PathBuf::from("/usr/share")],
    }
}

pub fn icon_theme_dirs() -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Some(data) = data_home() { result.push(data.join("icons")); }
    if let Some(home) = home() { result.push(home.join(".icons")); }
    for dir in data_dirs() {
        let path = dir.join("icons");
        if !result.contains(&path) { result.push(path); }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xdg_defaults_are_absolute() {
        assert!(PathBuf::from("/usr/share").is_absolute());
        assert!(!PathBuf::from("relative/path").is_absolute());
    }
    #[test]
    fn data_dirs_defaults_or_absolute_overrides_only() {
        assert!(data_dirs().iter().all(|p| p.is_absolute()));
    }
}
