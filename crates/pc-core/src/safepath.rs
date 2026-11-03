//! Path-traversal protection.
//!
//! Every relative path that comes from outside (modpack indexes, zip entries, overrides, asset
//! indexes, runtime manifests, profile imports) must go through [`safe_join`] before it touches
//! the disk.

use std::path::{Component, Path, PathBuf};

use crate::{Error, Result};

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "CONIN$", "CONOUT$",
];

pub fn is_reserved_windows_name(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
    RESERVED.contains(&stem.as_str())
}

/// Validate a relative path (using `/` or `\` separators) and return its normalised segments.
pub fn validate_relative(rel: &str) -> Result<Vec<String>> {
    let bad = |why: &str| Err(Error::UnsafePath(format!("{rel:?}: {why}")));
    if rel.is_empty() {
        return bad("empty path");
    }
    if rel.contains('\0') {
        return bad("NUL byte");
    }
    if rel.starts_with('/') || rel.starts_with('\\') {
        return bad("absolute path");
    }
    let normalised = rel.replace('\\', "/");
    let mut out = Vec::new();
    for seg in normalised.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return bad("parent directory reference");
        }
        if seg.contains(':') {
            return bad("drive letter or alternate data stream");
        }
        if seg.chars().any(|c| matches!(c, '<' | '>' | '"' | '|' | '?' | '*') || c.is_control()) {
            return bad("invalid character");
        }
        if seg.ends_with('.') || seg.ends_with(' ') {
            return bad("segment ends with dot or space");
        }
        if is_reserved_windows_name(seg) {
            return bad("reserved Windows device name");
        }
        out.push(seg.to_string());
    }
    if out.is_empty() {
        return bad("no path segments");
    }
    // Belt and braces: make sure std agrees there are only normal components.
    let p: PathBuf = out.iter().collect();
    if p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return bad("non-normal component");
    }
    Ok(out)
}

/// Join `rel` onto `base`, guaranteeing the result stays inside `base`.
pub fn safe_join(base: &Path, rel: &str) -> Result<PathBuf> {
    let segs = validate_relative(rel)?;
    let mut p = base.to_path_buf();
    for s in segs {
        p.push(s);
    }
    if !p.starts_with(base) {
        return Err(Error::UnsafePath(rel.to_string()));
    }
    // If parts of the path already exist, make sure no junction/symlink redirects it outside.
    if let (Some(real_base), Some(existing)) = (crate::paths::dunce_canonicalize(base), nearest_existing(&p)) {
        if let Some(real) = crate::paths::dunce_canonicalize(&existing) {
            if !real.starts_with(&real_base) {
                return Err(Error::UnsafePath(format!("{rel} resolves outside the target directory")));
            }
        }
    }
    Ok(p)
}

fn nearest_existing(p: &Path) -> Option<PathBuf> {
    let mut cur = p.to_path_buf();
    loop {
        if cur.exists() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

/// Only these hosts may be used for files listed in a Modrinth pack index (per the .mrpack spec).
pub fn is_allowed_mrpack_host(url: &str) -> bool {
    let Ok(u) = url::Url::parse(url) else { return false };
    // Unit tests serve pack files from a loopback mock server; release builds never allow it.
    if cfg!(test) && u.scheme() == "http" && u.host_str() == Some("127.0.0.1") {
        return true;
    }
    if u.scheme() != "https" {
        return false;
    }
    matches!(
        u.host_str(),
        Some("cdn.modrinth.com") | Some("github.com") | Some("raw.githubusercontent.com") | Some("gitlab.com")
    )
}

/// HTTPS everywhere, with plain HTTP only for loopback (local test servers).
pub fn is_allowed_download_url(url: &str) -> bool {
    let Ok(u) = url::Url::parse(url) else { return false };
    match u.scheme() {
        "https" => true,
        "http" => matches!(u.host_str(), Some("127.0.0.1") | Some("localhost") | Some("[::1]")),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_absolute() {
        let base = Path::new("C:/base");
        for bad in [
            "../evil.txt",
            "mods/../../evil.txt",
            "..\\evil",
            "/etc/passwd",
            "\\windows\\system32",
            "C:/Windows/evil.dll",
            "C:evil",
            "mods/file.jar:stream",
            "mods/CON",
            "mods/nul.txt",
            "config/trailing.",
            "a/b?/c",
            "",
            "./.",
        ] {
            assert!(safe_join(base, bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn accepts_normal_paths() {
        let dir = tempfile::tempdir().unwrap();
        let p = safe_join(dir.path(), "config/sodium-options.json").unwrap();
        assert!(p.starts_with(dir.path()));
        let p = safe_join(dir.path(), "mods\\./x.jar").unwrap();
        assert_eq!(p, dir.path().join("mods").join("x.jar"));
    }

    #[test]
    fn mrpack_hosts() {
        assert!(is_allowed_mrpack_host("https://cdn.modrinth.com/data/x/y.jar"));
        assert!(!is_allowed_mrpack_host("http://cdn.modrinth.com/data/x/y.jar"));
        assert!(!is_allowed_mrpack_host("https://evil.example/x.jar"));
        assert!(is_allowed_download_url("http://127.0.0.1:1234/x"));
        assert!(!is_allowed_download_url("http://example.com/x"));
        assert!(!is_allowed_download_url("file:///C:/x"));
    }
}
