//! Lightweight file version metadata for idempotent wallpaper application.
use std::{fs, path::Path, time::SystemTime};

pub(crate) fn file_identity(path: &Path) -> Option<(u64, SystemTime)> {
    let metadata = fs::metadata(path).ok()?;
    Some((metadata.len(), metadata.modified().ok()?))
}
