//! Installing game versions: version JSONs, client jar, libraries, natives, assets.
//! All files are hash verified and stored once in the shared data root.

use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

use super::loaders::{self, LoaderKind};
use super::manifest::{self, AssetIndex, Library, MavenCoord, RuleEnv, VersionJson, VersionManifest};
use crate::context::Ctx;
use crate::download::{fetch_to_file, DownloadItem};
use crate::{safepath, util, Error, Result};

const MOJANG_LIBRARIES: &str = "https://libraries.minecraft.net/";

/// Fetch the version manifest (10 minute cache, offline fallback to the cached copy).
pub async fn manifest(ctx: &Ctx, force: bool) -> Result<VersionManifest> {
    let ttl = if force { Duration::from_secs(0) } else { Duration::from_secs(600) };
    ctx.http.get_json(&ctx.api.minecraft.version_manifest, Some(ttl), HeaderMap::new()).await
}

/// Load (and download if needed) the vanilla version JSON for `id`.
pub async fn vanilla_json(ctx: &Ctx, id: &str) -> Result<VersionJson> {
    let path = ctx.paths.version_json(id);
    match manifest(ctx, false).await {
        Ok(m) => {
            let entry = m.find(id).ok_or_else(|| Error::NotFound(format!("Minecraft version {id} is not in Mojang's version list")))?;
            let item = DownloadItem::new(entry.url.clone(), path.clone()).sha1(entry.sha1.clone());
            fetch_to_file(&ctx.http, &item).await?;
        }
        Err(e) => {
            if !path.exists() {
                return Err(e);
            }
            tracing::warn!("version manifest unavailable, using local {id}.json: {e}");
        }
    }
    util::read_json(&path)
}

/// Fully resolved version ready for launching.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedVersion {
    pub id: String,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
    pub json: VersionJson,
}

/// Resolve `mc` + loader to a merged version JSON, downloading loader metadata as needed.
pub async fn resolve_version(ctx: &Ctx, mc: &str, loader: LoaderKind, loader_version: Option<&str>) -> Result<ResolvedVersion> {
    let vanilla = vanilla_json(ctx, mc).await?;
    match loader {
        LoaderKind::Vanilla => Ok(ResolvedVersion { id: mc.to_string(), mc_version: mc.to_string(), loader, loader_version: None, json: vanilla }),
        LoaderKind::Fabric | LoaderKind::Quilt => {
            let lv = match loader_version {
                Some(v) if !v.is_empty() => v.to_string(),
                _ => loaders::recommended_loader(&ctx.http, &ctx.api.minecraft, loader, mc).await?,
            };
            let profile = loaders::profile_json(&ctx.http, &ctx.api.minecraft, loader, mc, &lv).await?;
            util::write_json(&ctx.paths.version_json(&profile.id), &profile)?;
            let merged = manifest::merge(&profile, &vanilla);
            Ok(ResolvedVersion { id: profile.id.clone(), mc_version: mc.to_string(), loader, loader_version: Some(lv), json: merged })
        }
        other => Err(Error::Incompatible(format!(
            "{} must be installed with its official installer. Install it into a .minecraft folder, then add that folder under Settings → Minecraft → Scan folders.",
            other.as_str()
        ))),
    }
}

/// Resolve a version that already exists in an external game folder (official launcher,
/// Forge/NeoForge installer output). Parent versions are looked up in the same folder first.
pub async fn resolve_external(ctx: &Ctx, root: &Path, id: &str) -> Result<ResolvedVersion> {
    let mut chain = vec![];
    let mut cur = id.to_string();
    for _ in 0..5 {
        let p = root.join("versions").join(&cur).join(format!("{cur}.json"));
        let v: VersionJson = if p.exists() { util::read_json(&p)? } else { vanilla_json(ctx, &cur).await? };
        let parent = v.inherits_from.clone();
        chain.push(v);
        match parent {
            Some(par) => cur = par,
            None => break,
        }
    }
    let mut merged = chain.pop().ok_or_else(|| Error::NotFound(id.to_string()))?;
    let mc_version = merged.id.clone();
    while let Some(child) = chain.pop() {
        merged = manifest::merge(&child, &merged);
    }
    let (loader, loader_version) = manifest::detect_loader(&merged);
    Ok(ResolvedVersion {
        id: id.to_string(),
        mc_version,
        loader: LoaderKind::parse(loader).unwrap_or(LoaderKind::Vanilla),
        loader_version,
        json: merged,
    })
}

#[derive(Debug, Clone, Default)]
pub struct InstallPlan {
    pub items: Vec<DownloadItem>,
    pub classpath: Vec<PathBuf>,
    pub natives_jars: Vec<(PathBuf, Vec<String>)>,
    pub client_jar: PathBuf,
    pub logging: Option<(PathBuf, String)>,
    /// Libraries that have no download URL and were not found locally.
    pub missing: Vec<String>,
}

fn natives_classifier(lib: &Library, env: &RuleEnv) -> Option<String> {
    let n = lib.natives.as_ref()?.get(env.os_name)?;
    let bits = if env.arch == "x86" { "32" } else { "64" };
    Some(n.replace("${arch}", bits))
}

/// Find a library in extra (read-only) library folders, e.g. an existing `.minecraft/libraries`.
fn find_in_extra(rel: &str, sha1: Option<&str>, extra: &[PathBuf]) -> Option<PathBuf> {
    for dir in extra {
        if let Ok(p) = safepath::safe_join(dir, rel) {
            if p.is_file() && (sha1.is_none() || util::file_matches(&p, sha1, None, None)) {
                return Some(p);
            }
        }
    }
    None
}

/// Build the list of files needed for a resolved version (no network access).
pub fn plan(ctx: &Ctx, v: &ResolvedVersion, env: &RuleEnv, extra_lib_dirs: &[PathBuf]) -> Result<InstallPlan> {
    let lib_root = ctx.paths.libraries_dir();
    let mut plan = InstallPlan::default();
    let mut seen = std::collections::HashSet::new();

    for lib in &v.json.libraries {
        if !manifest::rules_allow(lib.rules.as_deref(), env) {
            continue;
        }
        let coord = MavenCoord::parse(&lib.name)?;

        // Old-style natives (classifier map).
        if let Some(classifier) = natives_classifier(lib, env) {
            let art = lib.downloads.as_ref().and_then(|d| d.classifiers.as_ref()).and_then(|c| c.get(&classifier)).cloned();
            let mut c2 = coord.clone();
            c2.classifier = Some(classifier.clone());
            let rel = art.as_ref().and_then(|a| a.path.clone()).unwrap_or_else(|| c2.path());
            let dest = safepath::safe_join(&lib_root, &rel)?;
            let url = art.as_ref().map(|a| a.url.clone()).filter(|u| !u.is_empty()).unwrap_or_else(|| format!("{}{}", lib.url.clone().unwrap_or_else(|| MOJANG_LIBRARIES.into()), rel));
            let sha1 = art.as_ref().and_then(|a| a.sha1.clone());
            let excludes = lib.extract.as_ref().map(|e| e.exclude.clone()).unwrap_or_default();
            if let Some(p) = find_in_extra(&rel, sha1.as_deref(), extra_lib_dirs) {
                plan.natives_jars.push((p, excludes));
            } else {
                plan.items.push(DownloadItem::new(url, dest.clone()).sha1(sha1).size(art.and_then(|a| a.size)));
                plan.natives_jars.push((dest, excludes));
            }
            // Some old libraries are natives-only; others also have a main artifact.
            if lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()).is_none() {
                continue;
            }
        }

        let (rel, url, sha1, size) = if let Some(art) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            let rel = art.path.clone().unwrap_or_else(|| coord.path());
            (rel, art.url.clone(), art.sha1.clone(), art.size)
        } else {
            let rel = coord.path();
            let base = lib.url.clone().unwrap_or_else(|| MOJANG_LIBRARIES.into());
            let base = if base.ends_with('/') { base } else { format!("{base}/") };
            (rel.clone(), format!("{base}{rel}"), lib.sha1.clone(), lib.size)
        };
        if !seen.insert(rel.clone()) {
            continue;
        }
        if let Some(p) = find_in_extra(&rel, sha1.as_deref(), extra_lib_dirs) {
            plan.classpath.push(p);
            continue;
        }
        let dest = safepath::safe_join(&lib_root, &rel)?;
        if url.is_empty() {
            // Forge-style generated artifact without a URL: must already exist.
            if dest.is_file() {
                plan.classpath.push(dest);
            } else {
                plan.missing.push(lib.name.clone());
            }
            continue;
        }
        let is_native_jar = coord.classifier.as_deref().is_some_and(|c| c.starts_with("natives-"));
        plan.items.push(DownloadItem::new(url, dest.clone()).sha1(sha1.clone()).sha512(lib.sha512.clone()).size(size));
        if is_native_jar {
            plan.natives_jars.push((dest.clone(), vec!["META-INF/".into()]));
        }
        plan.classpath.push(dest);
    }

    // Client jar (Fabric profiles reuse the vanilla jar).
    let jar_id = v.json.jar.clone().unwrap_or_else(|| v.mc_version.clone());
    let client = v.json.downloads.as_ref().and_then(|d| d.get("client")).cloned();
    let client_dest = ctx.paths.version_jar(&jar_id);
    match client {
        Some(c) => plan.items.push(DownloadItem::new(c.url, client_dest.clone()).sha1(c.sha1).size(c.size)),
        None => {
            if !client_dest.exists() {
                if let Some(p) = extra_lib_dirs.iter().filter_map(|d| d.parent()).map(|root| root.join("versions").join(&jar_id).join(format!("{jar_id}.jar"))).find(|p| p.exists()) {
                    plan.client_jar = p.clone();
                    plan.classpath.push(p);
                } else {
                    plan.missing.push(format!("{jar_id}.jar"));
                }
            }
        }
    }
    if plan.client_jar.as_os_str().is_empty() {
        plan.client_jar = client_dest.clone();
        plan.classpath.push(client_dest);
    }

    if let Some(log) = v.json.logging.as_ref().and_then(|l| l.get("client")) {
        let dest = ctx.paths.assets_dir().join("log_configs").join(util::sanitize_file_name(&log.file.id));
        plan.items.push(DownloadItem::new(log.file.url.clone(), dest.clone()).sha1(log.file.sha1.clone()).size(log.file.size));
        plan.logging = Some((dest, log.argument.clone()));
    }
    Ok(plan)
}

/// Download (if necessary) and parse the asset index, returning it with the asset download items.
pub async fn asset_items(ctx: &Ctx, v: &VersionJson) -> Result<Option<(String, AssetIndex, Vec<DownloadItem>)>> {
    let Some(idx) = &v.asset_index else { return Ok(None) };
    let index_path = ctx.paths.assets_dir().join("indexes").join(format!("{}.json", util::sanitize_file_name(&idx.id)));
    fetch_to_file(&ctx.http, &DownloadItem::new(idx.url.clone(), index_path.clone()).sha1(idx.sha1.clone()).size(idx.size)).await?;
    let index: AssetIndex = util::read_json(&index_path)?;
    let objects = ctx.paths.assets_dir().join("objects");
    let mut items = Vec::with_capacity(index.objects.len());
    for obj in index.objects.values() {
        if obj.hash.len() < 3 || !obj.hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::Invalid(format!("bad asset hash {}", obj.hash)));
        }
        let sub = &obj.hash[..2];
        let dest = objects.join(sub).join(&obj.hash);
        items.push(DownloadItem::new(format!("{}/{}/{}", ctx.api.minecraft.resources, sub, obj.hash), dest).sha1(Some(obj.hash.clone())).size(Some(obj.size)));
    }
    Ok(Some((idx.id.clone(), index, items)))
}

/// Result of installing a version.
#[derive(Debug, Clone)]
pub struct Installed {
    pub resolved: ResolvedVersion,
    pub plan: InstallPlan,
    pub natives_dir: PathBuf,
    pub assets_index: Option<String>,
    pub asset_index: Option<AssetIndex>,
    pub virtual_assets_dir: Option<PathBuf>,
}

/// Fast (size-only) check of which items are missing. Full hash checks happen in the
/// download manager, which skips files that verify.
pub fn missing_items(items: &[DownloadItem]) -> Vec<DownloadItem> {
    items
        .iter()
        .filter(|i| match std::fs::metadata(&i.dest) {
            Ok(m) => i.size.is_some_and(|s| s != m.len()) || m.len() == 0,
            Err(_) => true,
        })
        .cloned()
        .collect()
}

/// Install everything needed to launch `resolved`. With `verify_all = false` only missing or
/// wrong-sized files are downloaded (fast path used before each launch).
pub async fn install(ctx: &Ctx, resolved: ResolvedVersion, extra_lib_dirs: &[PathBuf], verify_all: bool) -> Result<Installed> {
    let env = RuleEnv::current();
    let plan = plan(ctx, &resolved, &env, extra_lib_dirs)?;
    if !plan.missing.is_empty() {
        return Err(Error::NotFound(format!(
            "These files are required but cannot be downloaded automatically: {}. Re-run the loader's official installer for this game folder.",
            plan.missing.join(", ")
        )));
    }
    let assets = asset_items(ctx, &resolved.json).await?;
    let mut items = plan.items.clone();
    if let Some((_, _, a)) = &assets {
        items.extend(a.iter().cloned());
    }
    let items = if verify_all { items } else { missing_items(&items) };
    if !items.is_empty() {
        let title = format!("Minecraft {}{}", resolved.mc_version, match resolved.loader {
            LoaderKind::Vanilla => String::new(),
            l => format!(" ({} {})", l.as_str(), resolved.loader_version.clone().unwrap_or_default()),
        });
        ctx.downloads.run(&title, items).await?;
    }

    let natives_dir = ctx.paths.natives_root().join(util::sanitize_file_name(&resolved.id));
    let jars = plan.natives_jars.clone();
    let nd = natives_dir.clone();
    tokio::task::spawn_blocking(move || extract_natives(&jars, &nd)).await.map_err(|e| Error::msg(e.to_string()))??;

    let (assets_index, asset_index, virtual_dir) = match assets {
        Some((id, index, _)) => {
            let vdir = if index.is_virtual {
                let dir = ctx.paths.assets_dir().join("virtual").join(util::sanitize_file_name(&id));
                let objects = ctx.paths.assets_dir().join("objects");
                let idx = index.clone();
                let d2 = dir.clone();
                tokio::task::spawn_blocking(move || copy_assets_to(&idx, &objects, &d2)).await.map_err(|e| Error::msg(e.to_string()))??;
                Some(dir)
            } else {
                None
            };
            (Some(id), Some(index), vdir)
        }
        None => (None, None, None),
    };
    Ok(Installed { resolved, plan, natives_dir, assets_index, asset_index, virtual_assets_dir: virtual_dir })
}

/// Extract DLLs/resources from natives jars, honouring exclude lists and path safety.
pub fn extract_natives(jars: &[(PathBuf, Vec<String>)], dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for (jar, excludes) in jars {
        let file = std::fs::File::open(jar)?;
        let mut zip = zip::ZipArchive::new(file)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name()?.to_string();
            if name.starts_with("META-INF/") || excludes.iter().any(|e| name.starts_with(e.as_str())) {
                continue;
            }
            // Only native libraries and their companion files; never class files.
            if name.ends_with(".class") {
                continue;
            }
            let out = safepath::safe_join(dest, &name)?;
            if let Ok(m) = std::fs::metadata(&out) {
                if m.len() == entry.size() {
                    continue;
                }
            }
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut f)?;
        }
    }
    Ok(())
}

/// Copy hashed asset objects to their named layout (legacy `virtual` and `map_to_resources`).
pub fn copy_assets_to(index: &AssetIndex, objects: &Path, dest: &Path) -> Result<u64> {
    let mut n = 0;
    for (name, obj) in &index.objects {
        let target = safepath::safe_join(dest, name)?;
        if target.exists() {
            continue;
        }
        let src = objects.join(&obj.hash[..2]).join(&obj.hash);
        if let Some(p) = target.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(&src, &target)?;
        n += 1;
    }
    Ok(n)
}

/// Summary of a version for the Versions page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    pub kind: String,
    pub release_time: String,
    pub installed: bool,
}

pub fn summarize(ctx: &Ctx, m: &VersionManifest, include_snapshots: bool, include_old: bool) -> Vec<VersionSummary> {
    m.versions
        .iter()
        .filter(|v| match v.kind.as_str() {
            "release" => true,
            "snapshot" => include_snapshots,
            "old_beta" | "old_alpha" => include_old,
            _ => false,
        })
        .map(|v| VersionSummary {
            id: v.id.clone(),
            kind: v.kind.clone(),
            release_time: v.release_time.clone(),
            installed: ctx.paths.version_jar(&v.id).exists(),
        })
        .collect()
}
