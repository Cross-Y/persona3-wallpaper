//! Desktop-independent domain layer for Persona 3 Wallpaper.

pub mod thumbnail;
pub mod wallpaper;

pub use wallpaper::{Wallpaper, WallpaperKind, WallpaperManager};

/// A desktop integration implements this interface without leaking desktop
/// specific details into the core.
pub trait WallpaperBackend {
    fn id(&self) -> &'static str;
    fn is_available(&self) -> bool;
}
