use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallpaperKind {
    Image,
    Video,
}

/// Applies a wallpaper to one named output.
pub trait WallpaperApplier {
    fn apply(&self, path: &Path, monitor: &str, kind: WallpaperKind) -> Result<(), String>;
}
