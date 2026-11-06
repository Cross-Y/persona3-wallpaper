//! Non-blocking video downscaling for the actual display resolution.
//!
//! The source file and saved wallpaper selection are never modified. A single
//! low-priority FFmpeg worker produces reusable, atomically published MP4s.
//! Failed or cancelled conversions leave the original video playing.
use std::{collections::{HashMap, HashSet}, fs, hash::{Hash, Hasher},
    path::{Path, PathBuf}, process::{Command, Stdio},
    sync::{mpsc::{self, SyncSender}, Mutex, OnceLock}, thread, time::{Duration, SystemTime, UNIX_EPOCH}};

use super::{environment, paths, universal_wallpaper::UniversalWallpaperBackend};

const CACHE_VERSION: u32 = 2;
const CACHE_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB, best-effort LRU.
const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_DURATION_SECONDS: f64 = 600.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplaySize { pub width: u32, pub height: u32, pub refresh: u32 }
impl DisplaySize {
    pub fn valid(self) -> bool { self.width >= 320 && self.height >= 240 && self.width <= 16384 && self.height <= 16384 }
    fn max_fps(self) -> u32 { if self.refresh == 0 { 60 } else { self.refresh.clamp(24, 60) } }
}

#[derive(Clone)]
struct Selection { source: PathBuf, cache: PathBuf, backend: String, monitor: String }
struct State { enabled: bool, selected: HashMap<String, Selection>, queued: HashSet<PathBuf>, skipped: HashSet<PathBuf>, waiting: HashSet<PathBuf>, errors: HashMap<PathBuf, String> }
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
static WORKER: OnceLock<SyncSender<Job>> = OnceLock::new();
static APPLY_LOCK: Mutex<()> = Mutex::new(());
fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State { enabled: true, selected: HashMap::new(), queued: HashSet::new(), skipped: HashSet::new(), waiting: HashSet::new(), errors: HashMap::new() }))
}
pub fn apply_lock() -> &'static Mutex<()> { &APPLY_LOCK }
pub fn set_enabled(enabled: bool) {
    // Serialize with an in-flight cache upgrade to avoid applying an optimized
    // file after the user has explicitly disabled the feature.
    if let Ok(_apply) = APPLY_LOCK.lock() {
        if let Ok(mut s) = state().lock() {
            s.enabled = enabled;
            if !enabled { s.selected.clear(); }
            // An explicit toggle is a useful point to retry previously failed
            // jobs after the user installs FFmpeg or frees up disk space.
            s.skipped.clear();
            s.errors.clear();
        }
    }
}
pub fn enabled() -> bool { state().lock().map(|s| s.enabled).unwrap_or(false) }
pub fn status_text() -> String {
    if enabled() && (!environment::command_exists("ffmpeg") || !environment::command_exists("ffprobe")) {
        return "Install FFmpeg and FFprobe to enable automatic video conversion.".into();
    }
    match state().lock() {
        Ok(s) if !s.enabled => "Disabled — original videos are used.".into(),
        Ok(s) if !s.queued.is_empty() => format!("Optimizing or queued: {} video(s). Playback remains available.", s.queued.len()),
        Ok(_) => "Ready — optimized videos are reused automatically when available.".into(),
        Err(_) => "Video optimizer temporarily unavailable.".into(),
    }
}
pub fn invalidate(backend: &str, monitor: &str) {
    if let Ok(mut s) = state().lock() { s.selected.remove(&format!("{backend}/{monitor}")); }
}
pub fn invalidate_all() {
    if let Ok(mut s) = state().lock() { s.selected.clear(); }
}
/// Cancel pre-play conversions when the application exits.
pub fn cancel_all_preparations() {
    if let Ok(mut s) = state().lock() { s.waiting.clear(); }
}

#[derive(Clone)]
struct Job { source: PathBuf, cache: PathBuf, size: DisplaySize }
fn worker() -> &'static SyncSender<Job> {
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel::<Job>(4);
        thread::Builder::new().name("p3w-video-optimizer".into()).spawn(move || {
            while let Ok(job) = rx.recv() {
                if wanted(&job.cache) {
                    if let Err(error) = convert(&job) {
                        eprintln!("Wallpaper optimization skipped: {error}");
                        if let Ok(mut s) = state().lock() {
                            if error != "Conversion cancelled" {
                                if s.skipped.len() >= 256 { s.skipped.clear(); }
                                s.skipped.insert(job.cache.clone());
                                if s.errors.len() >= 64 { s.errors.clear(); }
                                s.errors.insert(job.cache.clone(), error);
                            }
                        }
                    }
                    if is_ready(&job.cache) { activate(&job.cache); }
                }
                if let Ok(mut s) = state().lock() { s.queued.remove(&job.cache); }
            }
        }).expect("Could not start wallpaper optimization worker");
        tx
    })
}
fn wanted(path: &Path) -> bool {
    state().lock().map(|s|
        s.waiting.contains(path) || (s.enabled && s.selected.values().any(|v|
            v.cache == path && v.source.is_file()
        ))
    ).unwrap_or(false)
}
fn is_ready(path: &Path) -> bool {
    path.metadata().map(|m| m.is_file() && m.len() > 1024).unwrap_or(false)
}
fn cache_path(source: &Path, size: DisplaySize) -> Option<PathBuf> {
    let meta = source.metadata().ok()?;
    let stamp = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    CACHE_VERSION.hash(&mut hasher);
    source.hash(&mut hasher);
    meta.len().hash(&mut hasher);
    stamp.as_nanos().hash(&mut hasher);
    size.width.hash(&mut hasher);
    size.height.hash(&mut hasher);
    size.max_fps().hash(&mut hasher);
    let filename = format!("{:016x}-{}x{}-{}fps.mp4", hasher.finish(), size.width, size.height, size.max_fps());
    Some(paths::cache_home()?.join("persona3-wallpaper/optimized-video").join(filename))
}
/// Called with APPLY_LOCK already held, after the desktop/monitor has been validated.
/// Returns cached video immediately when present; otherwise streams the original
/// now, queues at most one conversion per cache key, and upgrades later.
pub fn select(source: &Path, backend: &str, monitor: &str, size: Option<DisplaySize>) -> PathBuf {
    let key = format!("{backend}/{monitor}");
    let Some(size) = size.filter(|v| v.valid()) else { invalidate(backend, monitor); return source.to_path_buf(); };
    let Some(cache) = cache_path(source, size) else { invalidate(backend, monitor); return source.to_path_buf(); };
    let mut queue = false;
    {
        let Ok(mut s) = state().lock() else { return source.to_path_buf(); };
        if !s.enabled { s.selected.remove(&key); return source.to_path_buf(); }
        if s.skipped.contains(&cache) { s.selected.remove(&key); return source.to_path_buf(); }
        s.selected.insert(key, Selection { source: source.to_path_buf(), cache: cache.clone(), backend: backend.to_owned(), monitor: monitor.to_owned() });
        if is_ready(&cache) { return cache; }
        if s.queued.insert(cache.clone()) { queue = true; }
    }
    if queue {
        if worker().try_send(Job { source: source.to_path_buf(), cache: cache.clone(), size }).is_err() {
            if let Ok(mut s) = state().lock() { s.queued.remove(&cache); }
        }
    }
    source.to_path_buf()
}
fn probe(path: &Path) -> Result<(u32, u32, f64, f64), String> {
    let output = Command::new("ffprobe").args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height,avg_frame_rate:format=duration", "-of", "json"])
        .arg(path).output().map_err(|e| format!("ffprobe unavailable: {e}"))?;
    if !output.status.success() { return Err("Could not inspect video".into()); }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let stream = &value["streams"][0];
    let w = stream["width"].as_u64().unwrap_or(0) as u32;
    let h = stream["height"].as_u64().unwrap_or(0) as u32;
    if w == 0 || h == 0 { return Err("Invalid video dimensions".into()); }
    let rate = stream["avg_frame_rate"].as_str().unwrap_or("0/1");
    let fps = parse_fps(rate);
    let duration = value["format"]["duration"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    Ok((w, h, fps, duration))
}
/// Lightweight preflight for the UI. The conversion is never run on the Qt thread.
pub fn inspect_video(path: &Path) -> Result<(u32, u32), String> {
    let (w, h, _, _) = probe(path)?;
    Ok((w, h))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preparation { Ready, Working, Failed(String), Unavailable }

pub fn preparation_status(source: &Path, size: DisplaySize) -> Preparation {
    let Some(cache) = cache_path(source, size) else { return Preparation::Unavailable; };
    if is_ready(&cache) { return Preparation::Ready; }
    let Ok(s) = state().lock() else { return Preparation::Unavailable; };
    if let Some(error) = s.errors.get(&cache) { return Preparation::Failed(error.clone()); }
    if s.queued.contains(&cache) { return Preparation::Working; }
    Preparation::Unavailable
}

/// Enqueue a pre-play conversion. This does NOT select/play the source video:
/// current wallpaper remains visible while FFmpeg is working.
pub fn request_preparation(source: &Path, size: DisplaySize) -> Result<Preparation, String> {
    if !size.valid() { return Err("Monitor resolution is not available".into()); }
    if !source.is_file() { return Err("Video does not exist".into()); }
    if !environment::command_exists("ffmpeg") || !environment::command_exists("ffprobe") {
        return Err("Install FFmpeg and FFprobe to optimize videos".into());
    }
    let cache = cache_path(source, size).ok_or("Could not locate optimization cache")?;
    if is_ready(&cache) { return Ok(Preparation::Ready); }
    let should_queue = {
        let mut s = state().lock().map_err(|_| "Optimizer is unavailable")?;
        s.waiting.insert(cache.clone());
        s.errors.remove(&cache);
        s.skipped.remove(&cache);
        s.queued.insert(cache.clone())
    };
    if should_queue && worker().try_send(Job { source: source.to_path_buf(), cache: cache.clone(), size }).is_err() {
        if let Ok(mut s) = state().lock() {
            s.queued.remove(&cache);
            s.waiting.remove(&cache);
        }
        return Err("Optimization queue is full. Try again in a moment".into());
    }
    Ok(Preparation::Working)
}

/// Called if user cancels the modal or chooses to play the original instead.
pub fn cancel_preparation(source: &Path, size: DisplaySize) {
    if let Some(cache) = cache_path(source, size) {
        if let Ok(mut s) = state().lock() { s.waiting.remove(&cache); }
    }
}

/// Only returns a verified, completed cache entry; never expose temp files.
pub fn prepared_path(source: &Path, size: DisplaySize) -> Option<PathBuf> {
    let cache = cache_path(source, size)?;
    is_ready(&cache).then_some(cache)
}

fn parse_fps(rate: &str) -> f64 {
    let mut parts = rate.split('/');
    let numerator = parts.next().and_then(|n| n.parse::<f64>().ok()).unwrap_or(0.0);
    let denominator = parts.next().and_then(|n| n.parse::<f64>().ok()).unwrap_or(1.0);
    if denominator <= 0.0 { 0.0 } else { numerator / denominator }
}
fn convert(job: &Job) -> Result<(), String> {
    if !wanted(&job.cache) || is_ready(&job.cache) { return Ok(()); }
    let meta = job.source.metadata().map_err(|e| e.to_string())?;
    if meta.len() > MAX_SOURCE_BYTES { return Err("Source is too large for automatic conversion".into()); }
    let (w, h, fps, duration) = probe(&job.source)?;
    if duration > MAX_DURATION_SECONDS { return Err("Video is longer than 10 minutes; skipping automatic conversion".into()); }
    let needs_scale = w > job.size.width || h > job.size.height;
    let needs_fps = fps > f64::from(job.size.max_fps()) + 0.5;
    if !needs_scale && !needs_fps {
        // A 1080p/60 video on a 1080p/60 monitor needs no conversion. Avoid
        // running ffprobe again every time the user applies this wallpaper.
        if let Ok(mut s) = state().lock() {
            if s.skipped.len() >= 256 { s.skipped.clear(); }
            s.skipped.insert(job.cache.clone());
        }
        return Ok(());
    }
    let parent = job.cache.parent().ok_or("Invalid cache destination")?;
    fs::create_dir_all(parent).map_err(|e| format!("Cannot create optimization cache: {e}"))?;
    cleanup_old_temps(parent);
    let temp = parent.join(format!(".{}-{}.tmp.mp4", job.cache.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("video")).to_string_lossy(), std::process::id()));
    let mut command = Command::new("ffmpeg");
    command.args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y", "-threads", "2", "-i"]).arg(&job.source)
        .args(["-map", "0:v:0", "-map", "0:a?", "-sn", "-dn"]);
    let mut filters = Vec::new();
    if needs_scale {
        filters.push(format!("scale={}:{}:force_original_aspect_ratio=decrease:force_divisible_by=2:flags=bicubic", job.size.width, job.size.height));
    }
    if needs_fps { filters.push(format!("fps={}", job.size.max_fps())); }
    if !filters.is_empty() { command.args(["-vf", &filters.join(",")]); }
    command.args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "24", "-pix_fmt", "yuv420p", "-threads", "2", "-c:a", "aac", "-b:a", "128k", "-movflags", "+faststart"])
        .arg(&temp).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    // Do not let a background render monopolize the machine. Ignore EPERM.
    use std::os::unix::process::CommandExt;
    unsafe { command.pre_exec(|| {
        libc::setpriority(libc::PRIO_PROCESS, 0, 10);
        if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 { return Err(std::io::Error::last_os_error()); }
        Ok(())
    }); }
    let mut child = command.spawn().map_err(|e| format!("FFmpeg unavailable: {e}"))?;
    let result = loop {
        if !wanted(&job.cache) { let _ = child.kill(); let _ = child.wait(); break Err("Conversion cancelled".into()); }
        match child.try_wait() {
            Ok(Some(status)) => break if status.success() { Ok(()) } else { Err(format!("FFmpeg failed ({status})")) },
            Ok(None) => thread::sleep(Duration::from_millis(300)),
            Err(e) => { let _ = child.kill(); let _ = child.wait(); break Err(e.to_string()); }
        }
    };
    if let Err(e) = result { let _ = fs::remove_file(&temp); return Err(e); }
    let valid = probe(&temp).map(|(ow, oh, _, _)| ow > 0 && oh > 0 && ow <= job.size.width && oh <= job.size.height).unwrap_or(false);
    if !valid || !is_ready(&temp) { let _ = fs::remove_file(&temp); return Err("Optimized output could not be verified".into()); }
    if !wanted(&job.cache) || cache_path(&job.source, job.size).as_ref() != Some(&job.cache) {
        let _ = fs::remove_file(&temp);
        return Ok(());
    }
    fs::rename(&temp, &job.cache).map_err(|e| { let _ = fs::remove_file(&temp); format!("Cannot publish optimized video: {e}") })?;
    Ok(())
}
/// Old interrupted transcodes are safe to discard; never touch recent temps
/// which might belong to another running instance of the application.
fn cleanup_old_temps(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|x| x.to_str()).unwrap_or("");
        if !name.starts_with('.') || !name.ends_with(".tmp.mp4") { continue; }
        let stale = entry.metadata().ok().and_then(|m| m.modified().ok())
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .map(|age| age > Duration::from_secs(24 * 60 * 60)).unwrap_or(false);
        if stale { let _ = fs::remove_file(&path); }
    }
}
fn activate(cache: &Path) {
    {
        let Ok(_apply) = APPLY_LOCK.lock() else { return; };
        let selected = state().lock().ok().map(|s| {
            if !s.enabled { return Vec::new(); }
            s.selected.values().filter(|v| v.cache == cache && v.source.is_file()).cloned().collect::<Vec<_>>()
        }).unwrap_or_default();
        for selection in selected {
            let backend = UniversalWallpaperBackend::detect();
            if backend.backend_id() != selection.backend { continue; }
            if let Err(e) = backend.activate_optimized(cache, &selection.monitor) {
                eprintln!("Could not switch to optimized wallpaper ({}): {e}", selection.monitor);
            }
        }
    }
    prune_cache(cache);
}
fn prune_cache(recent: &Path) {
    let Some(parent) = recent.parent() else { return; };
    let Ok(entries) = fs::read_dir(parent) else { return; };
    let protected: HashSet<PathBuf> = state().lock().map(|s| s.selected.values().map(|v| v.cache.clone()).collect()).unwrap_or_default();
    let mut files = Vec::new();
    let mut total = 0_u64;
    for item in entries.flatten() {
        let p = item.path();
        if p.extension().and_then(|x| x.to_str()) != Some("mp4") || p.file_name().and_then(|s| s.to_str()).unwrap_or("").starts_with('.') { continue; }
        if let Ok(m) = item.metadata() {
            total = total.saturating_add(m.len());
            files.push((p, m.len(), m.accessed().or_else(|_| m.modified()).unwrap_or(UNIX_EPOCH)));
        }
    }
    files.sort_by_key(|f| f.2);
    for (path, bytes, _) in files {
        if total <= CACHE_LIMIT_BYTES { break; }
        if path == recent || protected.contains(&path) { continue; }
        if fs::remove_file(&path).is_ok() { total = total.saturating_sub(bytes); }
    }
}

/// Parse a typical xrandr connected-output line; use the largest display for
/// non-per-monitor X11 desktops, never downsize beneath any attached monitor.
pub fn largest_x11_display(text: &str) -> Option<DisplaySize> {
    let mut biggest: Option<DisplaySize> = None;
    for line in text.lines().filter(|s| s.contains(" connected")) {
        for token in line.split_whitespace() {
            let part = token.split('+').next().unwrap_or("");
            let mut wh = part.split('x');
            let dims = wh.next().and_then(|w| w.parse::<u32>().ok())
                .zip(wh.next().and_then(|h| h.parse::<u32>().ok()));
            if let Some((width, height)) = dims {
                let result = DisplaySize { width, height, refresh: 60 };
                if result.valid() && biggest.map(|v| width as u64 * height as u64 > v.width as u64 * v.height as u64).unwrap_or(true) { biggest = Some(result); }
            }
        }
    }
    biggest
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn fps_is_safe() {
        assert!((parse_fps("30000/1001") - 29.97).abs() < 0.01);
        assert_eq!(parse_fps("30/0"), 0.0);
        assert_eq!(parse_fps("bogus"), 0.0);
    }
    #[test] fn xrandr_uses_largest_output() {
        let data = "eDP-1 connected primary 1920x1080+0+0\nHDMI-1 connected 3840x2160+1920+0\nHDMI-2 disconnected";
        assert_eq!(largest_x11_display(data), Some(DisplaySize { width: 3840, height: 2160, refresh: 60 }));
    }
    #[test] fn reject_unknown_monitor_sizes() {
        assert!(!DisplaySize { width: 0, height: 0, refresh: 60 }.valid());
    }
}
