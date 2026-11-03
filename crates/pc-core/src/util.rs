use std::io::Read;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;
use sha1::Digest;

use crate::{Error, Result};

/// Write `bytes` atomically: write to a sibling temp file, flush, then rename over the target.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp-{}",
        path.extension().and_then(|e| e.to_str()).unwrap_or(""),
        std::process::id()
    ));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        Error::Io(e)
    })
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    atomic_write(path, &bytes)
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Read JSON, returning `None` if the file does not exist.
pub fn read_json_opt<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileHashes {
    pub sha1: String,
    pub sha512: String,
    pub size: u64,
}

pub fn hash_file(path: &Path) -> Result<FileHashes> {
    let mut f = std::fs::File::open(path)?;
    let mut s1 = sha1::Sha1::new();
    let mut s512 = sha2::Sha512::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        s1.update(&buf[..n]);
        s512.update(&buf[..n]);
    }
    Ok(FileHashes { sha1: hex::encode(s1.finalize()), sha512: hex::encode(s512.finalize()), size })
}

pub fn sha1_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut s1 = sha1::Sha1::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        s1.update(&buf[..n]);
    }
    Ok(hex::encode(s1.finalize()))
}

pub fn sha1_hex(data: &[u8]) -> String {
    hex::encode(sha1::Sha1::digest(data))
}

/// Verify a file against whichever hashes are known. Returns Ok(true) if it matches, Ok(false)
/// if there was nothing to compare (no hashes and no size), Err on mismatch.
pub fn verify_file(path: &Path, sha1: Option<&str>, sha512: Option<&str>, size: Option<u64>) -> Result<bool> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if sha512.is_some() {
        let h = hash_file(path)?;
        if let Some(s) = sha512 {
            if !h.sha512.eq_ignore_ascii_case(s) {
                return Err(Error::HashMismatch { file: name, expected: s.into(), actual: h.sha512 });
            }
        }
        if let Some(s) = sha1 {
            if !h.sha1.eq_ignore_ascii_case(s) {
                return Err(Error::HashMismatch { file: name, expected: s.into(), actual: h.sha1 });
            }
        }
        return Ok(true);
    }
    if let Some(s) = sha1 {
        let actual = sha1_file(path)?;
        if !actual.eq_ignore_ascii_case(s) {
            return Err(Error::HashMismatch { file: name, expected: s.into(), actual });
        }
        return Ok(true);
    }
    if let Some(sz) = size {
        let actual = std::fs::metadata(path)?.len();
        if actual != sz {
            return Err(Error::HashMismatch { file: name, expected: format!("{sz} bytes"), actual: format!("{actual} bytes") });
        }
        return Ok(true);
    }
    Ok(false)
}

/// Quick check used to skip re-downloading files that already verify.
pub fn file_matches(path: &Path, sha1: Option<&str>, sha512: Option<&str>, size: Option<u64>) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if !meta.is_file() {
        return false;
    }
    if let Some(sz) = size {
        if meta.len() != sz {
            return false;
        }
    }
    if sha1.is_none() && sha512.is_none() {
        // Without a hash we only trust a size match; with no size either we must re-download.
        return size.is_some();
    }
    matches!(verify_file(path, sha1, sha512, size), Ok(true))
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn timestamp_slug() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

/// Make a string safe to use as a single file/folder name.
pub fn sanitize_file_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    out = out.trim().trim_end_matches('.').to_string();
    if out.is_empty() {
        out = "unnamed".into();
    }
    if crate::safepath::is_reserved_windows_name(&out) {
        out.insert(0, '_');
    }
    if out.len() > 80 {
        out.truncate(80);
    }
    out
}

/// Free bytes on the volume containing `path` (walks up to the nearest existing ancestor).
pub fn free_space(path: &Path) -> Option<u64> {
    let mut p = path.to_path_buf();
    while !p.exists() {
        if !p.pop() {
            return None;
        }
    }
    free_space_impl(&p)
}

#[cfg(windows)]
fn free_space_impl(p: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut avail: u64 = 0;
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, std::ptr::null_mut(), std::ptr::null_mut()) };
    if ok != 0 { Some(avail) } else { None }
}

#[cfg(not(windows))]
fn free_space_impl(_p: &Path) -> Option<u64> {
    None
}

/// Copy a directory tree. Refuses to follow symlinks out of the tree.
pub fn copy_dir_recursive(src: &Path, dst: &Path, skip: &dyn Fn(&Path) -> bool) -> Result<u64> {
    let mut copied = 0;
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if skip(&path) {
            continue;
        }
        let ft = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ft.is_dir() {
            copied += copy_dir_recursive(&path, &target, skip)?;
        } else if ft.is_file() {
            std::fs::copy(&path, &target)?;
            copied += 1;
        }
    }
    Ok(copied)
}

pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            if let Ok(ft) = e.file_type() {
                if ft.is_dir() {
                    total += dir_size(&e.path());
                } else if let Ok(m) = e.metadata() {
                    total += m.len();
                }
            }
        }
    }
    total
}

/// Move a file or directory, falling back to copy+delete across volumes.
pub fn move_path(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) if src.is_dir() => {
            copy_dir_recursive(src, dst, &|_| false)?;
            std::fs::remove_dir_all(src)?;
            Ok(())
        }
        Err(_) => {
            std::fs::copy(src, dst)?;
            std::fs::remove_file(src)?;
            Ok(())
        }
    }
}

/// Pick a path that does not exist yet by appending ` (n)` before the extension.
pub fn unique_path(path: &Path) -> std::path::PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    for i in 1..10_000 {
        let candidate = parent.join(format!("{stem} ({i}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}-{}{ext}", uuid::Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_names() {
        assert_eq!(sanitize_file_name("a/b:c"), "a_b_c");
        assert_eq!(sanitize_file_name("CON"), "_CON");
        assert_eq!(sanitize_file_name("  ...  "), "unnamed");
        assert_eq!(sanitize_file_name("World."), "World");
    }

    #[test]
    fn verify_detects_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.bin");
        std::fs::write(&p, b"hello").unwrap();
        let good = sha1_hex(b"hello");
        assert!(verify_file(&p, Some(&good), None, Some(5)).unwrap());
        assert!(matches!(verify_file(&p, Some("00"), None, None), Err(Error::HashMismatch { .. })));
        assert!(file_matches(&p, Some(&good), None, None));
        assert!(!file_matches(&p, None, None, None));
    }

    #[test]
    fn atomic_write_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.json");
        atomic_write(&p, b"1").unwrap();
        atomic_write(&p, b"2").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"2");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "no temp files left behind");
    }
}
