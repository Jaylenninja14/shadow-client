//! Per-profile record of what the launcher installed and where it came from. Stored in the
//! profile's launcher data folder, never inside the game folder.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ContentKind;
use crate::{util, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct IndexEntry {
    pub file_name: String,
    pub kind: ContentKind,
    pub sha1: String,
    pub sha512: Option<String>,
    /// "modrinth" | "curseforge" | "builtin" | "manual" | "modpack"
    pub source: String,
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub version_number: Option<String>,
    pub icon_url: Option<String>,
    pub website_url: Option<String>,
    /// Installed because the user asked (false = pulled in as a dependency)
    pub explicit: bool,
    pub builtin: bool,
    pub installed_at: String,
    pub required_by: Vec<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

impl Default for IndexEntry {
    fn default() -> Self {
        Self {
            file_name: String::new(),
            kind: ContentKind::Mod,
            sha1: String::new(),
            sha512: None,
            source: "manual".into(),
            project_id: None,
            version_id: None,
            slug: None,
            title: None,
            version_number: None,
            icon_url: None,
            website_url: None,
            explicit: true,
            builtin: false,
            installed_at: String::new(),
            required_by: vec![],
            game_versions: vec![],
            loaders: vec![],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModIndex {
    pub entries: Vec<IndexEntry>,
    /// file name → (size, mtime secs, sha1) to avoid re-hashing unchanged files
    pub hash_cache: HashMap<String, (u64, u64, String)>,
    /// sha1 → Modrinth identification result (None = unknown to Modrinth)
    pub identified: HashMap<String, Option<IdentifiedProject>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IdentifiedProject {
    pub project_id: String,
    pub version_id: String,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub icon_url: Option<String>,
    pub version_number: String,
}

pub fn index_path(profile_data: &Path) -> PathBuf {
    profile_data.join("mods-index.json")
}

impl ModIndex {
    pub fn load(profile_data: &Path) -> ModIndex {
        util::read_json_opt(&index_path(profile_data)).ok().flatten().unwrap_or_default()
    }

    pub fn save(&self, profile_data: &Path) -> Result<()> {
        util::write_json(&index_path(profile_data), self)
    }

    fn base_name(name: &str) -> &str {
        name.strip_suffix(".disabled").unwrap_or(name)
    }

    pub fn by_file(&self, kind: ContentKind, file_name: &str) -> Option<&IndexEntry> {
        let base = Self::base_name(file_name);
        self.entries.iter().find(|e| e.kind == kind && Self::base_name(&e.file_name) == base)
    }

    pub fn by_project(&self, kind: ContentKind, project_id: &str) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.kind == kind && e.project_id.as_deref() == Some(project_id))
    }

    pub fn upsert(&mut self, entry: IndexEntry) {
        let base = Self::base_name(&entry.file_name).to_string();
        self.entries.retain(|e| !(e.kind == entry.kind && Self::base_name(&e.file_name) == base));
        self.entries.push(entry);
    }

    pub fn remove_file(&mut self, kind: ContentKind, file_name: &str) -> Option<IndexEntry> {
        let base = Self::base_name(file_name).to_string();
        let pos = self.entries.iter().position(|e| e.kind == kind && Self::base_name(&e.file_name) == base)?;
        Some(self.entries.remove(pos))
    }

    pub fn rename_file(&mut self, kind: ContentKind, from: &str, to: &str) {
        let base = Self::base_name(from).to_string();
        for e in self.entries.iter_mut().filter(|e| e.kind == kind && Self::base_name(&e.file_name) == base) {
            e.file_name = to.to_string();
        }
    }

    /// SHA-1 of a file, using the cache when size and mtime are unchanged.
    pub fn cached_sha1(&mut self, path: &Path) -> Result<String> {
        let meta = std::fs::metadata(path)?;
        let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some((size, mt, sha)) = self.hash_cache.get(&name) {
            if *size == meta.len() && *mt == mtime {
                return Ok(sha.clone());
            }
        }
        let sha = util::sha1_file(path)?;
        self.hash_cache.insert(name, (meta.len(), mtime, sha.clone()));
        Ok(sha)
    }
}
