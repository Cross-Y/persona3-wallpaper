pub mod monitor;
pub mod wallpaper;

#[cfg(target_os = "linux")]
pub mod linux;

pub use monitor::{Monitor, MonitorProvider};
pub use wallpaper::{WallpaperApplier, WallpaperKind};
