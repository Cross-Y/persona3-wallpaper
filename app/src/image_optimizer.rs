//! Non-destructive static wallpaper framing. Original assets are never edited.
//! FFmpeg renders a monitor-sized JPEG in a fingerprinted cache; UI requests
//! run on a single low-priority worker instead of blocking Qt's event loop.
use p3w_platform::linux::{paths, video_optimizer::DisplaySize};
use serde_json::{json, Value};
use std::{collections::{HashMap, HashSet}, fs, hash::{Hash, Hasher}, path::{Path, PathBuf},
    process::{Command, Stdio}, sync::{mpsc::{self, SyncSender}, Mutex, OnceLock}, thread,
    time::{Duration, SystemTime, UNIX_EPOCH}};

#[derive(Debug, Clone)]
pub struct ImageLayout { pub mode: String, pub focus_x: f64, pub focus_y: f64, pub zoom: f64 }
impl ImageLayout {
    pub fn new(mode: &str, focus_x: f64, focus_y: f64, zoom: f64) -> Result<Self, String> {
        if !["fit", "stretch", "crop"].contains(&mode) { return Err("Unknown image layout".into()); }
        if !focus_x.is_finite() || !focus_y.is_finite() || !zoom.is_finite() {
            return Err("Invalid image crop values".into());
        }
        Ok(Self { mode: mode.into(), focus_x: focus_x.clamp(0.0, 1.0),
            focus_y: focus_y.clamp(0.0, 1.0), zoom: zoom.clamp(1.0, 3.0) })
    }
    pub fn from_value(v: &Value) -> Option<Self> {
        Self::new(v["mode"].as_str()?, v["focus_x"].as_f64().unwrap_or(0.5),
            v["focus_y"].as_f64().unwrap_or(0.5), v["zoom"].as_f64().unwrap_or(1.0)).ok()
    }
    pub fn to_value(&self) -> Value {
        json!({"mode": self.mode, "focus_x": self.focus_x, "focus_y": self.focus_y, "zoom": self.zoom})
    }
}

#[derive(Clone)]
struct Job { source: PathBuf, destination: PathBuf, size: DisplaySize, layout: ImageLayout }
#[derive(Default)]
struct State { pending: HashSet<PathBuf>, requested: HashSet<PathBuf>, errors: HashMap<PathBuf, String>, pinned: HashMap<String, PathBuf> }
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
static WORKER: OnceLock<SyncSender<Job>> = OnceLock::new();
fn state() -> &'static Mutex<State> { STATE.get_or_init(|| Mutex::new(State::default())) }
fn wanted(path: &Path) -> bool { state().lock().map(|s| s.requested.contains(path)).unwrap_or(false) }
fn worker() -> &'static SyncSender<Job> {
    WORKER.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<Job>(2);
        thread::Builder::new().name("p3w-static-image-worker".into()).spawn(move || {
            while let Ok(job) = receiver.recv() {
                if wanted(&job.destination) {
                    if let Err(error) = convert(&job, true) {
                        if error != "Cancelled" {
                            eprintln!("Static wallpaper framing failed: {error}");
                            if let Ok(mut s) = state().lock() {
                                if s.errors.len() > 32 { s.errors.clear(); }
                                s.errors.insert(job.destination.clone(), error);
                            }
                        }
                    }
                }
                if let Ok(mut s) = state().lock() {
                    s.pending.remove(&job.destination);
                    s.requested.remove(&job.destination);
                }
                prune_cache(&job.destination);
            }
        }).expect("Cannot launch image preparation worker");
        sender
    })
}
fn destination(source: &Path, size: DisplaySize, layout: &ImageLayout) -> Result<PathBuf, String> {
    let meta = source.metadata().map_err(|e| format!("Cannot inspect image: {e}"))?;
    if !meta.is_file() { return Err("Image is not a file".into()); }
    let modified = meta.modified().map_err(|e| e.to_string())?.duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    1_u8.hash(&mut hash);
    source.hash(&mut hash);
    meta.len().hash(&mut hash);
    modified.as_nanos().hash(&mut hash);
    size.width.hash(&mut hash); size.height.hash(&mut hash);
    layout.mode.hash(&mut hash);
    ((layout.focus_x * 1000.0).round() as u16).hash(&mut hash);
    ((layout.focus_y * 1000.0).round() as u16).hash(&mut hash);
    ((layout.zoom * 1000.0).round() as u16).hash(&mut hash);
    let folder = paths::cache_home().ok_or("Cannot locate image cache")?.join("persona3-wallpaper/framed-images");
    Ok(folder.join(format!("{:016x}-{}x{}.jpg", hash.finish(), size.width, size.height)))
}
fn ready(path: &Path) -> bool { path.metadata().map(|m| m.is_file() && m.len() > 1024).unwrap_or(false) }
fn validate_size(size: DisplaySize) -> Result<(), String> {
    if !size.valid() { return Err("Monitor size is unavailable".into()); }
    if u64::from(size.width) * u64::from(size.height) > 33_000_000 {
        return Err("Image output exceeds safe 8K processing size".into());
    }
    Ok(())
}
fn job(source: &Path, size: DisplaySize, layout: ImageLayout) -> Result<Job, String> {
    validate_size(size)?;
    if !source.is_file() { return Err("Image does not exist".into()); }
    let canonical = source.canonicalize().map_err(|e| e.to_string())?;
    let dest = destination(&canonical, size, &layout)?;
    Ok(Job { source: canonical, destination: dest, size, layout })
}
fn ffmpeg_filter(job: &Job) -> String {
    let w = job.size.width - job.size.width % 2;
    let h = job.size.height - job.size.height % 2;
    match job.layout.mode.as_str() {
        "stretch" => format!("scale={w}:{h}:flags=lanczos,setsar=1"),
        "fit" => format!("scale={w}:{h}:force_original_aspect_ratio=decrease:flags=lanczos,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1"),
        _ => {
            let mut filter = format!("scale={w}:{h}:force_original_aspect_ratio=increase:flags=lanczos");
            if job.layout.zoom > 1.001 {
                // Bound extra zoom to 3x. Values are validated numbers, never QML expressions.
                filter.push_str(&format!(",scale=trunc(iw*{:.3}/2)*2:trunc(ih*{:.3}/2)*2:flags=lanczos", job.layout.zoom, job.layout.zoom));
            }
            filter.push_str(&format!(",crop={w}:{h}:(iw-{w})*{:.3}:(ih-{h})*{:.3},setsar=1", job.layout.focus_x, job.layout.focus_y));
            filter
        }
    }
}
fn convert(job: &Job, cancellable: bool) -> Result<(), String> {
    if ready(&job.destination) { return Ok(()); }
    if (job.size.width as f64) * (job.size.height as f64) * job.layout.zoom.powi(2) > 33_000_000.0 {
        return Err("Image zoom would require too much memory; reduce zoom".into());
    }
    let parent = job.destination.parent().ok_or("Invalid image cache")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(".{}-{}.tmp.jpg", job.destination.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("image")).to_string_lossy(), std::process::id()));
    // -update 1 prevents image2 sequence filename warnings on modern FFmpeg.
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y", "-threads", "2", "-i"])
        .arg(&job.source)
        .args(["-vf", &ffmpeg_filter(job), "-frames:v", "1", "-q:v", "3", "-update", "1", "-threads", "2"])
        .arg(&temporary).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    use std::os::unix::process::CommandExt;
    unsafe { cmd.pre_exec(|| {
        libc::setpriority(libc::PRIO_PROCESS, 0, 10);
        if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 { return Err(std::io::Error::last_os_error()); }
        Ok(())
    }); }
    let mut process = cmd.spawn().map_err(|e| format!("FFmpeg unavailable: {e}"))?;
    let result = loop {
        if cancellable && !wanted(&job.destination) {
            let _ = process.kill(); let _ = process.wait(); break Err("Cancelled".into());
        }
        match process.try_wait() {
            Ok(Some(status)) => break if status.success() { Ok(()) } else { Err(format!("FFmpeg image conversion failed ({status})")) },
            Ok(None) => thread::sleep(Duration::from_millis(150)),
            Err(e) => { let _ = process.kill(); let _ = process.wait(); break Err(e.to_string()); }
        }
    };
    if let Err(e) = result { let _ = fs::remove_file(&temporary); return Err(e); }
    if !ready(&temporary) { let _ = fs::remove_file(&temporary); return Err("FFmpeg did not create an image".into()); }
    // Avoid publishing a partially written file.
    fs::rename(&temporary, &job.destination).map_err(|e| { let _ = fs::remove_file(&temporary); e.to_string() })?;
    Ok(())
}

/// UI calls this repeatedly while showing a progress indicator.
pub fn inspect_image(path: &Path) -> Result<(u32, u32), String> {
    let output = Command::new("ffprobe").args(["-v", "error", "-select_streams", "v:0",
        "-show_entries", "stream=width,height", "-of", "json"])
        .arg(path).output().map_err(|e| format!("FFprobe unavailable: {e}"))?;
    if !output.status.success() { return Err("Could not inspect image dimensions".into()); }
    let data: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let w = data["streams"][0]["width"].as_u64().unwrap_or(0) as u32;
    let h = data["streams"][0]["height"].as_u64().unwrap_or(0) as u32;
    if w == 0 || h == 0 { return Err("Image dimensions not detected".into()); }
    Ok((w, h))
}

pub fn prepare(source: &Path, size: DisplaySize, layout: ImageLayout) -> Result<&'static str, String> {
    let job = job(source, size, layout)?;
    if ready(&job.destination) {
        if let Ok(mut s) = state().lock() { s.requested.remove(&job.destination); }
        return Ok("ready");
    }
    let enqueue = {
        let mut s = state().lock().map_err(|_| "Image optimizer unavailable")?;
        if let Some(error) = s.errors.get(&job.destination) { return Err(error.clone()); }
        s.requested.insert(job.destination.clone());
        s.pending.insert(job.destination.clone())
    };
    if enqueue && worker().try_send(job.clone()).is_err() {
        if let Ok(mut s) = state().lock() { s.pending.remove(&job.destination); s.requested.remove(&job.destination); }
        return Err("Image conversion queue is busy".into());
    }
    Ok("working")
}

pub fn prepared(source: &Path, size: DisplaySize, layout: ImageLayout) -> Result<PathBuf, String> {
    let job = job(source, size, layout)?;
    if ready(&job.destination) { return Ok(job.destination); }
    Err(state().lock().ok().and_then(|s| s.errors.get(&job.destination).cloned())
        .unwrap_or_else(|| "Image conversion is not ready".into()))
}

pub fn prepare_blocking(source: &Path, size: DisplaySize, layout: ImageLayout) -> Result<PathBuf, String> {
    let job = job(source, size, layout)?;
    convert(&job, false)?;
    Ok(job.destination)
}

pub fn cancel(source: &Path, size: DisplaySize, layout: ImageLayout) {
    if let Ok(job) = job(source, size, layout) {
        if let Ok(mut s) = state().lock() {
            s.requested.remove(&job.destination);
            s.errors.remove(&job.destination);
        }
    }
}

/// Do not evict an image currently in use by a desktop service.
pub fn protect_active(backend: &str, monitor: &str, cached: &Path) {
    if let Ok(mut s) = state().lock() {
        s.pinned.insert(format!("{backend}/{monitor}"), cached.to_path_buf());
    }
}
pub fn release_active(backend: &str, monitor: &str) {
    if let Ok(mut s) = state().lock() { s.pinned.remove(&format!("{backend}/{monitor}")); }
}
pub fn cancel_all() {
    if let Ok(mut s) = state().lock() { s.requested.clear(); }
}

fn prune_cache(recent: &Path) {
    const LIMIT: u64 = 768 * 1024 * 1024;
    let Some(parent) = recent.parent() else { return; };
    let Ok(entries) = fs::read_dir(parent) else { return; };
    let protected: HashSet<PathBuf> = state().lock().map(|s| s.pinned.values().cloned().chain(s.pending.iter().cloned()).collect()).unwrap_or_default();
    let mut images = Vec::new();
    let mut total = 0_u64;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jpg") || path.file_name().and_then(|f| f.to_str()).unwrap_or("").starts_with('.') { continue; }
        if let Ok(meta) = entry.metadata() {
            total += meta.len();
            images.push((path, meta.len(), meta.accessed().or_else(|_| meta.modified()).unwrap_or(SystemTime::UNIX_EPOCH)));
        }
    }
    images.sort_by_key(|(_,_,age)| *age);
    for (path, bytes, _) in images {
        if total <= LIMIT { break; }
        if path == recent || protected.contains(&path) { continue; }
        if fs::remove_file(&path).is_ok() { total = total.saturating_sub(bytes); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_layout_parameters_are_rejected() {
        assert!(ImageLayout::new("system", 0.5, 0.5, 1.0).is_err());
        assert!(ImageLayout::new("crop", f64::NAN, 0.5, 1.0).is_err());
        assert_eq!(ImageLayout::new("crop", -1.0, 2.0, 9.0).unwrap().zoom, 3.0);
    }
    #[test]
    fn safe_crop_filter_positions() {
        let job = Job { source: PathBuf::from("/input.png"), destination: PathBuf::from("/output.jpg"),
            size: DisplaySize { width:1920, height:1080, refresh:60 },
            layout: ImageLayout::new("crop", 1.0, 0.0, 2.0).unwrap() };
        let filter = ffmpeg_filter(&job);
        assert!(filter.contains("crop=1920:1080"));
        assert!(filter.contains("(iw-1920)*1.000"));
    }
}
