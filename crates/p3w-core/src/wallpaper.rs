use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WallpaperKind {
    Video,
    Image,
}

#[derive(Debug, Clone)]
pub struct Wallpaper {
    pub id: String,
    pub title: String,
    pub source: PathBuf,
    pub kind: WallpaperKind,
}

#[derive(Debug, Default)]
pub struct WallpaperManager {
    items: Vec<Wallpaper>,
}

impl WallpaperManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn all(&self) -> &[Wallpaper] {
        &self.items
    }

    pub fn add(&mut self, wallpaper: Wallpaper) {
        self.items.push(wallpaper);
    }
}
