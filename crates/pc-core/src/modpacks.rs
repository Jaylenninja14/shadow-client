//! Modpacks: Modrinth `.mrpack` and CurseForge export zips.
//!
//! Safety: every path in an index or archive goes through `safe_join`; Modrinth pack files may
//! only come from the hosts the format allows; CurseForge files that the author does not allow
//! third-party apps to download are listed for manual download instead of being fetched.
//! Packs always install into a new isolated profile; updates back up first and keep worlds
//! and personal settings (options.txt, servers.dat).

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::download::DownloadItem;
use crate::minecraft::loaders::LoaderKind;
use crate::mods::curseforge as cf;
use crate::mods::index::{IndexEntry, ModIndex};
use crate::mods::ContentKind;
use crate::profiles::{NewProfile, Profile, ProfileKind, Profiles};
use crate::{safepath, util, Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    pub files: Vec<MrFile>,
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrFile {
    pub path: String,
    pub hashes: BTreeMap<String, String>,
    #[serde(default)]
    pub env: Option<BTreeMap<String, String>>,
    pub downloads: Vec<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackSummary {
    pub format: String,
    pub name: String,
    pub version: String,
    pub summary: Option<String>,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
    pub file_count: usize,
    pub override_count: usize,
    pub total_size: Option<u64>,
    pub warnings: Vec<String>,
    pub supported: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInstallReport {
    pub profile_id: String,
    pub installed_files: usize,
    pub overrides: usize,
    pub skipped: Vec<String>,
    /// Files the user must download manually (CurseForge opt-out), with project pages
    pub manual: Vec<ManualFile>,
    pub warnings: Vec<String>,
    pub backup: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualFile {
    pub name: String,
    pub file_name: String,
    pub url: String,
    pub target_folder: String,
}

fn loader_from_deps(deps: &BTreeMap<String, String>) -> (LoaderKind, Option<String>) {
    for (k, kind) in [("fabric-loader", LoaderKind::Fabric), ("quilt-loader", LoaderKind::Quilt), ("neoforge", LoaderKind::Neoforge), ("forge", LoaderKind::Forge)] {
        if let Some(v) = deps.get(k) {
            return (kind, Some(v.clone()));
        }
    }
    (LoaderKind::Vanilla, None)
}

fn open_zip(path: &Path) -> Result<zip::ZipArchive<std::fs::File>> {
    Ok(zip::ZipArchive::new(std::fs::File::open(path)?)?)
}

fn read_json_entry<T: serde::de::DeserializeOwned>(z: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Result<Option<T>> {
    let Ok(e) = z.by_name(name) else { return Ok(None) };
    let mut buf = vec![];
    e.take(32 * 1024 * 1024).read_to_end(&mut buf)?;
    Ok(Some(serde_json::from_slice(&buf)?))
}

fn count_prefix(z: &mut zip::ZipArchive<std::fs::File>, prefixes: &[&str]) -> usize {
    (0..z.len()).filter_map(|i| z.by_index(i).ok().and_then(|e| e.name().ok().map(|n| n.to_string()))).filter(|n| prefixes.iter().any(|p| n.starts_with(p)) && !n.ends_with('/')).count()
}

/// Read a pack file and describe it without installing anything.
pub fn inspect(path: &Path) -> Result<PackSummary> {
    let mut z = open_zip(path)?;
    if let Some(idx) = read_json_entry::<MrIndex>(&mut z, "modrinth.index.json")? {
        let mut warnings = vec![];
        if idx.game != "minecraft" {
            return Err(Error::Invalid(format!("This pack is for {}, not Minecraft", idx.game)));
        }
        let mc = idx.dependencies.get("minecraft").cloned().ok_or_else(|| Error::Invalid("The pack does not say which Minecraft version it needs".into()))?;
        let (loader, lv) = loader_from_deps(&idx.dependencies);
        let bad_hosts = idx.files.iter().filter(|f| !f.downloads.iter().any(|u| safepath::is_allowed_mrpack_host(u))).count();
        if bad_hosts > 0 {
            warnings.push(format!("{bad_hosts} file(s) use download hosts the Modrinth format does not allow; they will be skipped."));
        }
        let supported = loader.installable();
        if !supported {
            warnings.push(format!("This pack needs {}, which Shadow Client cannot install. Use the {} installer or another launcher for this pack.", loader.as_str(), loader.as_str()));
        }
        return Ok(PackSummary {
            format: "mrpack".into(),
            name: idx.name.clone(),
            version: idx.version_id.clone(),
            summary: idx.summary.clone(),
            mc_version: mc,
            loader,
            loader_version: lv,
            file_count: idx.files.len(),
            override_count: count_prefix(&mut z, &["overrides/", "client-overrides/"]),
            total_size: idx.files.iter().map(|f| f.file_size).sum(),
            warnings,
            supported,
        });
    }
    if let Some(m) = read_json_entry::<serde_json::Value>(&mut z, "manifest.json")? {
        if m["manifestType"] != "minecraftModpack" {
            return Err(Error::Invalid("manifest.json is not a CurseForge modpack manifest".into()));
        }
        let mc = m["minecraft"]["version"].as_str().ok_or_else(|| Error::Invalid("manifest has no Minecraft version".into()))?.to_string();
        let primary = m["minecraft"]["modLoaders"].as_array().and_then(|a| a.iter().find(|l| l["primary"] == true).or(a.first())).and_then(|l| l["id"].as_str()).unwrap_or("");
        let (lname, lver) = primary.split_once('-').unwrap_or((primary, ""));
        let loader = LoaderKind::parse(lname).unwrap_or(LoaderKind::Vanilla);
        let overrides = m["overrides"].as_str().unwrap_or("overrides").to_string();
        let supported = loader.installable();
        let mut warnings = vec![];
        if !supported {
            warnings.push(format!("This pack needs {}, which Shadow Client cannot install.", loader.as_str()));
        }
        return Ok(PackSummary {
            format: "curseforge".into(),
            name: m["name"].as_str().unwrap_or("CurseForge pack").into(),
            version: m["version"].as_str().unwrap_or("").into(),
            summary: m["author"].as_str().map(|a| format!("by {a}")),
            mc_version: mc,
            loader,
            loader_version: (!lver.is_empty()).then(|| lver.to_string()),
            file_count: m["files"].as_array().map(|a| a.len()).unwrap_or(0),
            override_count: count_prefix(&mut z, &[&format!("{overrides}/")]),
            total_size: None,
            warnings,
            supported,
        });
    }
    Err(Error::Invalid("Not a modpack: no modrinth.index.json or manifest.json inside".into()))
}

/// Extract `prefix/…` entries into `dest`, skipping `skip_existing` names that already exist.
fn extract_overrides(z: &mut zip::ZipArchive<std::fs::File>, prefix: &str, dest: &Path, keep_existing: &[&str]) -> Result<(usize, Vec<String>)> {
    let mut n = 0;
    let mut skipped = vec![];
    for i in 0..z.len() {
        let mut e = z.by_index(i)?;
        let name = e.name()?.to_string();
        let Some(rel) = name.strip_prefix(prefix) else { continue };
        if rel.is_empty() || e.is_dir() {
            continue;
        }
        let out = match safepath::safe_join(dest, rel) {
            Ok(p) => p,
            Err(err) => {
                skipped.push(format!("{name}: {err}"));
                continue;
            }
        };
        if keep_existing.iter().any(|k| rel.eq_ignore_ascii_case(k)) && out.exists() {
            skipped.push(format!("{rel} (kept your existing file)"));
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut f = std::fs::File::create(&out)?;
        std::io::copy(&mut e, &mut f)?;
        n += 1;
    }
    Ok((n, skipped))
}

fn kind_for_path(rel: &str) -> ContentKind {
    if rel.starts_with("resourcepacks/") {
        ContentKind::Resourcepack
    } else if rel.starts_with("shaderpacks/") {
        ContentKind::Shader
    } else {
        ContentKind::Mod
    }
}

/// Personal files a pack update must not overwrite.
const PERSONAL: &[&str] = &["options.txt", "servers.dat", "optionsof.txt", "optionsshaders.txt"];

/// Install a `.mrpack` into `game_dir`. Returns (files, overrides, skipped, warnings).
async fn apply_mrpack(ctx: &Ctx, pack: &Path, game_dir: &Path, data_dir: &Path, keep_personal: bool, previous: Option<&ModIndex>) -> Result<PackInstallReport> {
    let mut z = open_zip(pack)?;
    let idx: MrIndex = read_json_entry(&mut z, "modrinth.index.json")?.ok_or_else(|| Error::Invalid("modrinth.index.json missing".into()))?;
    if idx.format_version != 1 {
        return Err(Error::Invalid(format!("Unsupported .mrpack format version {}", idx.format_version)));
    }
    let mut report = PackInstallReport::default();
    let mut items = vec![];
    let mut entries = vec![];
    for f in &idx.files {
        if f.env.as_ref().and_then(|e| e.get("client")).is_some_and(|c| c == "unsupported") {
            report.skipped.push(format!("{} (server-only)", f.path));
            continue;
        }
        let dest = match safepath::safe_join(game_dir, &f.path) {
            Ok(d) => d,
            Err(e) => {
                report.skipped.push(format!("{}: {e}", f.path));
                continue;
            }
        };
        let Some(url) = f.downloads.iter().find(|u| safepath::is_allowed_mrpack_host(u)) else {
            report.skipped.push(format!("{} (download host not allowed)", f.path));
            continue;
        };
        let sha1 = f.hashes.get("sha1").cloned();
        let sha512 = f.hashes.get("sha512").cloned();
        if sha1.is_none() && sha512.is_none() {
            report.skipped.push(format!("{} (no hash to verify)", f.path));
            continue;
        }
        if f.env.as_ref().and_then(|e| e.get("client")).is_some_and(|c| c == "optional") {
            report.warnings.push(format!("{} is optional in this pack and was included.", f.path));
        }
        items.push(DownloadItem::new(url.clone(), dest.clone()).sha1(sha1.clone()).sha512(sha512.clone()).size(f.file_size));
        let file_name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        entries.push(IndexEntry {
            file_name,
            kind: kind_for_path(&f.path.replace('\\', "/")),
            sha1: sha1.unwrap_or_default(),
            sha512,
            source: "modpack".into(),
            explicit: true,
            installed_at: util::now_iso(),
            ..Default::default()
        });
    }
    // Remove files the previous pack version installed but the new one doesn't (with backup).
    if let Some(prev) = previous {
        let ts = util::timestamp_slug();
        for e in prev.entries.iter().filter(|e| e.source == "modpack") {
            if !entries.iter().any(|n| n.file_name == e.file_name && n.kind == e.kind) {
                let p = game_dir.join(e.kind.folder()).join(&e.file_name);
                if p.is_file() {
                    let b = data_dir.join("backups").join(e.kind.folder()).join(&ts).join(&e.file_name);
                    util::move_path(&p, &b)?;
                    report.warnings.push(format!("Removed {} (no longer in the pack; backed up)", e.file_name));
                }
            }
        }
    }
    report.installed_files = items.len();
    if !items.is_empty() {
        ctx.downloads.run(&format!("Modpack {} {}", idx.name, idx.version_id), items).await?;
    }
    let keep: &[&str] = if keep_personal { PERSONAL } else { &[] };
    let (a, s1) = extract_overrides(&mut z, "overrides/", game_dir, keep)?;
    let (b, s2) = extract_overrides(&mut z, "client-overrides/", game_dir, keep)?;
    report.overrides = a + b;
    report.skipped.extend(s1.into_iter().chain(s2));
    let mut index = ModIndex::load(data_dir);
    index.entries.retain(|e| e.source != "modpack");
    for e in entries {
        index.upsert(e);
    }
    index.save(data_dir)?;
    Ok(report)
}

async fn apply_curseforge(ctx: &Ctx, pack: &Path, game_dir: &Path, data_dir: &Path, keep_personal: bool) -> Result<PackInstallReport> {
    let mut z = open_zip(pack)?;
    let m: serde_json::Value = read_json_entry(&mut z, "manifest.json")?.ok_or_else(|| Error::Invalid("manifest.json missing".into()))?;
    let mut report = PackInstallReport::default();
    let file_ids: Vec<u64> = m["files"].as_array().map(|a| a.iter().filter(|f| f["required"] != false).filter_map(|f| f["fileID"].as_u64()).collect()).unwrap_or_default();
    if !file_ids.is_empty() && !cf::has_key(ctx) {
        return Err(Error::MissingConfig("This CurseForge pack lists its mods by ID, so a CurseForge API key is needed to download them. Add one under Settings → Accounts & services.".into()));
    }
    let files = cf::get_files(ctx, &file_ids).await?;
    let mod_ids: Vec<u64> = files.iter().map(|f| f.mod_id).collect();
    let mods = cf::get_mods(ctx, &mod_ids).await.unwrap_or_default();
    let mut items = vec![];
    let mut entries = vec![];
    for f in &files {
        let md = mods.iter().find(|x| x.id == f.mod_id);
        let folder = match md.and_then(|x| x.class_id) {
            Some(cf::CLASS_RESOURCEPACKS) => "resourcepacks",
            Some(cf::CLASS_SHADERS) => "shaderpacks",
            _ => "mods",
        };
        let segs = safepath::validate_relative(&f.file_name);
        if segs.as_ref().map(|s| s.len() != 1).unwrap_or(true) {
            report.skipped.push(format!("{}: unsafe file name", f.file_name));
            continue;
        }
        let dest = game_dir.join(folder).join(&f.file_name);
        match &f.download_url {
            Some(url) => {
                items.push(DownloadItem::new(url.clone(), dest).sha1(f.sha1()).size(f.file_length));
                entries.push(IndexEntry {
                    file_name: f.file_name.clone(),
                    kind: kind_for_path(&format!("{folder}/")),
                    sha1: f.sha1().unwrap_or_default(),
                    source: "modpack".into(),
                    project_id: Some(f.mod_id.to_string()),
                    version_id: Some(f.id.to_string()),
                    title: md.map(|x| x.name.clone()),
                    website_url: md.map(cf::project_url),
                    installed_at: util::now_iso(),
                    ..Default::default()
                });
            }
            None => report.manual.push(ManualFile {
                name: md.map(|x| x.name.clone()).unwrap_or_else(|| f.display_name.clone()),
                file_name: f.file_name.clone(),
                url: md.map(|x| format!("{}/files/{}", cf::project_url(x), f.id)).unwrap_or_else(|| format!("https://www.curseforge.com/projects/{}", f.mod_id)),
                target_folder: game_dir.join(folder).to_string_lossy().into_owned(),
            }),
        }
    }
    report.installed_files = items.len();
    if !items.is_empty() {
        ctx.downloads.run(&format!("Modpack {}", m["name"].as_str().unwrap_or("")), items).await?;
    }
    let overrides = format!("{}/", m["overrides"].as_str().unwrap_or("overrides"));
    let keep: &[&str] = if keep_personal { PERSONAL } else { &[] };
    let (n, skipped) = extract_overrides(&mut z, &overrides, game_dir, keep)?;
    report.overrides = n;
    report.skipped.extend(skipped);
    let mut index = ModIndex::load(data_dir);
    index.entries.retain(|e| e.source != "modpack");
    for e in entries {
        index.upsert(e);
    }
    index.save(data_dir)?;
    Ok(report)
}

/// Install a pack file as a new isolated profile.
pub async fn install(ctx: &Ctx, pack: &Path, name: Option<String>) -> Result<(Profile, PackInstallReport)> {
    let info = inspect(pack)?;
    if !info.supported {
        return Err(Error::Incompatible(info.warnings.join(" ")));
    }
    let profiles = Profiles::new(&ctx.paths);
    let profile = profiles.create(NewProfile {
        name: name.unwrap_or_else(|| format!("{} {}", info.name, info.version).trim().chars().take(48).collect()),
        kind: ProfileKind::Modded,
        mc_version: info.mc_version.clone(),
        loader: info.loader,
        loader_version: info.loader_version.clone(),
        game_dir: None,
        preset: None,
        builtin_mods: Some(false),
        ingame_features: None,
    })?;
    let game_dir = profiles.game_dir(&profile);
    let data_dir = ctx.paths.profile_data_dir(&profile.id);
    let result = match info.format.as_str() {
        "mrpack" => apply_mrpack(ctx, pack, &game_dir, &data_dir, false, None).await,
        _ => apply_curseforge(ctx, pack, &game_dir, &data_dir, false).await,
    };
    match result {
        Ok(mut r) => {
            r.profile_id = profile.id.clone();
            r.warnings.extend(info.warnings);
            let mut p = profile.clone();
            p.notes = Some(format!("Modpack: {} {} ({})", info.name, info.version, info.format));
            profiles.save(&p)?;
            Ok((p, r))
        }
        Err(e) => {
            // Leave nothing half-installed behind: remove the new (empty-world) profile.
            let _ = Profiles::without_recycle_bin(&ctx.paths).delete(&profile.id);
            Err(e)
        }
    }
}

/// Update an existing pack profile from a newer pack file. Backs up mods/config first and
/// keeps worlds plus personal settings.
pub async fn update(ctx: &Ctx, profile_id: &str, pack: &Path) -> Result<PackInstallReport> {
    let info = inspect(pack)?;
    let profiles = Profiles::new(&ctx.paths);
    let mut profile = profiles.get(profile_id)?;
    if info.mc_version != profile.mc_version || info.loader != profile.loader {
        return Err(Error::Incompatible(format!(
            "The new pack version uses Minecraft {} with {}, but this profile is {} with {}. Install it as a new profile instead so your worlds stay on the version they were made with.",
            info.mc_version,
            info.loader.as_str(),
            profile.mc_version,
            profile.loader.as_str()
        )));
    }
    let game_dir = profiles.game_dir(&profile);
    let data_dir = ctx.paths.profile_data_dir(profile_id);
    let backup = backup_instance(&game_dir, &data_dir)?;
    let previous = ModIndex::load(&data_dir);
    let mut report = match info.format.as_str() {
        "mrpack" => apply_mrpack(ctx, pack, &game_dir, &data_dir, true, Some(&previous)).await?,
        _ => apply_curseforge(ctx, pack, &game_dir, &data_dir, true).await?,
    };
    report.profile_id = profile_id.into();
    report.backup = Some(backup);
    profile.loader_version = info.loader_version.clone().or(profile.loader_version);
    profile.notes = Some(format!("Modpack: {} {} ({})", info.name, info.version, info.format));
    profiles.save(&profile)?;
    Ok(report)
}

/// Zip mods/, config/ and options.txt (not worlds) into the profile's backup folder.
pub fn backup_instance(game_dir: &Path, data_dir: &Path) -> Result<PathBuf> {
    let dest = data_dir.join("backups").join(format!("instance-{}.zip", util::timestamp_slug()));
    std::fs::create_dir_all(dest.parent().unwrap())?;
    let mut zw = zip::ZipWriter::new(std::fs::File::create(&dest)?);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    fn add_dir(zw: &mut zip::ZipWriter<std::fs::File>, base: &Path, dir: &Path, opts: zip::write::SimpleFileOptions) -> Result<()> {
        for e in std::fs::read_dir(dir)?.flatten() {
            let p = e.path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
            if p.is_dir() {
                add_dir(zw, base, &p, opts)?;
            } else {
                zw.start_file(rel, opts)?;
                let mut f = std::fs::File::open(&p)?;
                std::io::copy(&mut f, zw)?;
            }
        }
        Ok(())
    }
    for sub in ["mods", "config", "resourcepacks", "shaderpacks"] {
        let d = game_dir.join(sub);
        if d.is_dir() {
            add_dir(&mut zw, game_dir, &d, opts)?;
        }
    }
    for f in ["options.txt", "servers.dat"] {
        let p = game_dir.join(f);
        if p.is_file() {
            zw.start_file(f, opts)?;
            zw.write_all(&std::fs::read(&p)?)?;
        }
    }
    zw.finish()?;
    Ok(dest)
}

/// Export a profile as a `.mrpack`. Mods known to come from Modrinth are referenced by URL and
/// hash; other mods are included as overrides (for personal use). Worlds are not included.
pub fn export_mrpack(ctx: &Ctx, profile_id: &str, dest: &Path, include_options: bool) -> Result<PathBuf> {
    let profiles = Profiles::new(&ctx.paths);
    let p = profiles.get(profile_id)?;
    let game_dir = profiles.game_dir(&p);
    let index = ModIndex::load(&ctx.paths.profile_data_dir(profile_id));
    let mut files = vec![];
    let mut overrides: Vec<(String, PathBuf)> = vec![];
    for kind in [ContentKind::Mod, ContentKind::Resourcepack, ContentKind::Shader] {
        for e in std::fs::read_dir(game_dir.join(kind.folder())).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !e.path().is_file() || name.ends_with(".disabled") {
                continue;
            }
            let entry = index.by_file(kind, &name);
            let url = entry.and_then(|x| x.version_id.as_ref().zip(x.project_id.as_ref())).filter(|_| entry.is_some_and(|x| x.source == "modrinth" || x.source == "builtin")).map(|(v, pid)| {
                format!("https://cdn.modrinth.com/data/{pid}/versions/{v}/{}", url::form_urlencoded::byte_serialize(name.as_bytes()).collect::<String>().replace('+', "%20"))
            });
            match url {
                Some(u) => {
                    let h = util::hash_file(&e.path())?;
                    files.push(serde_json::json!({
                        "path": format!("{}/{}", kind.folder(), name),
                        "hashes": {"sha1": h.sha1, "sha512": h.sha512},
                        "env": {"client": "required", "server": "optional"},
                        "downloads": [u],
                        "fileSize": h.size,
                    }));
                }
                None => overrides.push((format!("overrides/{}/{}", kind.folder(), name), e.path())),
            }
        }
    }
    let mut deps = serde_json::Map::new();
    deps.insert("minecraft".into(), p.mc_version.clone().into());
    match (p.loader, &p.loader_version) {
        (LoaderKind::Fabric, Some(v)) => {
            deps.insert("fabric-loader".into(), v.clone().into());
        }
        (LoaderKind::Quilt, Some(v)) => {
            deps.insert("quilt-loader".into(), v.clone().into());
        }
        _ => {}
    }
    let index_json = serde_json::json!({
        "formatVersion": 1, "game": "minecraft", "versionId": "1.0.0", "name": p.name,
        "summary": "Exported from Shadow Client", "files": files, "dependencies": deps,
    });
    std::fs::create_dir_all(dest.parent().unwrap_or(Path::new(".")))?;
    let mut zw = zip::ZipWriter::new(std::fs::File::create(dest)?);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zw.start_file("modrinth.index.json", opts)?;
    zw.write_all(serde_json::to_vec_pretty(&index_json)?.as_slice())?;
    // Profile settings for Shadow Client imports (other launchers ignore this file).
    let mut settings = p.clone();
    settings.game_dir = None;
    settings.external_root = None;
    settings.java_path = None;
    zw.start_file("shadowclient-profile.json", opts)?;
    zw.write_all(serde_json::to_vec_pretty(&settings)?.as_slice())?;
    for (rel, path) in overrides {
        zw.start_file(rel, opts)?;
        zw.write_all(&std::fs::read(path)?)?;
    }
    let cfg = game_dir.join("config");
    if cfg.is_dir() {
        fn walk(zw: &mut zip::ZipWriter<std::fs::File>, base: &Path, dir: &Path, opts: zip::write::SimpleFileOptions) -> Result<()> {
            for e in std::fs::read_dir(dir)?.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(zw, base, &p, opts)?;
                } else {
                    let rel = p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
                    zw.start_file(format!("overrides/{rel}"), opts)?;
                    zw.write_all(&std::fs::read(&p)?)?;
                }
            }
            Ok(())
        }
        walk(&mut zw, &game_dir, &cfg, opts)?;
    }
    if include_options && game_dir.join("options.txt").is_file() {
        zw.start_file("overrides/options.txt", opts)?;
        zw.write_all(&std::fs::read(game_dir.join("options.txt"))?)?;
    }
    zw.finish()?;
    Ok(dest.to_path_buf())
}

/// Read Shadow Client profile settings from an exported pack, if present.
pub fn embedded_profile_settings(pack: &Path) -> Option<Profile> {
    let mut z = open_zip(pack).ok()?;
    // exports made before the rename use the old name
    ["shadowclient-profile.json", "sunnyclient-profile.json"].iter().find_map(|n| read_json_entry::<Profile>(&mut z, n).ok().flatten())
}
