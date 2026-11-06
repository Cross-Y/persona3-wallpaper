//! Provider-backed wallpaper catalog. No HTML scraping: each source uses its documented API.
use serde_json::{json, Value};
use std::{fs::{File, OpenOptions}, io::{copy, Read, Write}, path::{Path, PathBuf}, sync::OnceLock, time::Duration};
use p3w_platform::linux::paths;

const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
// Reuse the HTTP connection pool across catalog searches and downloads.
static CLIENT: OnceLock<Result<reqwest::blocking::Client, String>> = OnceLock::new();
fn client() -> Result<&'static reqwest::blocking::Client, String> {
    CLIENT.get_or_init(|| reqwest::blocking::Client::builder()
        .user_agent("Persona3Wallpaper/0.1 (desktop wallpaper client)")
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(35))
        .build().map_err(|e| e.to_string()))
        .as_ref().map_err(Clone::clone)
}
fn library_dir() -> Result<PathBuf, String> {
    let base = paths::data_home().ok_or("Could not locate XDG_DATA_HOME / HOME")?;
    let dir = base.join("persona3-wallpaper/wallpapers");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
fn safe_name(input: &str, fallback: &str) -> String {
    let name = Path::new(input).file_name().and_then(|s| s.to_str()).unwrap_or(fallback);
    let cleaned: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c=='.' || *c=='-' || *c=='_' || *c==' ').collect();
    if cleaned.is_empty() { fallback.into() } else { cleaned }
}
pub fn search(provider: &str, query: &str, page: u32) -> Result<Value, String> {
    let q = query.trim();
    let page = page.max(1);
    let http = client()?;
    match provider {
        "wallhaven" => {
            // Empty query loads the public front-page feed (latest wallpapers).
            let page_string = page.to_string();
            let mut params = vec![("sorting", if q.is_empty() { "date_added" } else { "relevance" }), ("purity", "100"), ("page", page_string.as_str())];
            if !q.is_empty() { params.push(("q", q)); }
            let data: Value = http.get("https://wallhaven.cc/api/v1/search")
                .query(&params)
                .send().map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?
                .json().map_err(|e| e.to_string())?;
            let items = data["data"].as_array().cloned().unwrap_or_default().into_iter().filter_map(|v| {
                let id=v["id"].as_str()?; let download=v["path"].as_str()?; let preview=v["thumbs"]["large"].as_str().or(v["thumbs"]["original"].as_str())?;
                let resolution = v["resolution"].as_str().unwrap_or("Unknown");
                Some(json!({"id":id,"title":format!("Wallpaper {id}"),"preview":preview,"download":download,"source_url":v["url"],"provider":"Wallhaven","resolution":resolution,"kind":"image","filename":format!("wallhaven-{id}.jpg"),"download_options":[{"label":format!("Original · {resolution}"),"resolution":resolution,"url":download,"filename":format!("wallhaven-{id}.jpg")}]}))
            }).collect::<Vec<_>>();
            let meta = &data["meta"];
            let current_page = meta["current_page"].as_u64().unwrap_or(page as u64);
            let last_page = meta["last_page"].as_u64().unwrap_or(page as u64);
            let total = meta["total"].as_u64().unwrap_or(items.len() as u64);
            Ok(json!({"items":items,"page":current_page,"last_page":last_page,"total":total}))
        }
        "pixabay" => {
            let key=std::env::var("PIXABAY_API_KEY").map_err(|_| "Set PIXABAY_API_KEY to enable Pixabay.".to_string())?;
            let data: Value = http.get("https://pixabay.com/api/videos/").query(&[("key",key.as_str()),("q",q),("safesearch","true"),("per_page","30")])
                .send().map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())?;
            let items=data["hits"].as_array().cloned().unwrap_or_default().into_iter().filter_map(|v| {
                let id=v["id"].as_u64()?; let rendition=v["videos"]["medium"].as_object().or(v["videos"]["small"].as_object())?;
                let download=rendition.get("url")?.as_str()?; let preview=rendition.get("thumbnail")?.as_str()?;
                Some(json!({"id":id.to_string(),"title":v["tags"],"preview":preview,"download":download,"source_url":v["pageURL"],"provider":"Pixabay","resolution":format!("{}x{}",rendition.get("width").and_then(Value::as_u64).unwrap_or(0),rendition.get("height").and_then(Value::as_u64).unwrap_or(0)),"kind":"video","filename":format!("pixabay-{id}.mp4")}))
            }).collect::<Vec<_>>(); Ok(json!(items))
        }
        "desktophut" => Err("DesktopHut does not provide a documented public API for this feature. Integration requires official authorization; scraping is not performed.".into()),
        _ => Err("Unknown wallpaper provider.".into()),
    }
}
fn create_download_file(dir: &Path, name: &str) -> Result<(PathBuf, File), String> {
    let source = Path::new(name);
    let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("wallpaper");
    let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("");
    for number in 0..10_000 {
        let candidate = if number == 0 { name.to_string() }
            else if ext.is_empty() { format!("{stem}-{number}") }
            else { format!("{stem}-{number}.{ext}") };
        let path = dir.join(candidate);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Could not create wallpaper file: {error}")),
        }
    }
    Err("Too many wallpapers already have this filename".into())
}

pub fn download_into_library(url: &str, filename: &str) -> Result<String, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https" { return Err("Only HTTPS downloads are allowed.".into()); }
    let host = parsed.host_str().unwrap_or("");
    if !["wallhaven.cc", "w.wallhaven.cc", "pixabay.com", "cdn.pixabay.com"]
        .iter().any(|h| host == *h || host.ends_with(&format!(".{h}"))) {
        return Err("Download domain is not allowed for these providers.".into());
    }

    let name = safe_name(filename, "wallpaper.bin");
    let ext = Path::new(&name).extension().and_then(|s| s.to_str())
        .unwrap_or("").to_ascii_lowercase();
    if !["mp4", "webm", "mkv", "mov", "jpg", "jpeg", "png", "webp"]
        .contains(&ext.as_str()) { return Err("Unsupported downloaded file format.".into()); }

    let mut response = client()?.get(parsed).send().map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?;
    if response.content_length().unwrap_or(0) > MAX_DOWNLOAD_BYTES {
        return Err("The file exceeds the 512 MiB limit.".into());
    }

    let dir = library_dir()?;
    let (dest, mut file) = create_download_file(&dir, &name)?;
    let result = (|| -> Result<(), String> {
        let copied = copy(&mut response.take(MAX_DOWNLOAD_BYTES + 1), &mut file)
            .map_err(|e| format!("Download interrupted: {e}"))?;
        if copied > MAX_DOWNLOAD_BYTES {
            return Err("Download canceled: file exceeds 512 MiB.".into());
        }
        file.flush().map_err(|e| format!("Could not save download: {e}"))?;
        Ok(())
    })();
    drop(file);
    if let Err(error) = result {
        let _ = std::fs::remove_file(&dest);
        return Err(error);
    }
    Ok(dest.to_string_lossy().to_string())
}
