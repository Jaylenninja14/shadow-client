//! Dependency resolution and compatibility planning.
//!
//! Given a requested project and the profile's Minecraft version + loader, produce an
//! [`InstallPlan`] listing every file that will be added, optional extras, conflicts and
//! warnings. Nothing is downloaded here; the plan is shown to the user first.

use std::collections::{BTreeSet, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::compat::{self, Conflict};
use super::curseforge as cf;
use super::modrinth as mr;
use super::{ContentKind, Target};
use crate::context::Ctx;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    /// "modrinth" | "curseforge" | "builtin"
    pub platform: String,
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub slug: Option<String>,
    pub version_number: String,
    pub file_name: String,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub size: Option<u64>,
    /// "requested" | "dependency" | "builtin" | "update"
    pub reason: String,
    pub required_by: Option<String>,
    /// Existing file this one replaces (update / reinstall)
    pub replaces: Option<String>,
    pub icon_url: Option<String>,
    pub website_url: Option<String>,
    /// False when the platform does not allow third-party downloads (CurseForge opt-out)
    pub downloadable: bool,
    /// Name of a file embedded in the launcher (built-in mods)
    pub embedded: Option<String>,
    pub builtin: bool,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub kind: ContentKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OptionalDep {
    pub platform: String,
    pub project_id: String,
    pub title: String,
    pub required_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompatSuggestion {
    /// Newest Minecraft versions for which the project supports the profile's loader
    pub mc_versions: Vec<String>,
    /// Loaders the project supports for the profile's Minecraft version
    pub loaders: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallPlan {
    pub target: Option<Target>,
    pub files: Vec<PlannedFile>,
    pub optional: Vec<OptionalDep>,
    pub already_installed: Vec<String>,
    pub conflicts: Vec<Conflict>,
    pub warnings: Vec<String>,
    pub blocked: bool,
    pub blocked_reason: Option<String>,
    pub suggestion: Option<CompatSuggestion>,
    pub total_size: Option<u64>,
    pub manual_downloads: Vec<PlannedFile>,
}

/// Minimal view of what is already in the profile.
#[derive(Debug, Clone, Default)]
pub struct InstalledSet {
    pub project_ids: HashSet<String>,
    pub mod_ids: BTreeSet<String>,
    /// project id → file name (for updates)
    pub files_by_project: std::collections::HashMap<String, String>,
}

impl InstalledSet {
    /// A Modrinth project is already there, by id or (for jars copied in by hand, which have no
    /// project id) by the mod id it is known to ship.
    pub fn has_project(&self, project_id: &str) -> bool {
        self.project_ids.contains(project_id) || compat::mod_id_for_project(project_id).is_some_and(|m| self.mod_ids.contains(m))
    }
}

impl InstallPlan {
    fn block(&mut self, reason: String) {
        self.blocked = true;
        if self.blocked_reason.is_none() {
            self.blocked_reason = Some(reason);
        }
    }

    pub fn finalize(&mut self, installed: &InstalledSet) {
        // Conflicts among installed + planned (by mod id where we know it).
        let mut ids = installed.mod_ids.clone();
        for f in &self.files {
            if let Some(m) = compat::mod_id_for_project(&f.project_id) {
                ids.insert(m.to_string());
            }
        }
        for c in compat::find_conflicts(&ids) {
            if !self.conflicts.contains(&c) {
                self.conflicts.push(c);
            }
        }
        if let Some(c) = self.conflicts.iter().find(|c| c.severity == "error") {
            let msg = format!("Conflict: {} and {} cannot be used together. {}", c.a, c.b, c.reason);
            self.block(msg);
        }
        self.total_size = self.files.iter().map(|f| f.size).sum();
        let (manual, auto): (Vec<_>, Vec<_>) = self.files.drain(..).partition(|f| !f.downloadable);
        self.files = auto;
        self.manual_downloads = manual;
    }
}

fn accepted(target: &Target, kind: ContentKind) -> Vec<&'static str> {
    match kind {
        ContentKind::Mod => target.loader.accepted_mod_loaders(),
        // Resource packs use "minecraft"; shaders use iris/optifine/canvas/vanilla.
        ContentKind::Resourcepack => vec!["minecraft"],
        ContentKind::Shader => vec!["iris", "optifine", "canvas", "vanilla"],
    }
}

fn planned_from_mr(v: &mr::Version, project: Option<&mr::Project>, reason: &str, required_by: Option<String>, kind: ContentKind) -> Option<PlannedFile> {
    let f = v.primary_file()?;
    Some(PlannedFile {
        platform: "modrinth".into(),
        project_id: v.project_id.clone(),
        version_id: v.id.clone(),
        title: project.map(|p| p.title.clone()).unwrap_or_else(|| v.name.clone().unwrap_or_else(|| v.project_id.clone())),
        slug: project.and_then(|p| p.slug.clone()),
        version_number: v.version_number.clone(),
        file_name: f.filename.clone(),
        url: Some(f.url.clone()),
        sha1: f.hashes.sha1.clone(),
        sha512: f.hashes.sha512.clone(),
        size: f.size,
        reason: reason.into(),
        required_by,
        replaces: None,
        icon_url: project.and_then(|p| p.icon_url.clone()),
        website_url: project.map(|p| mr::project_url(&p.project_type, p.slug.as_deref().unwrap_or(&p.id))),
        downloadable: true,
        embedded: None,
        builtin: false,
        game_versions: v.game_versions.clone(),
        loaders: v.loaders.clone(),
        kind,
    })
}

/// Suggest Minecraft versions / loaders for which `project_id` is available.
pub async fn modrinth_suggestion(ctx: &Ctx, project_id: &str, target: &Target, kind: ContentKind) -> Option<CompatSuggestion> {
    let all = mr::project_versions(ctx, project_id, &[], &[]).await.ok()?;
    let acc = accepted(target, kind);
    let mut mcs: Vec<String> = vec![];
    let mut loaders: BTreeSet<String> = BTreeSet::new();
    for v in &all {
        if v.loaders.iter().any(|l| acc.contains(&l.as_str())) {
            for g in &v.game_versions {
                if !mcs.contains(g) {
                    mcs.push(g.clone());
                }
            }
        }
        if v.game_versions.iter().any(|g| g == &target.mc_version) {
            loaders.extend(v.loaders.iter().cloned());
        }
    }
    // Keep only full releases (x.y or x.y.z) and sort newest first.
    mcs.retain(|g| g.chars().all(|c| c.is_ascii_digit() || c == '.'));
    mcs.sort_by(|a, b| super::metadata::compare_versions(b, a).unwrap_or(std::cmp::Ordering::Equal));
    mcs.truncate(6);
    Some(CompatSuggestion { mc_versions: mcs, loaders: loaders.into_iter().collect() })
}

/// Resolve a Modrinth project (optionally a specific version) and its required dependencies.
pub async fn resolve_modrinth(
    ctx: &Ctx,
    target: &Target,
    kind: ContentKind,
    project_id: &str,
    version_id: Option<&str>,
    installed: &InstalledSet,
    allow_update_of_installed: bool,
) -> Result<InstallPlan> {
    let mut plan = InstallPlan { target: Some(target.clone()), ..Default::default() };
    let acc = accepted(target, kind);
    if kind == ContentKind::Mod && acc.is_empty() {
        plan.block("Vanilla profiles cannot load mods. Create a Fabric or Quilt profile for this Minecraft version.".into());
        return Ok(plan);
    }
    let mut queue: VecDeque<(String, Option<String>, String, Option<String>)> = VecDeque::new();
    queue.push_back((project_id.to_string(), version_id.map(String::from), "requested".into(), None));
    let mut visited: HashSet<String> = HashSet::new();

    while let Some((pid, vid, reason, required_by)) = queue.pop_front() {
        if !visited.insert(pid.clone()) {
            continue;
        }
        let is_root = reason == "requested";
        let present = if is_root { installed.project_ids.contains(&pid) } else { installed.has_project(&pid) };
        if present && !(is_root && allow_update_of_installed) {
            let title = mr::project(ctx, &pid).await.map(|p| p.title).unwrap_or(pid.clone());
            plan.already_installed.push(title);
            continue;
        }
        let project = mr::project(ctx, &pid).await.ok();
        let chosen = match &vid {
            Some(v) => {
                let ver = mr::version(ctx, v).await?;
                let mc_ok = ver.game_versions.iter().any(|g| g == &target.mc_version);
                let loader_ok = ver.loaders.iter().any(|l| acc.contains(&l.as_str()));
                if !mc_ok || !loader_ok {
                    let msg = format!(
                        "{} {} supports Minecraft {} on {}, but this profile is Minecraft {} with {}.",
                        project.as_ref().map(|p| p.title.as_str()).unwrap_or(&pid),
                        ver.version_number,
                        ver.game_versions.join(", "),
                        ver.loaders.join("/"),
                        target.mc_version,
                        target.loader.as_str()
                    );
                    if is_root {
                        plan.block(msg);
                        plan.suggestion = modrinth_suggestion(ctx, &pid, target, kind).await;
                        return Ok(plan);
                    }
                    plan.warnings.push(format!("Pinned dependency skipped: {msg}"));
                    None
                } else {
                    Some(ver)
                }
            }
            None => {
                let versions = mr::project_versions(ctx, &pid, &acc, &[&target.mc_version]).await?;
                mr::pick_best(&versions, &target.mc_version, &acc)
            }
        };
        let Some(ver) = chosen else {
            let title = project.as_ref().map(|p| p.title.clone()).unwrap_or(pid.clone());
            let msg = format!("{title} has no release for Minecraft {} on {}.", target.mc_version, target.loader.as_str());
            if is_root {
                plan.block(msg);
                plan.suggestion = modrinth_suggestion(ctx, &pid, target, kind).await;
                return Ok(plan);
            }
            plan.block(format!("Required dependency missing: {msg} (needed by {})", required_by.clone().unwrap_or_default()));
            continue;
        };
        if ver.version_type != "release" {
            plan.warnings.push(format!(
                "{} {} is a {} build; no stable release exists for this version.",
                project.as_ref().map(|p| p.title.as_str()).unwrap_or(&pid),
                ver.version_number,
                ver.version_type
            ));
        }
        let Some(mut file) = planned_from_mr(&ver, project.as_ref(), &reason, required_by.clone(), kind) else {
            plan.block(format!("{pid} has no downloadable file"));
            continue;
        };
        if let Some(existing) = installed.files_by_project.get(&pid) {
            file.replaces = Some(existing.clone());
            file.reason = "update".into();
        }
        let title = file.title.clone();
        plan.files.push(file);

        for dep in &ver.dependencies {
            let dep_pid = match (&dep.project_id, &dep.version_id) {
                (Some(p), _) => p.clone(),
                (None, Some(v)) => match mr::version(ctx, v).await {
                    Ok(dv) => dv.project_id,
                    Err(_) => continue,
                },
                _ => continue,
            };
            match dep.dependency_type.as_str() {
                "required" => queue.push_back((dep_pid, dep.version_id.clone(), "dependency".into(), Some(title.clone()))),
                "optional" => {
                    if !installed.project_ids.contains(&dep_pid) && !plan.optional.iter().any(|o| o.project_id == dep_pid) {
                        let t = mr::project(ctx, &dep_pid).await.map(|p| p.title).unwrap_or(dep_pid.clone());
                        plan.optional.push(OptionalDep { platform: "modrinth".into(), project_id: dep_pid, title: t, required_by: title.clone() });
                    }
                }
                "incompatible" => {
                    let in_plan = plan.files.iter().any(|f| f.project_id == dep_pid);
                    if installed.project_ids.contains(&dep_pid) || in_plan {
                        let t = mr::project(ctx, &dep_pid).await.map(|p| p.title).unwrap_or(dep_pid.clone());
                        plan.conflicts.push(Conflict { a: title.clone(), b: t.clone(), reason: format!("{title} declares it is incompatible with {t}"), severity: "error".into() });
                    }
                }
                _ => {} // embedded
            }
        }
    }
    plan.finalize(installed);
    Ok(plan)
}

fn planned_from_cf(m: &cf::CfMod, f: &cf::CfFile, reason: &str, required_by: Option<String>, kind: ContentKind) -> PlannedFile {
    PlannedFile {
        platform: "curseforge".into(),
        project_id: m.id.to_string(),
        version_id: f.id.to_string(),
        title: m.name.clone(),
        slug: Some(m.slug.clone()),
        version_number: f.display_name.clone(),
        file_name: f.file_name.clone(),
        url: f.download_url.clone(),
        sha1: f.sha1(),
        sha512: None,
        size: f.file_length,
        reason: reason.into(),
        required_by,
        replaces: None,
        icon_url: m.logo.as_ref().and_then(|l| l.thumbnail_url.clone()),
        website_url: Some(cf::project_url(m)),
        downloadable: f.download_url.is_some(),
        embedded: None,
        builtin: false,
        game_versions: f.game_versions.iter().filter(|g| g.chars().next().is_some_and(|c| c.is_ascii_digit())).cloned().collect(),
        loaders: f.game_versions.iter().filter(|g| cf::loader_type(&g.to_ascii_lowercase()).is_some()).map(|g| g.to_ascii_lowercase()).collect(),
        kind,
    }
}

/// Resolve a CurseForge project and its required dependencies.
pub async fn resolve_curseforge(ctx: &Ctx, target: &Target, kind: ContentKind, mod_id: u64, file_id: Option<u64>, installed: &InstalledSet) -> Result<InstallPlan> {
    let mut plan = InstallPlan { target: Some(target.clone()), ..Default::default() };
    let loader_names: Vec<&str> = if kind == ContentKind::Mod { target.loader.accepted_mod_loaders() } else { vec![] };
    if kind == ContentKind::Mod && loader_names.is_empty() {
        plan.block("Vanilla profiles cannot load mods. Create a Fabric or Quilt profile for this Minecraft version.".into());
        return Ok(plan);
    }
    let mut queue: VecDeque<(u64, Option<u64>, String, Option<String>)> = VecDeque::new();
    queue.push_back((mod_id, file_id, "requested".into(), None));
    let mut visited = HashSet::new();
    while let Some((mid, fid, reason, required_by)) = queue.pop_front() {
        if !visited.insert(mid) {
            continue;
        }
        let is_root = reason == "requested";
        let key = mid.to_string();
        if installed.project_ids.contains(&key) && !is_root {
            plan.already_installed.push(key);
            continue;
        }
        let m = cf::get_mod(ctx, mid).await?;
        let file = match fid {
            Some(f) => cf::get_files(ctx, &[f]).await?.into_iter().next(),
            None => {
                let mut best = None;
                for l in loader_names.iter().map(|l| Some(*l)).chain(if loader_names.is_empty() { vec![None] } else { vec![] }) {
                    let files = cf::mod_files(ctx, mid, Some(&target.mc_version), l).await?;
                    best = cf::pick_best(&files, &target.mc_version, l.map(|x| if x == "fabric" { "Fabric" } else if x == "quilt" { "Quilt" } else { x }));
                    if best.is_some() {
                        break;
                    }
                }
                best
            }
        };
        let Some(file) = file else {
            let msg = format!("{} has no file for Minecraft {} on {}.", m.name, target.mc_version, target.loader.as_str());
            if is_root {
                plan.block(msg);
                return Ok(plan);
            }
            plan.block(format!("Required dependency missing: {msg}"));
            continue;
        };
        if !file.supports_mc(&target.mc_version) {
            let msg = format!("{} ({}) is for Minecraft {}", m.name, file.display_name, file.game_versions.join(", "));
            if is_root {
                plan.block(msg);
                return Ok(plan);
            }
            plan.warnings.push(msg);
        }
        let mut pf = planned_from_cf(&m, &file, &reason, required_by.clone(), kind);
        if let Some(existing) = installed.files_by_project.get(&key) {
            pf.replaces = Some(existing.clone());
            pf.reason = "update".into();
        }
        if !pf.downloadable {
            plan.warnings.push(format!(
                "{}'s author does not allow downloads through third-party apps. Download it from the CurseForge page and drop it into the mods folder.",
                m.name
            ));
        }
        let title = m.name.clone();
        plan.files.push(pf);
        for dep in &file.dependencies {
            match dep.relation_type {
                3 => queue.push_back((dep.mod_id, None, "dependency".into(), Some(title.clone()))),
                2 => {
                    if !installed.project_ids.contains(&dep.mod_id.to_string()) {
                        let t = cf::get_mod(ctx, dep.mod_id).await.map(|x| x.name).unwrap_or(dep.mod_id.to_string());
                        plan.optional.push(OptionalDep { platform: "curseforge".into(), project_id: dep.mod_id.to_string(), title: t, required_by: title.clone() });
                    }
                }
                5 => {
                    if installed.project_ids.contains(&dep.mod_id.to_string()) {
                        plan.conflicts.push(Conflict { a: title.clone(), b: dep.mod_id.to_string(), reason: format!("{title} declares an incompatibility"), severity: "error".into() });
                    }
                }
                _ => {}
            }
        }
    }
    plan.finalize(installed);
    Ok(plan)
}

/// Merge a secondary plan (e.g. a dependency the user ticked) into `base`.
pub fn merge_plans(base: &mut InstallPlan, other: InstallPlan) {
    for f in other.files {
        if !base.files.iter().any(|x| x.project_id == f.project_id) {
            base.files.push(f);
        }
    }
    base.manual_downloads.extend(other.manual_downloads);
    base.warnings.extend(other.warnings);
    base.conflicts.extend(other.conflicts);
    if other.blocked {
        base.blocked = true;
        base.blocked_reason = base.blocked_reason.clone().or(other.blocked_reason);
    }
    base.total_size = base.files.iter().map(|f| f.size).sum();
}

pub fn not_found_is_incompatible(e: Error, what: &str) -> Error {
    match e {
        Error::NotFound(_) => Error::Incompatible(format!("{what} was not found")),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_copied_jars_count_as_installed_dependencies() {
        let mut s = InstalledSet::default();
        assert!(!s.has_project("P7dR8mSH"));
        s.mod_ids.insert("fabric-api".into());
        assert!(s.has_project("P7dR8mSH"), "Fabric API copied in without a Modrinth id isn't downloaded twice");
        assert!(!s.has_project("AANobbMI"));
    }
}
