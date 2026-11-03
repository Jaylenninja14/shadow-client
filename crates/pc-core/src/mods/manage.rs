//! Operations on a profile's content folders: scan, install (execute a plan), enable/disable,
//! remove (with backup), restore, update checks.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::compat::{self, Conflict};
use super::index::{IdentifiedProject, IndexEntry, ModIndex};
use super::metadata::{self, ModMeta};
use super::modrinth as mr;
use super::resolver::{InstallPlan, InstalledSet, PlannedFile};
use super::{ContentKind, Target};
use crate::context::Ctx;
use crate::download::DownloadItem;
use crate::{safepath, util, Error, Result};

/// Everything the mod operations need to know about a profile.
#[derive(Debug, Clone)]
pub struct ProfileScope {
    pub profile_id: String,
    pub game_dir: PathBuf,
    pub data_dir: PathBuf,
    pub target: Target,
}

impl ProfileScope {
    pub fn folder(&self, kind: ContentKind) -> PathBuf {
        self.game_dir.join(kind.folder())
    }
    pub fn backups(&self, kind: ContentKind) -> PathBuf {
        self.data_dir.join("backups").join(kind.folder())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version_id: String,
    pub version_number: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub file_name: String,
    pub path: PathBuf,
    pub enabled: bool,
    pub size: u64,
    pub sha1: String,
    pub meta: Option<ModMeta>,
    pub icon: Option<String>,
    pub source: Option<IndexEntry>,
    pub identified: Option<IdentifiedProject>,
    pub update: Option<UpdateInfo>,
    pub warnings: Vec<String>,
    pub builtin: bool,
}

impl InstalledMod {
    pub fn display_name(&self) -> String {
        self.meta
            .as_ref()
            .map(|m| m.name.clone())
            .or_else(|| self.source.as_ref().and_then(|s| s.title.clone()))
            .or_else(|| self.identified.as_ref().and_then(|i| i.title.clone()))
            .unwrap_or_else(|| self.file_name.clone())
    }
    pub fn project_id(&self) -> Option<String> {
        self.source.as_ref().and_then(|s| s.project_id.clone()).or_else(|| self.identified.as_ref().map(|i| i.project_id.clone()))
    }
}

/// Ids provided by the game or the loader itself (Fabric Loader ≥ 0.15, Quilt, Forge and
/// NeoForge all bundle MixinExtras), so they are never "missing".
const IGNORED_DEPS: &[&str] = &["minecraft", "java", "fabricloader", "fabric-loader", "quilt_loader", "quilt_base", "forge", "neoforge", "javafml", "fml", "mixinextras"];

fn is_content_file(kind: ContentKind, name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let base = lower.strip_suffix(".disabled").unwrap_or(&lower);
    match kind {
        ContentKind::Mod => base.ends_with(".jar"),
        ContentKind::Resourcepack | ContentKind::Shader => base.ends_with(".zip"),
    }
}

/// Scan the mods folder, merge launcher index data and compute warnings.
pub fn scan_mods(scope: &ProfileScope) -> Result<Vec<InstalledMod>> {
    let dir = scope.folder(ContentKind::Mod);
    let mut index = ModIndex::load(&scope.data_dir);
    let mut out = vec![];
    if dir.is_dir() {
        for e in std::fs::read_dir(&dir)?.flatten() {
            let path = e.path();
            if !path.is_file() {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if !is_content_file(ContentKind::Mod, &name) {
                continue;
            }
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            let sha1 = index.cached_sha1(&path).unwrap_or_default();
            let meta = metadata::read_jar(&path).ok().flatten();
            let icon = meta.as_ref().and_then(|m| m.icon_path.as_deref()).and_then(|ip| metadata::read_icon(&path, ip));
            let source = index.by_file(ContentKind::Mod, &name).cloned();
            let identified = index.identified.get(&sha1).cloned().flatten();
            let builtin = source.as_ref().is_some_and(|s| s.builtin);
            out.push(InstalledMod {
                enabled: !name.to_ascii_lowercase().ends_with(".disabled"),
                file_name: name,
                path,
                size,
                sha1,
                meta,
                icon,
                source,
                identified,
                update: None,
                warnings: vec![],
                builtin,
            });
        }
    }
    let _ = index.save(&scope.data_dir);
    compute_warnings(&mut out, &scope.target);
    out.sort_by_key(|m| m.display_name().to_lowercase());
    Ok(out)
}

/// Dependency/compatibility warnings across the enabled mods.
pub fn compute_warnings(mods: &mut [InstalledMod], target: &Target) {
    let enabled: Vec<&InstalledMod> = mods.iter().filter(|m| m.enabled).collect();
    let mut present: HashMap<String, String> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for m in &enabled {
        if let Some(meta) = &m.meta {
            present.insert(meta.mod_id.clone(), meta.version.clone());
            *counts.entry(meta.mod_id.clone()).or_default() += 1;
            for p in &meta.provides {
                present.entry(p.clone()).or_insert_with(|| meta.version.clone());
            }
        }
    }
    let has_fabric_api = present.contains_key("fabric-api") || present.contains_key("fabric");
    let accepted = target.loader.accepted_mod_loaders();
    let ids: BTreeSet<String> = present.keys().cloned().collect();
    let conflicts = compat::find_conflicts(&ids);

    for m in mods.iter_mut() {
        if !m.enabled {
            continue;
        }
        let Some(meta) = m.meta.clone() else {
            if !accepted.is_empty() {
                m.warnings.push("No Fabric/Quilt/Forge metadata found: this file may not be a mod for this loader.".into());
            }
            continue;
        };
        if accepted.is_empty() {
            m.warnings.push("This profile is Vanilla, so mods are not loaded.".into());
        } else if !accepted.contains(&meta.loader.as_str()) {
            m.warnings.push(format!("This is a {} mod but the profile uses {}.", meta.loader, target.loader.as_str()));
        }
        if let Some(preds) = meta.depends.get("minecraft") {
            if metadata::matches_any(&target.mc_version, preds) == Some(false) {
                m.warnings.push(format!("Made for Minecraft {}, but this profile is {}.", preds.join(" or "), target.mc_version));
            }
        }
        for (dep, preds) in &meta.depends {
            if IGNORED_DEPS.contains(&dep.as_str()) {
                continue;
            }
            if dep.starts_with("fabric-") && has_fabric_api {
                continue; // Fabric API modules are bundled in fabric-api
            }
            match present.get(dep) {
                None => m.warnings.push(format!("Missing required dependency: {dep}")),
                Some(v) => {
                    if metadata::matches_any(v, preds) == Some(false) {
                        m.warnings.push(format!("Needs {dep} {} but {v} is installed.", preds.join(" or ")));
                    }
                }
            }
        }
        for c in compat::declared_breaks(&meta.mod_id, &meta.breaks, &present) {
            m.warnings.push(c.reason);
        }
        for c in &conflicts {
            if c.a == meta.mod_id || c.b == meta.mod_id {
                m.warnings.push(format!("Conflicts with {}: {}", if c.a == meta.mod_id { &c.b } else { &c.a }, c.reason));
            }
        }
        if counts.get(&meta.mod_id).copied().unwrap_or(0) > 1 {
            m.warnings.push(format!("Duplicate: more than one enabled jar provides '{}'. Disable or remove the extra copy.", meta.mod_id));
        }
    }
}

/// Ask Modrinth which project each unknown jar belongs to (by SHA-1). Results are cached.
pub async fn identify(ctx: &Ctx, scope: &ProfileScope, mods: &mut [InstalledMod]) -> Result<()> {
    let mut index = ModIndex::load(&scope.data_dir);
    let unknown: Vec<String> = mods
        .iter()
        .filter(|m| m.source.as_ref().and_then(|s| s.project_id.clone()).is_none() && !index.identified.contains_key(&m.sha1) && !m.sha1.is_empty())
        .map(|m| m.sha1.clone())
        .collect();
    if !unknown.is_empty() {
        let found = mr::versions_by_hashes(ctx, &unknown).await?;
        let pids: Vec<String> = found.values().map(|v| v.project_id.clone()).collect::<BTreeSet<_>>().into_iter().collect();
        let projects = mr::projects(ctx, &pids).await.unwrap_or_default();
        for h in &unknown {
            let ident = found.get(h).map(|v| {
                let p = projects.iter().find(|p| p.id == v.project_id);
                IdentifiedProject {
                    project_id: v.project_id.clone(),
                    version_id: v.id.clone(),
                    title: p.map(|p| p.title.clone()),
                    slug: p.and_then(|p| p.slug.clone()),
                    icon_url: p.and_then(|p| p.icon_url.clone()),
                    version_number: v.version_number.clone(),
                }
            });
            index.identified.insert(h.clone(), ident);
        }
        index.save(&scope.data_dir)?;
    }
    for m in mods.iter_mut() {
        if m.identified.is_none() {
            m.identified = index.identified.get(&m.sha1).cloned().flatten();
        }
    }
    Ok(())
}

pub fn installed_set(mods: &[InstalledMod]) -> InstalledSet {
    let mut s = InstalledSet::default();
    for m in mods {
        if let Some(pid) = m.project_id() {
            s.project_ids.insert(pid.clone());
            s.files_by_project.insert(pid, m.file_name.clone());
        }
        if m.enabled {
            if let Some(meta) = &m.meta {
                s.mod_ids.insert(meta.mod_id.clone());
            }
        }
    }
    s
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
    pub backups: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub manual: Vec<PlannedFile>,
    pub restart_required: bool,
}

/// Embedded jars (built-in mods) by file name.
pub type EmbeddedLookup = fn(&str) -> Option<&'static [u8]>;

fn check_file_name(name: &str) -> Result<()> {
    let segs = safepath::validate_relative(name)?;
    if segs.len() != 1 {
        return Err(Error::UnsafePath(format!("file name {name:?} contains folders")));
    }
    Ok(())
}

/// Verify a downloaded mod jar against the profile before it is installed.
fn verify_mod_jar(path: &Path, pf: &PlannedFile, target: &Target) -> Result<(Option<ModMeta>, Vec<String>)> {
    let mut warnings = vec![];
    let meta = metadata::read_jar(path).map_err(|e| Error::Invalid(format!("{} is not a valid jar: {e}", pf.file_name)))?;
    let accepted = target.loader.accepted_mod_loaders();
    match &meta {
        None => {
            return Err(Error::Incompatible(format!("{} does not contain {} metadata, so {} would not load it.", pf.file_name, target.loader.as_str(), target.loader.as_str())));
        }
        Some(m) => {
            if !accepted.contains(&m.loader.as_str()) {
                return Err(Error::Incompatible(format!("{} is a {} mod; this profile uses {}.", pf.title, m.loader, target.loader.as_str())));
            }
            if let Some(preds) = m.depends.get("minecraft") {
                match metadata::matches_any(&target.mc_version, preds) {
                    Some(false) => {
                        return Err(Error::Incompatible(format!(
                            "{} declares Minecraft {}, but this profile is {}. It was not installed.",
                            pf.title,
                            preds.join(" or "),
                            target.mc_version
                        )))
                    }
                    None => warnings.push(format!("Could not verify {}'s Minecraft requirement ({}).", pf.title, preds.join(" or "))),
                    _ => {}
                }
            }
        }
    }
    Ok((meta, warnings))
}

/// Execute a plan: download to a temp folder, verify hashes and metadata, re-check conflicts
/// with the real mod ids, back up anything replaced, then move files into place.
pub async fn execute(ctx: &Ctx, scope: &ProfileScope, plan: &InstallPlan, embedded: EmbeddedLookup) -> Result<InstallReport> {
    if plan.blocked {
        return Err(Error::Incompatible(plan.blocked_reason.clone().unwrap_or_else(|| "This installation is blocked".into())));
    }
    let mut report = InstallReport { manual: plan.manual_downloads.clone(), ..Default::default() };
    if plan.files.is_empty() {
        return Ok(report);
    }
    let staging = ctx.paths.tmp_dir().join(format!("install-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&staging)?;
    let result = execute_inner(ctx, scope, plan, embedded, &staging, &mut report).await;
    let _ = std::fs::remove_dir_all(&staging);
    result.map(|_| report)
}

async fn execute_inner(ctx: &Ctx, scope: &ProfileScope, plan: &InstallPlan, embedded: EmbeddedLookup, staging: &Path, report: &mut InstallReport) -> Result<()> {
    let mut items = vec![];
    for f in &plan.files {
        check_file_name(&f.file_name)?;
        let dest = staging.join(&f.file_name);
        if let Some(name) = &f.embedded {
            let bytes = embedded(name).ok_or_else(|| Error::NotFound(format!("embedded file {name}")))?;
            std::fs::write(&dest, bytes)?;
            util::verify_file(&dest, f.sha1.as_deref(), f.sha512.as_deref(), f.size)?;
            continue;
        }
        let url = f.url.clone().ok_or_else(|| Error::Invalid(format!("{} has no download URL", f.title)))?;
        items.push(DownloadItem::new(url, dest).sha1(f.sha1.clone()).sha512(f.sha512.clone()).size(f.size));
    }
    if !items.is_empty() {
        // Name the job after a file that is actually downloaded (embedded built-ins are not).
        let downloaded: Vec<&PlannedFile> = plan.files.iter().filter(|f| f.embedded.is_none()).collect();
        let first = downloaded.iter().find(|f| f.reason == "requested").or(downloaded.first()).map(|f| f.title.clone()).unwrap_or_default();
        let title = if items.len() > 1 { format!("{first} (+{} more)", items.len() - 1) } else { first };
        ctx.downloads.run(&title, items).await?;
    }

    // Verify metadata of mods and collect real mod ids.
    let mut new_meta: Vec<(PlannedFile, Option<ModMeta>)> = vec![];
    for f in &plan.files {
        let path = staging.join(&f.file_name);
        if f.kind == ContentKind::Mod {
            let (meta, warns) = verify_mod_jar(&path, f, &scope.target)?;
            report.warnings.extend(warns);
            new_meta.push((f.clone(), meta));
        } else {
            new_meta.push((f.clone(), None));
        }
    }

    // Re-check conflicts with what is actually on disk now.
    let current = if scope.folder(ContentKind::Mod).exists() { scan_mods(scope)? } else { vec![] };
    let replaced: BTreeSet<String> = plan.files.iter().filter_map(|f| f.replaces.clone()).collect();
    let mut present: HashMap<String, String> = HashMap::new();
    for m in current.iter().filter(|m| m.enabled && !replaced.contains(&m.file_name)) {
        if let Some(meta) = &m.meta {
            present.insert(meta.mod_id.clone(), meta.version.clone());
        }
    }
    let mut conflicts: Vec<Conflict> = vec![];
    for (_, meta) in &new_meta {
        if let Some(meta) = meta {
            conflicts.extend(compat::declared_breaks(&meta.mod_id, &meta.breaks, &present));
            present.insert(meta.mod_id.clone(), meta.version.clone());
        }
    }
    for m in current.iter().filter(|m| m.enabled) {
        if let Some(meta) = &m.meta {
            let mut others = present.clone();
            others.remove(&meta.mod_id);
            conflicts.extend(compat::declared_breaks(&meta.mod_id, &meta.breaks, &others).into_iter().filter(|c| new_meta.iter().any(|(_, nm)| nm.as_ref().is_some_and(|n| n.mod_id == c.b))));
        }
    }
    conflicts.extend(compat::find_conflicts(&present.keys().cloned().collect()));
    if let Some(c) = conflicts.first() {
        return Err(Error::Incompatible(format!("Nothing was installed: {} conflicts with {}. {}", c.a, c.b, c.reason)));
    }

    // Duplicate guard: same mod id already installed under another file name (and not replaced).
    for (f, meta) in &new_meta {
        if let Some(meta) = meta {
            if let Some(dupe) = current.iter().find(|m| {
                m.meta.as_ref().is_some_and(|x| x.mod_id == meta.mod_id) && Some(&m.file_name) != f.replaces.as_ref() && m.file_name != f.file_name
            }) {
                return Err(Error::Incompatible(format!(
                    "{} is already installed as {}. Update or remove that file instead of installing a second copy.",
                    meta.name, dupe.file_name
                )));
            }
        }
    }

    // Move into place with backups.
    let mut index = ModIndex::load(&scope.data_dir);
    let ts = util::timestamp_slug();
    for (f, meta) in new_meta {
        let folder = scope.folder(f.kind);
        std::fs::create_dir_all(&folder)?;
        let dest = folder.join(&f.file_name);
        let staged = staging.join(&f.file_name);
        let mut was_builtin = false;
        if let Some(old) = &f.replaces {
            check_file_name(old)?;
            was_builtin = index.by_file(f.kind, old).is_some_and(|e| e.builtin);
            let old_path = folder.join(old);
            if old_path.exists() {
                let b = scope.backups(f.kind).join(&ts).join(old);
                util::move_path(&old_path, &b)?;
                report.backups.push(b);
                index.remove_file(f.kind, old);
            }
        }
        if dest.exists() {
            if util::file_matches(&dest, f.sha1.as_deref(), f.sha512.as_deref(), None) {
                report.skipped.push(format!("{} (already present)", f.file_name));
                continue;
            }
            let b = scope.backups(f.kind).join(&ts).join(&f.file_name);
            util::move_path(&dest, &b)?;
            report.backups.push(b);
        }
        util::move_path(&staged, &dest)?;
        let hashes = util::hash_file(&dest)?;
        index.upsert(IndexEntry {
            file_name: f.file_name.clone(),
            kind: f.kind,
            sha1: hashes.sha1,
            sha512: Some(hashes.sha512),
            source: if f.builtin || was_builtin { "builtin".into() } else { f.platform.clone() },
            project_id: Some(f.project_id.clone()),
            version_id: Some(f.version_id.clone()),
            slug: f.slug.clone(),
            title: Some(meta.as_ref().map(|m| m.name.clone()).unwrap_or(f.title.clone())),
            version_number: Some(f.version_number.clone()),
            icon_url: f.icon_url.clone(),
            website_url: f.website_url.clone(),
            explicit: f.reason != "dependency",
            builtin: f.builtin || was_builtin,
            installed_at: util::now_iso(),
            required_by: f.required_by.iter().cloned().collect(),
            game_versions: f.game_versions.clone(),
            loaders: f.loaders.clone(),
        });
        report.installed.push(f.title.clone());
    }
    index.save(&scope.data_dir)?;
    report.restart_required = true;
    Ok(())
}

/// Enable/disable a mod by renaming `x.jar` ↔ `x.jar.disabled` (Fabric/Quilt skip .disabled).
/// Built-in mods installed by the launcher are always on: never disabled or removed here.
fn is_locked_builtin(scope: &ProfileScope, kind: ContentKind, file_name: &str) -> bool {
    kind == ContentKind::Mod && ModIndex::load(&scope.data_dir).by_file(kind, file_name).is_some_and(|e| e.builtin)
}

pub fn set_enabled(scope: &ProfileScope, kind: ContentKind, file_name: &str, enabled: bool) -> Result<String> {
    check_file_name(file_name)?;
    if !enabled && is_locked_builtin(scope, kind, file_name) {
        return Err(Error::Invalid("Built-in mods are always on and can't be disabled.".into()));
    }
    let folder = scope.folder(kind);
    let path = folder.join(file_name);
    if !path.is_file() {
        return Err(Error::NotFound(file_name.into()));
    }
    let is_disabled = file_name.to_ascii_lowercase().ends_with(".disabled");
    let new_name = match (enabled, is_disabled) {
        (true, true) => file_name[..file_name.len() - ".disabled".len()].to_string(),
        (false, false) => format!("{file_name}.disabled"),
        _ => return Ok(file_name.to_string()),
    };
    let new_path = folder.join(&new_name);
    if new_path.exists() {
        return Err(Error::Invalid(format!("{new_name} already exists; remove the duplicate first")));
    }
    std::fs::rename(&path, &new_path)?;
    let mut index = ModIndex::load(&scope.data_dir);
    index.rename_file(kind, file_name, &new_name);
    index.save(&scope.data_dir)?;
    Ok(new_name)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveReport {
    pub backup: PathBuf,
    pub dependents: Vec<String>,
}

/// Remove a file by moving it into the profile's backup folder (recoverable).
pub fn remove(scope: &ProfileScope, kind: ContentKind, file_name: &str) -> Result<RemoveReport> {
    remove_file(scope, kind, file_name, false)
}

/// Take out an outdated or renamed built-in mod (the launcher's own file), recoverable like any
/// other removal. Users can't do this; the launcher does it before installing the current build.
pub(crate) fn remove_builtin(scope: &ProfileScope, file_name: &str) -> Result<RemoveReport> {
    remove_file(scope, ContentKind::Mod, file_name, true)
}

fn remove_file(scope: &ProfileScope, kind: ContentKind, file_name: &str, allow_builtin: bool) -> Result<RemoveReport> {
    check_file_name(file_name)?;
    if !is_content_file(kind, file_name) {
        return Err(Error::Invalid(format!("{file_name} is not a {} file managed here", kind.folder())));
    }
    let path = scope.folder(kind).join(file_name);
    if !path.is_file() {
        return Err(Error::NotFound(file_name.into()));
    }
    if !allow_builtin && is_locked_builtin(scope, kind, file_name) {
        return Err(Error::Invalid("Built-in mods are part of Shadow Client and can't be removed.".into()));
    }
    let mut dependents = vec![];
    if kind == ContentKind::Mod {
        if let Ok(Some(meta)) = metadata::read_jar(&path) {
            for m in scan_mods(scope)?.iter().filter(|m| m.enabled && m.file_name != file_name) {
                if m.meta.as_ref().is_some_and(|x| x.depends.contains_key(&meta.mod_id)) {
                    dependents.push(m.display_name());
                }
            }
        }
    }
    let backup = scope.backups(kind).join(util::timestamp_slug()).join(file_name);
    util::move_path(&path, &backup)?;
    let mut index = ModIndex::load(&scope.data_dir);
    if let Some(mut entry) = index.remove_file(kind, file_name) {
        // remember where it came from so it can be restored with metadata
        entry.file_name = file_name.to_string();
        let _ = util::write_json(&backup.with_extension("pcmeta.json"), &entry);
    }
    index.save(&scope.data_dir)?;
    Ok(RemoveReport { backup, dependents })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub file_name: String,
    pub path: PathBuf,
    pub taken_at: String,
    pub title: Option<String>,
}

pub fn list_backups(scope: &ProfileScope, kind: ContentKind) -> Vec<BackupEntry> {
    let mut out = vec![];
    let root = scope.backups(kind);
    for stamp in std::fs::read_dir(&root).into_iter().flatten().flatten() {
        if !stamp.path().is_dir() {
            continue;
        }
        for f in std::fs::read_dir(stamp.path()).into_iter().flatten().flatten() {
            let name = f.file_name().to_string_lossy().into_owned();
            if name.ends_with(".pcmeta.json") || !f.path().is_file() {
                continue;
            }
            let meta: Option<IndexEntry> = util::read_json_opt(&f.path().with_extension("pcmeta.json")).ok().flatten();
            out.push(BackupEntry { file_name: name, path: f.path(), taken_at: stamp.file_name().to_string_lossy().into_owned(), title: meta.and_then(|m| m.title) });
        }
    }
    out.sort_by(|a, b| b.taken_at.cmp(&a.taken_at));
    out
}

/// Restore a backed-up file. If a file for the same mod is installed now, it is backed up first.
pub fn restore(scope: &ProfileScope, kind: ContentKind, backup_path: &Path) -> Result<String> {
    let root = crate::paths::canonical_or_self(&scope.backups(kind));
    let real = crate::paths::canonical_or_self(backup_path);
    if !real.starts_with(&root) || !real.is_file() {
        return Err(Error::UnsafePath("backup path is outside this profile's backups".into()));
    }
    let name = real.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or_else(|| Error::Invalid("bad backup".into()))?;
    check_file_name(&name)?;
    let folder = scope.folder(kind);
    std::fs::create_dir_all(&folder)?;
    // Back up a current file of the same mod id (the version being rolled back).
    if kind == ContentKind::Mod {
        if let Ok(Some(meta)) = metadata::read_jar(&real) {
            for m in scan_mods(scope)? {
                if m.meta.as_ref().is_some_and(|x| x.mod_id == meta.mod_id) {
                    remove(scope, kind, &m.file_name)?;
                }
            }
        }
    }
    let dest = util::unique_path(&folder.join(&name));
    util::move_path(&real, &dest)?;
    let meta_path = real.with_extension("pcmeta.json");
    let mut index = ModIndex::load(&scope.data_dir);
    let mut entry: IndexEntry = util::read_json_opt(&meta_path).ok().flatten().unwrap_or_default();
    let _ = std::fs::remove_file(&meta_path);
    entry.file_name = dest.file_name().unwrap().to_string_lossy().into_owned();
    entry.kind = kind;
    entry.sha1 = util::sha1_file(&dest)?;
    index.upsert(entry);
    index.save(&scope.data_dir)?;
    Ok(dest.file_name().unwrap().to_string_lossy().into_owned())
}

/// Check Modrinth for updates of identified/installed mods (same MC version + loader only).
pub async fn check_updates(ctx: &Ctx, scope: &ProfileScope, mods: &mut [InstalledMod]) -> Result<usize> {
    let loaders = scope.target.loader.accepted_mod_loaders();
    let hashes: Vec<String> = mods.iter().filter(|m| m.project_id().is_some() && !m.sha1.is_empty()).map(|m| m.sha1.clone()).collect();
    let updates = mr::updates_by_hashes(ctx, &hashes, &loaders, &[&scope.target.mc_version]).await?;
    let mut n = 0;
    for m in mods.iter_mut() {
        if let Some(v) = updates.get(&m.sha1) {
            let current_version = m.source.as_ref().and_then(|s| s.version_id.clone()).or_else(|| m.identified.as_ref().map(|i| i.version_id.clone()));
            let same_file = v.files.iter().any(|f| f.hashes.sha1.as_deref() == Some(m.sha1.as_str()));
            if !same_file && current_version.as_deref() != Some(v.id.as_str()) {
                m.update = Some(UpdateInfo { version_id: v.id.clone(), version_number: v.version_number.clone(), platform: "modrinth".into() });
                n += 1;
            }
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft::loaders::LoaderKind;
    use crate::mods::metadata::tests::make_jar;

    fn scope(dir: &Path) -> ProfileScope {
        let s = ProfileScope {
            profile_id: "p".into(),
            game_dir: dir.join("game"),
            data_dir: dir.join("data"),
            target: Target::new("1.21.11", LoaderKind::Fabric),
        };
        std::fs::create_dir_all(s.folder(ContentKind::Mod)).unwrap();
        s
    }

    #[test]
    fn warnings_for_missing_deps_duplicates_and_versions() {
        let d = tempfile::tempdir().unwrap();
        let s = scope(d.path());
        let mods = s.folder(ContentKind::Mod);
        make_jar(&mods.join("a.jar"), r#"{"id":"a","version":"1","depends":{"minecraft":"1.21.11","fabric-api":"*","b":">=2"}}"#);
        make_jar(&mods.join("old.jar"), r#"{"id":"old","version":"1","depends":{"minecraft":"1.20.1"}}"#);
        make_jar(&mods.join("dup1.jar"), r#"{"id":"dup","version":"1"}"#);
        make_jar(&mods.join("dup2.jar"), r#"{"id":"dup","version":"2"}"#);
        make_jar(&mods.join("sodium.jar"), r#"{"id":"sodium","version":"0.6"}"#);
        make_jar(&mods.join("optifabric.jar"), r#"{"id":"optifabric","version":"1"}"#);
        let list = scan_mods(&s).unwrap();
        let w = |id: &str| list.iter().find(|m| m.meta.as_ref().unwrap().mod_id == id && m.enabled).unwrap().warnings.join(" | ");
        assert!(w("a").contains("Missing required dependency: fabric-api"));
        assert!(w("a").contains("Missing required dependency: b"));
        assert!(w("old").contains("Made for Minecraft 1.20.1"));
        assert!(w("dup").contains("Duplicate"));
        assert!(w("sodium").contains("Conflicts with optifabric"));
    }

    #[test]
    fn enable_disable_remove_restore_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let s = scope(d.path());
        let mods = s.folder(ContentKind::Mod);
        make_jar(&mods.join("lib.jar"), r#"{"id":"lib","version":"1"}"#);
        make_jar(&mods.join("user.jar"), r#"{"id":"user","version":"1","depends":{"lib":"*"}}"#);
        let n = set_enabled(&s, ContentKind::Mod, "lib.jar", false).unwrap();
        assert_eq!(n, "lib.jar.disabled");
        assert!(scan_mods(&s).unwrap().iter().any(|m| m.file_name == "user.jar" && m.warnings.iter().any(|w| w.contains("Missing required dependency: lib"))));
        set_enabled(&s, ContentKind::Mod, "lib.jar.disabled", true).unwrap();
        let r = remove(&s, ContentKind::Mod, "lib.jar").unwrap();
        assert_eq!(r.dependents, vec!["user".to_string()]);
        assert!(!mods.join("lib.jar").exists() && r.backup.exists());
        let backups = list_backups(&s, ContentKind::Mod);
        assert_eq!(backups.len(), 1);
        let restored = restore(&s, ContentKind::Mod, &backups[0].path).unwrap();
        assert_eq!(restored, "lib.jar");
        assert!(mods.join("lib.jar").exists());
        // built-ins are locked
        let mut idx = ModIndex::load(&s.data_dir);
        idx.upsert(IndexEntry { file_name: "user.jar".into(), builtin: true, source: "builtin".into(), ..Default::default() });
        idx.save(&s.data_dir).unwrap();
        assert!(set_enabled(&s, ContentKind::Mod, "user.jar", false).is_err());
        assert!(remove(&s, ContentKind::Mod, "user.jar").is_err());
        assert!(mods.join("user.jar").exists());
        // path traversal on remove is rejected
        assert!(remove(&s, ContentKind::Mod, "../game/x.jar").is_err());
        assert!(remove(&s, ContentKind::Mod, "notes.txt").is_err());
    }
}
