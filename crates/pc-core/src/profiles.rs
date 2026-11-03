//! Profiles: independent game setups with their own version, loader, mods, settings and
//! game directory.
//!
//! Safety rules:
//! * A profile that points at an existing/external folder (`external = true`, or a game dir
//!   outside our `instances` folder) never has files deleted by the launcher.
//! * Deleting a managed profile moves `saves/` (worlds) and `screenshots/` into
//!   `preserved-worlds/` and sends the rest of the folder to the Windows Recycle Bin.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::minecraft::loaders::LoaderKind;
use crate::paths::AppPaths;
use crate::{util, Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileKind {
    Vanilla,
    Performance,
    Pvp,
    Survival,
    Modded,
    Custom,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRecord {
    pub at: String,
    /// "running" | "ok" | "crashed" | "failed"
    pub result: String,
    pub message: Option<String>,
    pub duration_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub kind: ProfileKind,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
    /// For external installs: the exact version id in that folder (e.g. a Forge version id).
    pub external_version_id: Option<String>,
    /// None ⇒ `instances/<id>`
    pub game_dir: Option<PathBuf>,
    /// Root of the external `.minecraft` this profile came from (libraries/versions lookup).
    pub external_root: Option<PathBuf>,
    pub external: bool,
    pub java_path: Option<String>,
    pub memory_min_mb: Option<u32>,
    pub memory_max_mb: Option<u32>,
    pub jvm_args: Option<String>,
    pub resolution_width: Option<u32>,
    pub resolution_height: Option<u32>,
    pub fullscreen: Option<bool>,
    /// "low" | "balanced" | "high" | "custom" | None (leave options.txt alone)
    pub preset: Option<String>,
    pub builtin_mods: bool,
    pub icon: Option<String>,
    pub created_at: String,
    pub last_played: Option<String>,
    pub last_launch: Option<LaunchRecord>,
    pub total_play_secs: u64,
    pub notes: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            kind: ProfileKind::Custom,
            mc_version: String::new(),
            loader: LoaderKind::Vanilla,
            loader_version: None,
            external_version_id: None,
            game_dir: None,
            external_root: None,
            external: false,
            java_path: None,
            memory_min_mb: None,
            memory_max_mb: None,
            jvm_args: None,
            resolution_width: None,
            resolution_height: None,
            fullscreen: None,
            preset: None,
            builtin_mods: false,
            icon: None,
            created_at: String::new(),
            last_played: None,
            last_launch: None,
            total_play_secs: 0,
            notes: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProfile {
    pub name: String,
    pub kind: ProfileKind,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
    pub game_dir: Option<PathBuf>,
    pub preset: Option<String>,
    pub builtin_mods: Option<bool>,
    /// Add the Lunar-style in-game feature pack (Fabric/Quilt only)
    pub ingame_features: Option<bool>,
}

pub struct Profiles<'a> {
    paths: &'a AppPaths,
    /// Send deleted instance folders to the Recycle Bin (tests park them instead).
    use_recycle_bin: bool,
}

fn validate_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 48 {
        return Err(Error::Invalid("Profile names must be 1–48 characters".into()));
    }
    if n.chars().any(|c| c.is_control()) {
        return Err(Error::Invalid("Profile names cannot contain control characters".into()));
    }
    Ok(n.to_string())
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(Error::Invalid(format!("invalid profile id {id:?}")));
    }
    Ok(())
}

impl<'a> Profiles<'a> {
    pub fn new(paths: &'a AppPaths) -> Self {
        Self { paths, use_recycle_bin: true }
    }

    pub fn without_recycle_bin(paths: &'a AppPaths) -> Self {
        Self { paths, use_recycle_bin: false }
    }

    fn file(&self, id: &str) -> Result<PathBuf> {
        validate_id(id)?;
        Ok(self.paths.profiles_dir().join(format!("{id}.json")))
    }

    pub fn list(&self) -> Vec<Profile> {
        let mut out: Vec<Profile> = std::fs::read_dir(self.paths.profiles_dir())
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                    .filter_map(|e| match util::read_json::<Profile>(&e.path()) {
                        Ok(p) => Some(p),
                        Err(err) => {
                            tracing::warn!("skipping unreadable profile {}: {err}", e.path().display());
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| b.last_played.cmp(&a.last_played).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        out
    }

    pub fn get(&self, id: &str) -> Result<Profile> {
        let f = self.file(id)?;
        util::read_json_opt(&f)?.ok_or_else(|| Error::NotFound(format!("profile {id}")))
    }

    pub fn save(&self, p: &Profile) -> Result<()> {
        validate_name(&p.name)?;
        util::write_json(&self.file(&p.id)?, p)
    }

    pub fn game_dir(&self, p: &Profile) -> PathBuf {
        p.game_dir.clone().unwrap_or_else(|| self.paths.instance_dir(&p.id))
    }

    /// Whether the launcher manages (and may clean up) this profile's folder.
    pub fn is_managed(&self, p: &Profile) -> bool {
        !p.external && self.paths.is_managed_instance_path(&self.game_dir(p))
    }

    pub fn create(&self, req: NewProfile) -> Result<Profile> {
        let name = validate_name(&req.name)?;
        if req.mc_version.trim().is_empty() {
            return Err(Error::Invalid("Choose a Minecraft version".into()));
        }
        let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
        let external = req.game_dir.as_ref().is_some_and(|d| !self.paths.is_managed_instance_path(d));
        let p = Profile {
            id: id.clone(),
            name,
            kind: req.kind,
            mc_version: req.mc_version.trim().to_string(),
            loader: req.loader,
            loader_version: req.loader_version.filter(|v| !v.is_empty()),
            game_dir: req.game_dir,
            external,
            preset: req.preset,
            builtin_mods: req.builtin_mods.unwrap_or(false),
            created_at: util::now_iso(),
            ..Default::default()
        };
        let dir = self.game_dir(&p);
        std::fs::create_dir_all(&dir)?;
        if !external {
            for sub in ["mods", "config", "resourcepacks", "shaderpacks", "saves"] {
                std::fs::create_dir_all(dir.join(sub))?;
            }
        }
        self.save(&p)?;
        Ok(p)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<Profile> {
        let mut p = self.get(id)?;
        p.name = validate_name(name)?;
        self.save(&p)?;
        Ok(p)
    }

    /// Duplicate a profile into a new managed instance. Worlds are only copied when asked.
    pub fn duplicate(&self, id: &str, new_name: &str, include_worlds: bool) -> Result<Profile> {
        let src = self.get(id)?;
        let mut copy = src.clone();
        copy.id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
        copy.name = validate_name(new_name)?;
        copy.game_dir = None;
        copy.external = false;
        copy.kind = if src.kind == ProfileKind::External { ProfileKind::Custom } else { src.kind };
        copy.created_at = util::now_iso();
        copy.last_played = None;
        copy.last_launch = None;
        copy.total_play_secs = 0;
        let from = self.game_dir(&src);
        let to = self.game_dir(&copy);
        std::fs::create_dir_all(&to)?;
        if from.exists() {
            let skip = |p: &Path| {
                let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                let is_top = p.parent() == Some(from.as_path());
                is_top && (matches!(name.as_str(), "logs" | "crash-reports" | "screenshots" | ".shadowclient" | ".sunnyclient") || (!include_worlds && name == "saves"))
                    // never copy the official launcher's own files out of a .minecraft folder
                    || (is_top && (name.starts_with("launcher_") || matches!(name.as_str(), "versions" | "libraries" | "assets" | "runtime" | "webcache2")))
            };
            util::copy_dir_recursive(&from, &to, &skip)?;
        }
        std::fs::create_dir_all(to.join("saves"))?;
        self.save(&copy)?;
        Ok(copy)
    }

    /// Delete a profile. Returns a human-readable summary of what happened to its files.
    pub fn delete(&self, id: &str) -> Result<DeleteReport> {
        let p = self.get(id)?;
        let dir = self.game_dir(&p);
        let mut report = DeleteReport { profile_removed: true, ..Default::default() };
        if !self.is_managed(&p) {
            report.message = format!("Removed the profile only. Files in {} were not touched.", dir.display());
            std::fs::remove_file(self.file(id)?)?;
            return Ok(report);
        }
        if dir.exists() {
            let keep_root = self.paths.preserved_worlds_dir().join(format!("{}-{}", util::sanitize_file_name(&p.name), util::timestamp_slug()));
            for sub in ["saves", "screenshots"] {
                let src = dir.join(sub);
                let has_content = std::fs::read_dir(&src).map(|mut rd| rd.next().is_some()).unwrap_or(false);
                if has_content {
                    util::move_path(&src, &keep_root.join(sub))?;
                    report.preserved.push(keep_root.join(sub));
                }
            }
            let recycled = if self.use_recycle_bin { trash::delete(&dir).map_err(|e| e.to_string()) } else { Err("recycle bin disabled".to_string()) };
            match recycled {
                Ok(()) => report.recycled = Some(dir.clone()),
                Err(e) => {
                    // Could not reach the Recycle Bin: keep the folder rather than deleting it.
                    let parked = self.paths.backups_dir().join("deleted-profiles").join(format!("{}-{}", p.id, util::timestamp_slug()));
                    util::move_path(&dir, &parked)?;
                    if self.use_recycle_bin {
                        tracing::warn!("recycle bin unavailable ({e}); moved instance to {}", parked.display());
                    }
                    report.parked = Some(parked);
                }
            }
        }
        std::fs::remove_file(self.file(id)?)?;
        report.message = match (&report.preserved.is_empty(), &report.recycled) {
            (false, _) => format!("Profile deleted. Worlds/screenshots were kept in {}.", report.preserved[0].parent().unwrap().display()),
            (true, Some(_)) => "Profile deleted; its folder is in the Recycle Bin.".into(),
            (true, None) => "Profile deleted.".into(),
        };
        Ok(report)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReport {
    pub profile_removed: bool,
    pub preserved: Vec<PathBuf>,
    pub recycled: Option<PathBuf>,
    pub parked: Option<PathBuf>,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, AppPaths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().join("data"));
        paths.ensure().unwrap();
        (dir, paths)
    }

    fn req(name: &str) -> NewProfile {
        NewProfile { name: name.into(), kind: ProfileKind::Performance, mc_version: "1.21.11".into(), loader: LoaderKind::Fabric, loader_version: None, game_dir: None, preset: Some("low".into()), builtin_mods: Some(true), ingame_features: None }
    }

    #[test]
    fn create_list_rename() {
        let (_d, paths) = setup();
        let pr = Profiles::new(&paths);
        let p = pr.create(req("Perf")).unwrap();
        assert!(pr.game_dir(&p).join("mods").is_dir());
        assert!(pr.is_managed(&p));
        assert_eq!(pr.list().len(), 1);
        assert_eq!(pr.rename(&p.id, "Fast").unwrap().name, "Fast");
        assert!(pr.rename(&p.id, "   ").is_err());
        assert!(pr.get("../evil").is_err());
    }

    #[test]
    fn profiles_are_isolated() {
        let (_d, paths) = setup();
        let pr = Profiles::new(&paths);
        let a = pr.create(req("A")).unwrap();
        let b = pr.create(req("B")).unwrap();
        assert_ne!(pr.game_dir(&a), pr.game_dir(&b));
        std::fs::write(pr.game_dir(&a).join("mods").join("x.jar"), b"x").unwrap();
        assert!(!pr.game_dir(&b).join("mods").join("x.jar").exists());
    }

    #[test]
    fn delete_preserves_worlds() {
        let (_d, paths) = setup();
        let pr = Profiles::without_recycle_bin(&paths);
        let p = pr.create(req("World Keeper")).unwrap();
        let dir = pr.game_dir(&p);
        std::fs::create_dir_all(dir.join("saves").join("My World")).unwrap();
        std::fs::write(dir.join("saves").join("My World").join("level.dat"), b"world").unwrap();
        std::fs::write(dir.join("mods").join("m.jar"), b"m").unwrap();
        let report = pr.delete(&p.id).unwrap();
        assert!(!dir.exists(), "instance folder should be parked");
        assert!(report.parked.as_ref().unwrap().join("mods").join("m.jar").exists(), "mods are recoverable");
        assert_eq!(report.preserved.len(), 1);
        let kept = report.preserved[0].join("My World").join("level.dat");
        assert_eq!(std::fs::read(kept).unwrap(), b"world");
        assert!(pr.get(&p.id).is_err());
    }

    #[test]
    fn delete_never_touches_external_folders() {
        let (d, paths) = setup();
        let pr = Profiles::new(&paths);
        let ext = d.path().join("official-mc");
        std::fs::create_dir_all(ext.join("saves").join("W")).unwrap();
        std::fs::write(ext.join("options.txt"), b"x").unwrap();
        let mut r = req("Official");
        r.game_dir = Some(ext.clone());
        let p = pr.create(r).unwrap();
        assert!(p.external);
        let report = pr.delete(&p.id).unwrap();
        assert!(report.preserved.is_empty() && report.recycled.is_none());
        assert!(ext.join("options.txt").exists() && ext.join("saves").join("W").exists());
    }

    #[test]
    fn duplicate_skips_worlds_unless_asked() {
        let (_d, paths) = setup();
        let pr = Profiles::new(&paths);
        let p = pr.create(req("Src")).unwrap();
        let dir = pr.game_dir(&p);
        std::fs::create_dir_all(dir.join("saves").join("W")).unwrap();
        std::fs::write(dir.join("config").join("c.json"), b"{}").unwrap();
        let c = pr.duplicate(&p.id, "Copy", false).unwrap();
        let cd = pr.game_dir(&c);
        assert!(cd.join("config").join("c.json").exists());
        assert!(!cd.join("saves").join("W").exists());
        let c2 = pr.duplicate(&p.id, "Copy2", true).unwrap();
        assert!(pr.game_dir(&c2).join("saves").join("W").exists());
    }
}
