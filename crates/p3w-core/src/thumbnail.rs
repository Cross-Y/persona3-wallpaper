use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_THUMBNAIL: AtomicU64 = AtomicU64::new(0);

/// Generates a JPEG thumbnail from a video using FFmpeg.
///
/// Tries to extract a frame at second 2, then falls back to second 0.
pub fn generate_thumbnail(video_path: &Path, thumbnail_path: &Path) -> Result<(), String> {
    if !video_path.is_file() {
        return Err("Video file does not exist.".into());
    }

    // Invalidate an old preview when the original video is replaced.
    let fresh = fs::metadata(thumbnail_path).ok().filter(|metadata| metadata.len() > 0)
        .and_then(|metadata| metadata.modified().ok())
        .zip(fs::metadata(video_path).and_then(|metadata| metadata.modified()).ok())
        .map(|(thumbnail_time, video_time)| thumbnail_time >= video_time)
        .unwrap_or(false);
    if fresh { return Ok(()); }

    let parent = thumbnail_path.parent().ok_or("Invalid thumbnail path.")?;

    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create thumbnail directory: {error}"))?;

    let file_name = thumbnail_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Invalid thumbnail filename.")?;

    let serial = NEXT_THUMBNAIL.fetch_add(1, Ordering::Relaxed);
    let temporary_path = parent.join(format!("{file_name}.{}-{serial}.tmp.jpg", std::process::id()));

    let mut last_error = String::from("FFmpeg could not extract a video frame.");

    for timestamp in ["2", "0"] {
        let result = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-ss",
                timestamp,
                "-i",
            ])
            .arg(video_path)
            .args([
                "-frames:v",
                "1",
                "-vf",
                "scale='min(640,iw)':-2",
                "-q:v",
                "3",
            ])
            .arg(&temporary_path)
            .output();

        match result {
            Ok(output) => {
                if output.status.success() && temporary_path.is_file() {
                    match fs::rename(&temporary_path, thumbnail_path) {
                        Ok(()) => return Ok(()),
                        Err(error) => {
                            return Err(format!("Could not save thumbnail: {error}"));
                        }
                    }
                }

                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

                if !stderr.is_empty() {
                    last_error = stderr;
                }
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary_path);
                return Err(format!("Could not execute FFmpeg: {error}"));
            }
        }

        let _ = fs::remove_file(&temporary_path);
    }

    Err(last_error)
}
