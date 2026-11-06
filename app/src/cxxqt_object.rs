use cxx_qt_lib::QString;
use p3w_core::thumbnail::generate_thumbnail;
use p3w_platform::linux::{paths, universal_wallpaper::UniversalWallpaperBackend, video_optimizer::{self, Preparation}};
use p3w_platform::{WallpaperApplier, WallpaperKind};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use crate::{settings_store, theme_manager, image_optimizer::{self, ImageLayout}};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = super::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type WallpaperController = super::WallpaperControllerRust;

        #[qinvokable]
        fn import_file(self: Pin<&mut WallpaperController>, path: QString) -> QString;

        #[qinvokable]
        fn list_wallpapers(self: Pin<&mut WallpaperController>) -> QString;

        #[qinvokable]
        fn search_catalog(self: Pin<&mut WallpaperController>, provider: QString, query: QString, page: i32) -> QString;

        #[qinvokable]
        fn install_remote(self: Pin<&mut WallpaperController>, url: QString, filename: QString) -> QString;

        #[qinvokable]
        fn apply_wallpaper(
            self: Pin<&mut WallpaperController>,
            path: QString,
            monitor: QString,
            is_video: bool,
        ) -> QString;

        #[qinvokable]
        fn wallpaper_media_info(self: Pin<&mut WallpaperController>, path: QString, monitor: QString, is_video: bool) -> QString;

        #[qinvokable]
        fn prepare_video_before_apply(self: Pin<&mut WallpaperController>, path: QString, monitor: QString) -> QString;

        #[qinvokable]
        fn video_preparation_status(self: Pin<&mut WallpaperController>, path: QString, monitor: QString) -> QString;

        #[qinvokable]
        fn cancel_video_preparation(self: Pin<&mut WallpaperController>, path: QString, monitor: QString);

        #[qinvokable]
        fn apply_video_choice(self: Pin<&mut WallpaperController>, path: QString, monitor: QString, optimized: bool) -> QString;

        #[qinvokable]
        fn prepare_image_layout(self: Pin<&mut WallpaperController>, path: QString, monitor: QString, mode: QString, focus_x: f64, focus_y: f64, zoom: f64) -> QString;

        #[qinvokable]
        fn cancel_image_preparation(self: Pin<&mut WallpaperController>, path: QString, monitor: QString, mode: QString, focus_x: f64, focus_y: f64, zoom: f64);

        #[qinvokable]
        fn apply_image_layout(self: Pin<&mut WallpaperController>, path: QString, monitor: QString, mode: QString, focus_x: f64, focus_y: f64, zoom: f64) -> QString;
    }
}

#[derive(Default)]
pub struct WallpaperControllerRust;

fn library_directory() -> Option<PathBuf> {
    paths::data_home().map(|base| base.join("persona3-wallpaper/wallpapers"))
}

fn is_video_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "gif"
    )
}

fn is_supported_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "mp4" | "mkv" | "webm" | "mov" | "avi" | "gif"
    )
}

fn ensure_thumbnail(video: &Path, library_dir: &Path) -> Option<PathBuf> {
    let data_dir = library_dir.parent()?;
    let file_name = video.file_name()?.to_string_lossy();
    let thumbnail_dir = data_dir.join("thumbnails");
    let thumbnail_path = thumbnail_dir.join(format!("{file_name}.jpg"));

    if !thumbnail_path.is_file() {
        if let Err(error) = generate_thumbnail(video, &thumbnail_path) {
            eprintln!(
                "Could not generate thumbnail for {}: {}",
                video.display(),
                error
            );
            return None;
        }
    }

    thumbnail_path.is_file().then_some(thumbnail_path)
}

/// Shared completion path: remember the ORIGINAL asset, never a cache copy.
fn complete_apply(backend: &UniversalWallpaperBackend, original: &Path, monitor: &str,
    is_video: bool, was_paused: bool, layout: Option<&ImageLayout>, video_choice: Option<&str>, result: Result<(), String>) -> QString {
    match result {
        Ok(()) => {
            let saved = if let Some(layout) = layout {
                settings_store::remember_wallpaper_with_layout(backend.backend_id(), &original.to_string_lossy(), monitor, layout)
            } else if let Some(choice) = video_choice {
                settings_store::remember_video_playback_choice(backend.backend_id(), &original.to_string_lossy(), monitor, choice)
            } else {
                settings_store::remember_wallpaper_for(backend.backend_id(), &original.to_string_lossy(), monitor, is_video)
            };
            if let Err(error) = saved { return QString::from(format!("Wallpaper applied, but its selection could not be saved: {error}")); }
            let notice = match theme_manager::apply_for_wallpaper(original, is_video) {
                Ok(info) if settings_store::load()["theme_auto_enabled"].as_bool().unwrap_or(false) => format!(" · {info}"),
                Err(error) => format!(" · Color warning: {error}"),
                _ => String::new(),
            };
            if was_paused { QString::from(format!("Wallpaper queued for {monitor}; resume the engine to display it.{notice}")) }
            else { QString::from(format!("Applied successfully: {} on {monitor}{notice}", original.file_name().and_then(|n| n.to_str()).unwrap_or("wallpaper"))) }
        }
        Err(error) => QString::from(format!("Could not apply wallpaper: {error}")),
    }
}

fn display_for(monitor: &str) -> Result<p3w_platform::linux::video_optimizer::DisplaySize, String> {
    UniversalWallpaperBackend::detect().video_display_size(monitor).ok_or("Monitor resolution unavailable".into())
}

fn optimization_status_text(status: Preparation) -> QString {
    QString::from(match status {
        Preparation::Ready => "ready".to_owned(),
        Preparation::Working => "working".to_owned(),
        Preparation::Failed(error) => format!("ERROR:{error}"),
        Preparation::Unavailable => "ERROR:Conversion stopped before completion".to_owned(),
    })
}

impl qobject::WallpaperController {
    pub fn search_catalog(self: Pin<&mut Self>, provider: QString, query: QString, page: i32) -> QString {
        match crate::url_catalog::search(&provider.to_string(), &query.to_string(), page.max(1) as u32) {
            Ok(items) => QString::from(serde_json::to_string(&items).unwrap_or_else(|_| "{}".into())),
            Err(error) => QString::from(format!("ERROR:{error}")),
        }
    }

    pub fn install_remote(self: Pin<&mut Self>, url: QString, filename: QString) -> QString {
        match crate::url_catalog::download_into_library(&url.to_string(), &filename.to_string()) {
            Ok(path) => {
                let path_buf = PathBuf::from(&path);
                if is_video_file(&path_buf) {
                    if let Some(dir) = library_directory() { let _ = ensure_thumbnail(&path_buf, &dir); }
                }
                QString::from(format!("OK:{path}"))
            }
            Err(error) => QString::from(format!("ERROR:{error}")),
        }
    }
    pub fn apply_wallpaper(
        self: Pin<&mut Self>,
        path: QString,
        monitor: QString,
        is_video: bool,
    ) -> QString {
        let backend = UniversalWallpaperBackend::detect();
        if let Err(error) = backend.support(if is_video { WallpaperKind::Video } else { WallpaperKind::Image }) {
            return QString::from(format!("Could not apply wallpaper: {error}"));
        }
        let path = PathBuf::from(path.to_string());
        let monitor = monitor.to_string();
        let kind = if is_video {
            WallpaperKind::Video
        } else {
            WallpaperKind::Image
        };

        let was_paused = backend.playback_controls() && backend.is_paused();

        let result = backend.apply(&path, &monitor, kind);
        if result.is_ok() { image_optimizer::release_active(backend.backend_id(), &monitor); }
        complete_apply(&backend, &path, &monitor, is_video, was_paused, None, None, result)
    }

    pub fn wallpaper_media_info(self: Pin<&mut Self>, path: QString, monitor: QString, is_video: bool) -> QString {
        let path = PathBuf::from(path.to_string());
        let backend = UniversalWallpaperBackend::detect();
        let size = backend.video_display_size(&monitor.to_string());
        let dims = if is_video { video_optimizer::inspect_video(&path) } else { image_optimizer::inspect_image(&path) };
        match dims {
            Ok((width, height)) => {
                let screen = size.unwrap_or(p3w_platform::linux::video_optimizer::DisplaySize { width: 0, height: 0, refresh: 0 });
                let oversized = screen.valid() && (width > screen.width || height > screen.height);
                let saved_layout = if is_video { None } else {
                    settings_store::saved_wallpaper_entries_for(backend.backend_id()).into_iter()
                        .find(|entry| entry.monitor == monitor.to_string() && entry.path == path)
                        .and_then(|entry| entry.layout.map(|layout| layout.to_value()))
                };
                QString::from(json!({"ok":true,"source_width":width,"source_height":height,
                    "screen_width":screen.width,"screen_height":screen.height,
                    "monitor_known":screen.valid(),"oversized":oversized,
                    "image_layout":saved_layout,
                    "cached": is_video && size.and_then(|s| video_optimizer::prepared_path(&path, s)).is_some()}).to_string())
            }
            Err(error) => QString::from(json!({"ok":false,"error":error}).to_string()),
        }
    }

    pub fn prepare_video_before_apply(self: Pin<&mut Self>, path: QString, monitor: QString) -> QString {
        let path = PathBuf::from(path.to_string());
        match display_for(&monitor.to_string()).and_then(|s| video_optimizer::request_preparation(&path, s)) {
            Ok(status) => optimization_status_text(status),
            Err(error) => QString::from(format!("ERROR:{error}")),
        }
    }

    pub fn video_preparation_status(self: Pin<&mut Self>, path: QString, monitor: QString) -> QString {
        let path = PathBuf::from(path.to_string());
        match display_for(&monitor.to_string()) {
            Ok(size) => optimization_status_text(video_optimizer::preparation_status(&path, size)),
            Err(error) => QString::from(format!("ERROR:{error}")),
        }
    }

    pub fn cancel_video_preparation(self: Pin<&mut Self>, path: QString, monitor: QString) {
        if let Ok(size) = display_for(&monitor.to_string()) {
            video_optimizer::cancel_preparation(&PathBuf::from(path.to_string()), size);
        }
    }

    pub fn apply_video_choice(self: Pin<&mut Self>, path: QString, monitor: QString, optimized: bool) -> QString {
        let backend = UniversalWallpaperBackend::detect();
        let path = PathBuf::from(path.to_string());
        let monitor = monitor.to_string();
        let paused = backend.playback_controls() && backend.is_paused();
        let result = if optimized { backend.apply_prepared_video(&path, &monitor) }
            else { backend.apply_original_video(&path, &monitor) };
        if result.is_ok() { image_optimizer::release_active(backend.backend_id(), &monitor); }
        complete_apply(&backend, &path, &monitor, true, paused, None, Some(if optimized { "optimized" } else { "original" }), result)
    }

    pub fn prepare_image_layout(self: Pin<&mut Self>, path: QString, monitor: QString, mode: QString,
        focus_x: f64, focus_y: f64, zoom: f64) -> QString {
        let result = ImageLayout::new(&mode.to_string(), focus_x, focus_y, zoom)
            .and_then(|layout| display_for(&monitor.to_string()).and_then(|size|
                image_optimizer::prepare(&PathBuf::from(path.to_string()), size, layout).map(str::to_string)));
        match result { Ok(status) => QString::from(status), Err(error) => QString::from(format!("ERROR:{error}")) }
    }

    pub fn cancel_image_preparation(self: Pin<&mut Self>, path: QString, monitor: QString, mode: QString,
        focus_x: f64, focus_y: f64, zoom: f64) {
        if let (Ok(layout), Ok(size)) = (ImageLayout::new(&mode.to_string(), focus_x, focus_y, zoom), display_for(&monitor.to_string())) {
            image_optimizer::cancel(&PathBuf::from(path.to_string()), size, layout);
        }
    }

    pub fn apply_image_layout(self: Pin<&mut Self>, path: QString, monitor: QString, mode: QString,
        focus_x: f64, focus_y: f64, zoom: f64) -> QString {
        let backend = UniversalWallpaperBackend::detect();
        let path = PathBuf::from(path.to_string());
        let monitor = monitor.to_string();
        let layout = match ImageLayout::new(&mode.to_string(), focus_x, focus_y, zoom) {
            Ok(value) => value, Err(error) => return QString::from(format!("ERROR:{error}")),
        };
        let prepared = match display_for(&monitor).and_then(|size| image_optimizer::prepared(&path, size, layout.clone())) {
            Ok(value) => value, Err(error) => return QString::from(format!("ERROR:{error}")),
        };
        let paused = backend.playback_controls() && backend.is_paused();
        let result = backend.apply(&prepared, &monitor, WallpaperKind::Image);
        if result.is_ok() { image_optimizer::protect_active(backend.backend_id(), &monitor, &prepared); }
        complete_apply(&backend, &path, &monitor, false, paused, Some(&layout), None, result)
    }

    pub fn import_file(self: Pin<&mut Self>, path: QString) -> QString {
        let source = PathBuf::from(path.to_string());

        if !source.is_file() {
            return QString::from("File does not exist.");
        }

        if !is_supported_file(&source) {
            return QString::from("Unsupported file format.");
        }

        let Some(destination_dir) = library_directory() else {
            return QString::from("Could not locate data directory.");
        };

        if let Err(error) = std::fs::create_dir_all(&destination_dir) {
            return QString::from(format!("Could not create library: {error}"));
        }

        let Some(file_name) = source.file_name() else {
            return QString::from("Invalid file path.");
        };

        let mut destination = destination_dir.join(file_name);

        if destination.exists() {
            let stem = source
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("wallpaper");

            let extension = source
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            let mut index = 1;
            loop {
                let new_name = if extension.is_empty() {
                    format!("{stem} ({index})")
                } else {
                    format!("{stem} ({index}).{extension}")
                };

                destination = destination_dir.join(new_name);
                if !destination.exists() {
                    break;
                }
                index += 1;
            }
        }

        match std::fs::copy(&source, &destination) {
            Ok(_) => {
                let mut message = format!(
                    "Imported successfully: {}",
                    destination
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("wallpaper")
                );

                if is_video_file(&destination)
                    && ensure_thumbnail(&destination, &destination_dir).is_none()
                {
                    message.push_str(" (thumbnail could not be generated)");
                }

                QString::from(message)
            }
            Err(error) => QString::from(format!("Import failed: {error}")),
        }
    }

    pub fn list_wallpapers(self: Pin<&mut Self>) -> QString {
        let Some(directory) = library_directory() else {
            return QString::from("[]");
        };

        if let Err(error) = std::fs::create_dir_all(&directory) {
            eprintln!("Could not read wallpaper library: {error}");
            return QString::from("[]");
        }

        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("Could not read wallpaper directory: {error}");
                return QString::from("[]");
            }
        };

        let mut wallpapers = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !is_supported_file(&path) {
                continue;
            }

            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Wallpaper");

            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            let is_video = is_video_file(&path);
            let thumbnail_path = if is_video {
                ensure_thumbnail(&path, &directory)
                    .map(|thumbnail| thumbnail.to_string_lossy().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };

            wallpapers.push(json!({
                "name": name,
                "path": path.to_string_lossy(),
                "extension": extension,
                "is_video": is_video,
                "thumbnail_path": thumbnail_path
            }));
        }

        wallpapers.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
        });

        match serde_json::to_string(&wallpapers) {
            Ok(data) => QString::from(data),
            Err(error) => {
                eprintln!("Could not serialize wallpaper library: {error}");
                QString::from("[]")
            }
        }
    }
}
