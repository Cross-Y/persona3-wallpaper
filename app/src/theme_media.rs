//! Cache video preview frames and extract a tiny bounded RGB sample using FFmpeg.
use std::{fs, path::{Path, PathBuf}, process::{Command, Stdio}, time::UNIX_EPOCH};
use p3w_platform::linux::{paths, environment::command_exists};

fn is_animated(path: &Path, video: bool) -> bool {
    video || path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("gif")).unwrap_or(false)
}

// Prune only on a newly generated video frame, never while browsing. Keeping
// at most 128 JPEG previews prevents an unbounded XDG cache on long-lived use.
fn prune_frame_cache(dir: &Path, current: &Path) {
    const MAX_FRAMES: usize = 128;
    let Ok(entries) = fs::read_dir(dir) else { return; };
    let mut frames: Vec<(std::time::SystemTime, PathBuf)> = entries.flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("jpg")
            && !path.file_name().and_then(|s| s.to_str()).unwrap_or("").contains(".tmp.jpg")
            && path.is_file())
        .map(|path| {
            let modified = fs::metadata(&path).and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
            (modified, path)
        }).collect();
    if frames.len() <= MAX_FRAMES { return; }
    frames.sort_by_key(|(modified, _)| *modified);
    let excess = frames.len() - MAX_FRAMES;
    for (_, path) in frames.into_iter().filter(|(_, path)| path != current).take(excess) {
        let _ = fs::remove_file(path);
    }
}

/// Return a still image. Animated files use a cached JPEG from FFmpeg;
/// static files go straight to Noctalia without transcoding.
pub(crate) fn source_image(source: &Path, video: bool) -> Result<PathBuf, String> {
    if !source.is_file() { return Err(format!("Wallpaper does not exist: {}", source.display())); }
    if !is_animated(source,video) { return Ok(source.to_path_buf()); }
    let cache = paths::cache_home().ok_or("Could not locate XDG_CACHE_HOME")?.join("persona3-wallpaper/theme-frames");
    fs::create_dir_all(&cache).map_err(|e| format!("Could not create frame cache: {e}"))?;
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    let frame = cache.join(format!("{:016x}.jpg",hasher.finish()));
    // Thumbnail extraction validates mtime and atomically replaces stale data.
    let existed = frame.is_file();
    p3w_core::thumbnail::generate_thumbnail(source, &frame)?;
    if !existed { prune_frame_cache(&cache, &frame); }
    Ok(frame)
}

/// FFmpeg is already a project dependency for video previews; using 64x64 RGB
/// avoids decoding full-resolution images into the Qt process or adding deps.
pub(crate) fn sample_rgb(path: &Path) -> Result<Vec<u8>, String> {
    if !command_exists("ffmpeg") { return Err("FFmpeg is required to extract colors".into()); }
    let output = Command::new("ffmpeg").args(["-nostdin","-hide_banner","-loglevel","error","-i"])
        .arg(path).args(["-frames:v","1","-vf","scale=64:64:flags=area,format=rgb24","-pix_fmt","rgb24","-f","rawvideo","-"])
        .stdin(Stdio::null()).output().map_err(|e| format!("Could not run FFmpeg: {e}"))?;
    if !output.status.success() || output.stdout.len() != 64*64*3 {
        return Err(format!("Could not extract color palette: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(output.stdout)
}

