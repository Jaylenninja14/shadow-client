//! Data-root resolution.
//!
//! * `SHADOW_DATA_DIR` env var (tests, power users) wins (`SUNNY_DATA_DIR`, the old name, too).
//! * Portable mode: a file named `portable.txt` next to the executable ⇒ `<exe dir>\data`.
//! * Otherwise `%APPDATA%\ShadowClient`. Data from the client's old name (`%APPDATA%\SunnyClient`)
//!   is moved there the first time, or used where it is if it can't be moved.
//!
//! The root deliberately differs from Tauri's bundle-identifier directory so the uninstaller's
//! "delete app data" option can never remove instances or worlds.

use std::path::{Path, PathBuf};

pub const DATA_DIR_NAME: &str = "ShadowClient";
/// Data folders from the client's earlier names, newest first.
const LEGACY_DIR_NAMES: &[&str] = &["SunnyClient"];

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub portable: bool,
}

impl AppPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into(), portable: false }
    }

    /// Resolve the data root for the running executable.
    pub fn resolve() -> Self {
        for var in ["SHADOW_DATA_DIR", "SUNNY_DATA_DIR"] {
            if let Ok(dir) = std::env::var(var) {
                if !dir.trim().is_empty() {
                    return Self::new(dir);
                }
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                if dir.join("portable.txt").exists() {
                    return Self { root: dir.join("data"), portable: true };
                }
            }
        }
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        Self::new(migrate_legacy(&base))
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for d in [
            self.root.clone(),
            self.profiles_dir(),
            self.instances_dir(),
            self.libraries_dir(),
            self.assets_dir(),
            self.versions_dir(),
            self.runtimes_dir(),
            self.http_cache_dir(),
            self.tmp_dir(),
            self.logs_dir(),
            self.natives_root(),
        ] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }

    pub fn settings_file(&self) -> PathBuf { self.root.join("settings.json") }
    pub fn state_file(&self) -> PathBuf { self.root.join("state.json") }
    pub fn secrets_file(&self) -> PathBuf { self.root.join("secrets.dat") }
    pub fn downloads_history_file(&self) -> PathBuf { self.root.join("downloads-history.json") }
    pub fn profiles_dir(&self) -> PathBuf { self.root.join("profiles") }
    pub fn instances_dir(&self) -> PathBuf { self.root.join("instances") }
    pub fn libraries_dir(&self) -> PathBuf { self.root.join("libraries") }
    pub fn assets_dir(&self) -> PathBuf { self.root.join("assets") }
    pub fn versions_dir(&self) -> PathBuf { self.root.join("versions") }
    pub fn runtimes_dir(&self) -> PathBuf { self.root.join("runtime") }
    pub fn cache_dir(&self) -> PathBuf { self.root.join("cache") }
    pub fn http_cache_dir(&self) -> PathBuf { self.cache_dir().join("http") }
    pub fn tmp_dir(&self) -> PathBuf { self.root.join("tmp") }
    pub fn logs_dir(&self) -> PathBuf { self.root.join("logs") }
    pub fn natives_root(&self) -> PathBuf { self.root.join("natives") }
    pub fn preserved_worlds_dir(&self) -> PathBuf { self.root.join("preserved-worlds") }
    pub fn backups_dir(&self) -> PathBuf { self.root.join("backups") }
    pub fn exports_dir(&self) -> PathBuf { self.root.join("exports") }

    pub fn version_dir(&self, id: &str) -> PathBuf { self.versions_dir().join(id) }
    pub fn version_json(&self, id: &str) -> PathBuf { self.version_dir(id).join(format!("{id}.json")) }
    pub fn version_jar(&self, id: &str) -> PathBuf { self.version_dir(id).join(format!("{id}.jar")) }
    pub fn instance_dir(&self, profile_id: &str) -> PathBuf { self.instances_dir().join(profile_id) }
    /// Launcher bookkeeping for a profile (mod index, backups). Kept outside the game folder so
    /// external installations never receive launcher files.
    pub fn profile_data_dir(&self, profile_id: &str) -> PathBuf { self.root.join("profile-data").join(profile_id) }

    /// True if `p` is located inside the managed instances directory.
    pub fn is_managed_instance_path(&self, p: &Path) -> bool {
        let root = canonical_or_self(&self.instances_dir());
        let p = canonical_or_self(p);
        p.starts_with(&root) && p != root
    }
}

pub fn canonical_or_self(p: &Path) -> PathBuf {
    dunce_canonicalize(p).unwrap_or_else(|| p.to_path_buf())
}

/// `std::fs::canonicalize` on Windows returns `\\?\` paths; strip that prefix for display and
/// comparison with user-provided paths.
pub fn dunce_canonicalize(p: &Path) -> Option<PathBuf> {
    let c = std::fs::canonicalize(p).ok()?;
    let s = c.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return Some(PathBuf::from(format!(r"\\{rest}")));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return Some(PathBuf::from(rest));
    }
    Some(c)
}

/// The official launcher's default game directory.
pub fn default_minecraft_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join(".minecraft")
}

/// `<base>\ShadowClient`, moving the data folder of an older name there the first time.
fn migrate_legacy(base: &Path) -> PathBuf {
    let dir = base.join(DATA_DIR_NAME);
    if dir.exists() {
        return dir;
    }
    for old in LEGACY_DIR_NAMES.iter().map(|n| base.join(n)) {
        if old.is_dir() {
            // A rename on the same drive is instant and keeps everything; if it fails (a file
            // in use), keep using the old folder rather than starting empty.
            return match std::fs::rename(&old, &dir) {
                Ok(()) => dir,
                Err(_) => old,
            };
        }
    }
    dir
}
