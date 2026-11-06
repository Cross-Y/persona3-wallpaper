//! Restore saved wallpapers without stopping other monitors when one fails.
use std::{collections::HashSet, path::PathBuf};
use crate::{settings_store, theme_manager, image_optimizer};
use p3w_platform::linux::universal_wallpaper::UniversalWallpaperBackend;
use p3w_platform::{WallpaperApplier, WallpaperKind};

pub(crate) fn restore_saved_wallpapers() -> Result<(), String> {
    let backend = UniversalWallpaperBackend::detect();
    let saved = settings_store::saved_wallpaper_entries_for(backend.backend_id());
    if saved.is_empty() { return Ok(()); }
    let connected: HashSet<String> = backend.list_monitors()?.into_iter().map(|m| m.name).collect();
    let mut failures = Vec::new();
    let mut last_restored: Option<(PathBuf, bool)> = None;
    for entry in saved {
        let monitor = entry.monitor;
        let path = entry.path;
        let is_video = entry.is_video;
        // A laptop may have been unplugged since the wallpaper was saved.
        // Keep its saved choice, but do not stop other screens from restoring.
        if !connected.contains(&monitor) {
            eprintln!("Skipping disconnected wallpaper output {monitor}");
            continue;
        }
        let kind = if is_video {
            WallpaperKind::Video
        } else {
            WallpaperKind::Image
        };
        // Persist the ORIGINAL and render the chosen frame to a reusable cache.
        // Restore is synchronous at startup so cropped wallpapers return in the
        // same position rather than temporarily showing a full-size original.
        let rendered = if !is_video {
            match (entry.layout, backend.video_display_size(&monitor)) {
                (Some(layout), Some(size)) => match image_optimizer::prepare_blocking(&path, size, layout) {
                    Ok(cached) => cached,
                    Err(error) => { eprintln!("Cannot restore image crop on {monitor}: {error}"); path.clone() }
                },
                _ => path.clone(),
            }
        } else { path.clone() };
        let applied = match (is_video, entry.video_choice.as_deref()) {
            (true, Some("original")) => backend.apply_original_video(&path, &monitor),
            (true, Some("optimized")) => backend.apply_prepared_video(&path, &monitor)
                .or_else(|_| backend.apply(&path, &monitor, kind)),
            _ => backend.apply(&rendered, &monitor, kind),
        };
        if let Err(e) = applied {
            eprintln!("Could not restore wallpaper on {monitor}: {e}");
            failures.push(format!("{monitor}: {e}"));
        } else {
            if !is_video && rendered != path {
                image_optimizer::protect_active(backend.backend_id(), &monitor, &rendered);
            } else {
                image_optimizer::release_active(backend.backend_id(), &monitor);
            }
            last_restored = Some((path, is_video));
        }
    }
    if let Some((path, is_video)) = last_restored {
        if let Err(error) = theme_manager::apply_for_wallpaper(&path, is_video) {
            eprintln!("Could not refresh dynamic desktop colors: {error}");
        }
    }
    if failures.is_empty() { Ok(()) } else { Err(failures.join("; ")) }
}
