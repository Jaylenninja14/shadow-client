//! Minecraft Bedrock Edition on Windows: find it, start it, and manage its resource and
//! behavior packs (add-ons), including ones downloaded from CurseForge.
//!
//! Bedrock is a Microsoft Store / Xbox app, so Windows starts it (Shadow Client can't mod it or
//! change its settings). Packs are plain folders under
//! `%APPDATA%\Minecraft Bedrock\Users\Shared\games\com.mojang\{resource,behavior,skin}_packs`
//! (older UWP builds: `%LOCALAPPDATA%\Packages\<family>\LocalState\games\com.mojang`). Imports
//! only add folders; removing or replacing a pack moves the old folder to Shadow's backups.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{safepath, util, Error, Result};

pub const PACKAGE_FAMILY: &str = "Microsoft.MinecraftUWP_8wekyb3d8bbwe";
/// Application id from the package manifest (GDK builds start through GameLaunchHelper.exe).
pub const APP_ID: &str = "Game";
pub const STORE_URL: &str = "https://www.microsoft.com/store/productId/9NBLGGH2JHXJ";
pub const CURSEFORGE_GAME_ID: u32 = 78022;

/// Largest pack we extract (uncompressed), to stay safe from zip bombs.
const MAX_UNPACKED: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockInfo {
    pub installed: bool,
    pub version: Option<String>,
    /// `games/com.mojang`, where packs and worlds live
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockPack {
    /// "resource" | "behavior" | "skin"
    pub kind: String,
    pub name: String,
    pub description: String,
    pub uuid: String,
    pub version: String,
    /// folder name inside the kind's packs folder
    pub folder: String,
    pub development: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub installed: Vec<String>,
    pub replaced: Vec<String>,
    pub skipped: Vec<String>,
    /// Worlds and templates are handed to Bedrock's own importer
    pub opened_in_bedrock: bool,
}

// ------------------------------------------------------------------ detection & launch

/// Is Bedrock installed (and which version), and where are its packs?
pub fn detect() -> BedrockInfo {
    let version = installed_version();
    BedrockInfo { installed: version.is_some(), version, data_dir: default_data_dir() }
}

#[cfg(windows)]
fn installed_version() -> Option<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    let key = winreg::RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages")
        .ok()?;
    key.enum_keys().filter_map(|k| k.ok()).filter_map(|name| version_from_package_name(&name)).max_by(|a, b| crate::mods::metadata::compare_versions(a, b).unwrap_or(std::cmp::Ordering::Equal))
}

#[cfg(not(windows))]
fn installed_version() -> Option<String> {
    None
}

/// "Microsoft.MinecraftUWP_1.26.6005.0_x64__8wekyb3d8bbwe" → "1.26.6005.0"
fn version_from_package_name(name: &str) -> Option<String> {
    let rest = name.strip_prefix("Microsoft.MinecraftUWP_")?;
    if !name.ends_with("__8wekyb3d8bbwe") {
        return None;
    }
    let version = rest.split('_').next()?;
    version.chars().all(|c| c.is_ascii_digit() || c == '.').then(|| version.to_string())
}

fn default_data_dir() -> Option<PathBuf> {
    let gdk = std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Minecraft Bedrock").join("Users").join("Shared").join("games").join("com.mojang"));
    let uwp = std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("Packages").join(PACKAGE_FAMILY).join("LocalState").join("games").join("com.mojang"));
    [gdk, uwp].into_iter().flatten().find(|p| p.is_dir())
}

/// Start Bedrock the way the Start menu does.
pub fn launch() -> Result<()> {
    if installed_version().is_none() {
        return Err(Error::NotFound("Minecraft Bedrock isn't installed. Get it from the Microsoft Store, then try again.".into()));
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(format!(r"shell:AppsFolder\{PACKAGE_FAMILY}!{APP_ID}"))
            .spawn()
            .map_err(|e| Error::msg(format!("Could not start Minecraft Bedrock: {e}")))?;
        Ok(())
    }
    #[cfg(not(windows))]
    Err(Error::Incompatible("Minecraft Bedrock can only be started on Windows.".into()))
}

/// Hand a world (.mcworld) or template to Bedrock's own importer (it opens the game).
fn open_with_bedrock(file: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe").arg(file).spawn().map_err(|e| Error::msg(format!("Could not open {}: {e}", file.display())))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = file;
        Err(Error::Incompatible("Minecraft Bedrock can only be used on Windows.".into()))
    }
}

// ------------------------------------------------------------------ packs

fn kind_dir(kind: &str) -> Option<&'static str> {
    match kind {
        "resource" => Some("resource_packs"),
        "behavior" => Some("behavior_packs"),
        "skin" => Some("skin_packs"),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct Manifest {
    header: Header,
    #[serde(default)]
    modules: Vec<ManifestModule>,
}

#[derive(Debug, Deserialize)]
struct Header {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    uuid: String,
    #[serde(default)]
    version: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ManifestModule {
    #[serde(rename = "type", default)]
    kind: String,
}

/// Bedrock manifests are often JSON with comments and a BOM.
fn parse_manifest(text: &str) -> Option<Manifest> {
    let text = text.trim_start_matches('\u{feff}');
    serde_json::from_str(text).ok().or_else(|| serde_json::from_str(&strip_comments(text)).ok())
}

fn strip_comments(s: &str) -> String {
    let (mut out, mut chars, mut in_str) = (String::with_capacity(s.len()), s.chars().peekable(), false);
    while let Some(c) = chars.next() {
        if in_str {
            out.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if c == '/' && chars.peek() == Some(&'/') {
            for n in chars.by_ref() {
                if n == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut prev = ' ';
            for n in chars.by_ref() {
                if prev == '*' && n == '/' {
                    break;
                }
                prev = n;
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn manifest_kind(m: &Manifest) -> Option<&'static str> {
    let types: Vec<&str> = m.modules.iter().map(|x| x.kind.as_str()).collect();
    if types.contains(&"resources") {
        Some("resource")
    } else if types.iter().any(|t| matches!(*t, "data" | "script" | "javascript" | "client_data")) {
        Some("behavior")
    } else if types.contains(&"skin_pack") {
        Some("skin")
    } else {
        None
    }
}

fn version_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Array(a) => a.iter().map(|x| x.to_string().trim_matches('"').to_string()).collect::<Vec<_>>().join("."),
        serde_json::Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

/// "pack.name" style keys are resolved from texts/en_US.lang when present.
fn localized(value: &str, lang: Option<&str>) -> String {
    let looks_like_key = !value.contains(' ') && value.contains('.') && value.starts_with("pack.");
    let resolved = looks_like_key
        .then(|| lang.and_then(|l| l.lines().find_map(|line| line.split_once('=').filter(|(k, _)| k.trim() == value).map(|(_, v)| v.split('#').next().unwrap_or(v).trim().to_string()))))
        .flatten();
    let value = resolved.as_deref().unwrap_or(value);
    // strip Minecraft § formatting codes
    let mut out = String::new();
    let mut skip = false;
    for c in value.chars() {
        if skip {
            skip = false;
        } else if c == '§' {
            skip = true;
        } else {
            out.push(c);
        }
    }
    out
}

/// The folder that actually holds a pack's manifest: the folder itself, or (as Bedrock's own
/// importer sometimes leaves it) its only subfolder. Old packs use pack_manifest.json.
fn manifest_in(dir: &Path) -> Option<(PathBuf, PathBuf)> {
    for name in ["manifest.json", "pack_manifest.json"] {
        if dir.join(name).is_file() {
            return Some((dir.to_path_buf(), dir.join(name)));
        }
    }
    let subs: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    match subs.as_slice() {
        [only] => ["manifest.json", "pack_manifest.json"].iter().map(|n| only.join(n)).find(|p| p.is_file()).map(|m| (only.clone(), m)),
        _ => None,
    }
}

fn read_pack_dir(dir: &Path, kind: &str, development: bool) -> Option<BedrockPack> {
    let (root, manifest) = manifest_in(dir)?;
    let m = parse_manifest(&std::fs::read_to_string(manifest).ok()?)?;
    let lang = std::fs::read_to_string(root.join("texts").join("en_US.lang")).ok();
    Some(BedrockPack {
        kind: kind.into(),
        name: localized(&m.header.name, lang.as_deref()),
        description: localized(&m.header.description, lang.as_deref()),
        uuid: m.header.uuid.to_lowercase(),
        version: version_string(&m.header.version),
        folder: dir.file_name()?.to_string_lossy().into_owned(),
        development,
    })
}

/// Every pack in the data dir (installed and development folders).
pub fn list_packs(data_dir: &Path) -> Vec<BedrockPack> {
    let mut out = vec![];
    for kind in ["resource", "behavior", "skin"] {
        let base = kind_dir(kind).expect("known kind");
        for (folder, development) in [(base.to_string(), false), (format!("development_{base}"), true)] {
            let Ok(rd) = std::fs::read_dir(data_dir.join(folder)) else { continue };
            for e in rd.flatten() {
                if e.path().is_dir() {
                    if let Some(p) = read_pack_dir(&e.path(), kind, development) {
                        out.push(p);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

/// Move one pack folder into Shadow's backups.
pub fn remove_pack(data_dir: &Path, backups: &Path, kind: &str, folder: &str, development: bool) -> Result<PathBuf> {
    let base = kind_dir(kind).ok_or_else(|| Error::Invalid(format!("Unknown pack type {kind}")))?;
    let parent = data_dir.join(if development { format!("development_{base}") } else { base.to_string() });
    let path = safepath::safe_join(&parent, folder)?;
    if manifest_in(&path).is_none() {
        return Err(Error::NotFound(folder.into()));
    }
    let dest = util::unique_path(&backups.join("bedrock").join(util::timestamp_slug()).join(base).join(folder));
    util::move_path(&path, &dest)?;
    Ok(dest)
}

// ------------------------------------------------------------------ import

/// Install a .mcpack / .mcaddon / .zip into the data dir. Worlds (.mcworld) and templates
/// (.mctemplate) are opened with Bedrock's own importer instead.
pub fn import_file(data_dir: &Path, backups: &Path, file: &Path) -> Result<ImportReport> {
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if matches!(ext.as_str(), "mcworld" | "mctemplate") {
        open_with_bedrock(file)?;
        return Ok(ImportReport { opened_in_bedrock: true, ..Default::default() });
    }
    if !matches!(ext.as_str(), "mcpack" | "mcaddon" | "zip") {
        return Err(Error::Invalid("Pick a .mcpack, .mcaddon, .zip, .mcworld or .mctemplate file.".into()));
    }
    let bytes = std::fs::read(file)?;
    let mut report = ImportReport::default();
    import_zip(data_dir, backups, &bytes, &mut report, 0)?;
    if report.installed.is_empty() && report.replaced.is_empty() {
        return Err(Error::Invalid(if report.skipped.is_empty() {
            "No Bedrock pack (manifest.json) was found in that file.".into()
        } else {
            format!("Nothing could be installed: {}", report.skipped.join("; "))
        }));
    }
    Ok(report)
}

fn import_zip(data_dir: &Path, backups: &Path, bytes: &[u8], report: &mut ImportReport, depth: u8) -> Result<()> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| Error::Invalid("That file isn't a valid Bedrock pack (not a zip archive).".into()))?;
    if zip.len() > MAX_ENTRIES {
        return Err(Error::Invalid("That pack has too many files.".into()));
    }
    let names: Vec<String> = (0..zip.len()).filter_map(|i| zip.by_index(i).ok().and_then(|f| f.name().ok().map(|n| n.replace('\\', "/")))).collect();

    // .mcaddon files can hold .mcpack files: import those too (one level deep).
    if depth == 0 {
        for name in names.iter().filter(|n| n.to_ascii_lowercase().ends_with(".mcpack")) {
            let mut inner = vec![];
            zip.by_name(name)?.take(MAX_UNPACKED).read_to_end(&mut inner)?;
            import_zip(data_dir, backups, &inner, report, 1)?;
        }
    }

    // Each folder holding a manifest.json is a pack (skip ones nested inside another pack).
    let mut roots: Vec<String> = names
        .iter()
        .filter(|n| n.rsplit('/').next() == Some("manifest.json"))
        .map(|n| n[..n.len() - "manifest.json".len()].to_string())
        .collect();
    roots.sort_by_key(|r| r.len());
    let mut packs: Vec<String> = vec![];
    for r in roots {
        if !packs.iter().any(|p| !p.is_empty() && r.starts_with(p.as_str())) && !(packs.iter().any(|p| p.is_empty())) {
            packs.push(r);
        }
    }

    for prefix in packs {
        let mut text = String::new();
        zip.by_name(&format!("{prefix}manifest.json"))?.take(1024 * 1024).read_to_string(&mut text)?;
        let Some(manifest) = parse_manifest(&text) else {
            report.skipped.push(format!("{prefix}manifest.json can't be read"));
            continue;
        };
        let lang = {
            let mut s = String::new();
            zip.by_name(&format!("{prefix}texts/en_US.lang")).ok().and_then(|mut f| f.read_to_string(&mut s).ok()).map(|_| s)
        };
        let name = localized(&manifest.header.name, lang.as_deref());
        let Some(kind) = manifest_kind(&manifest) else {
            report.skipped.push(format!("{name}: not a resource, behavior or skin pack (world templates open in Bedrock)"));
            continue;
        };
        let base = kind_dir(kind).expect("known kind");
        let uuid = manifest.header.uuid.to_lowercase();
        if uuid.len() < 8 {
            report.skipped.push(format!("{name}: its manifest has no valid uuid"));
            continue;
        }
        let parent = data_dir.join(base);
        std::fs::create_dir_all(&parent)?;

        // the same pack (uuid) already installed → replace it, keeping the old one in backups
        let mut replaced = false;
        for existing in std::fs::read_dir(&parent)?.flatten() {
            if read_pack_dir(&existing.path(), kind, false).is_some_and(|p| p.uuid == uuid) {
                let dest = util::unique_path(&backups.join("bedrock").join(util::timestamp_slug()).join(base).join(existing.file_name()));
                util::move_path(&existing.path(), &dest)?;
                replaced = true;
            }
        }

        let folder = format!("{}_{}", util::sanitize_file_name(&name).chars().take(40).collect::<String>().trim(), &uuid[..8]);
        let dest = parent.join(folder.trim_start_matches('_'));
        let staging = parent.join(format!(".shadow-import-{}", &uuid[..8]));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging)?;
        let mut total = 0u64;
        let result = (|| -> Result<()> {
            for i in 0..zip.len() {
                let mut f = zip.by_index(i)?;
                let entry = f.name()?.replace('\\', "/");
                let Some(rel) = entry.strip_prefix(prefix.as_str()) else { continue };
                if rel.is_empty() || f.is_dir() {
                    continue;
                }
                let target = safepath::safe_join(&staging, rel)?;
                total += f.size();
                if total > MAX_UNPACKED {
                    return Err(Error::Invalid("That pack is too large to unpack safely.".into()));
                }
                if let Some(d) = target.parent() {
                    std::fs::create_dir_all(d)?;
                }
                let mut out = std::fs::File::create(&target)?;
                std::io::copy(&mut (&mut f).take(MAX_UNPACKED), &mut out)?;
            }
            Ok(())
        })();
        if let Err(e) = result {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e);
        }
        if dest.exists() {
            let old = util::unique_path(&backups.join("bedrock").join(util::timestamp_slug()).join(base).join(dest.file_name().unwrap_or_default()));
            util::move_path(&dest, &old)?;
        }
        std::fs::rename(&staging, &dest)?;
        let label = format!("{name} ({kind} pack)");
        if replaced {
            report.replaced.push(label);
        } else {
            report.installed.push(label);
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ CurseForge (Bedrock)

/// Download a Bedrock project's newest file from CurseForge and install it.
pub async fn install_from_curseforge(ctx: &crate::context::Ctx, data_dir: &Path, backups: &Path, mod_id: u64) -> Result<ImportReport> {
    use crate::mods::curseforge as cf;
    let project = cf::get_mod(ctx, mod_id).await?;
    let mut files = cf::mod_files(ctx, mod_id, None, None).await?;
    files.retain(|f| f.is_available != Some(false));
    // releases before betas/alphas, newest first
    files.sort_by(|a, b| a.release_type.cmp(&b.release_type).then(b.file_date.cmp(&a.file_date)));
    let file = files.into_iter().next().ok_or_else(|| Error::NotFound(format!("{} has no downloadable files", project.name)))?;
    let url = file.download_url.clone().ok_or_else(|| {
        Error::Incompatible(format!("{}'s author only allows downloads on the CurseForge website. Download it there, then use Import file.", project.name))
    })?;
    if !safepath::is_allowed_download_url(&url) {
        return Err(Error::Invalid(format!("Refusing to download from {url}")));
    }
    let tmp = ctx.paths.tmp_dir().join("bedrock");
    std::fs::create_dir_all(&tmp)?;
    let dest = tmp.join(util::sanitize_file_name(&file.file_name));
    let item = crate::download::DownloadItem::new(url, &dest).sha1(file.sha1()).size(file.file_length);
    ctx.downloads.run(&format!("Bedrock: {}", project.name), vec![item]).await?;
    let report = import_file(data_dir, backups, &dest);
    if !matches!(report, Ok(ImportReport { opened_in_bedrock: true, .. })) {
        let _ = std::fs::remove_file(&dest);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn pack_zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            for (name, body) in entries {
                z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
                z.write_all(body.as_bytes()).unwrap();
            }
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    const RP: &str = r#"{
        // comments are allowed in Bedrock manifests
        "format_version": 2,
        "header": { "name": "pack.name", "description": "§aShiny§r textures", "uuid": "AAAA1111-2222-3333-4444-555566667777", "version": [1, 2, 0] },
        "modules": [ { "type": "resources", "uuid": "x", "version": [1, 0, 0] } ]
    }"#;
    const BP: &str = r#"{"format_version":2,"header":{"name":"Mobs+","description":"More mobs","uuid":"bbbb1111-2222-3333-4444-555566667777","version":[1,0,0]},"modules":[{"type":"data"}]}"#;

    #[test]
    fn package_names_give_versions() {
        assert_eq!(version_from_package_name("Microsoft.MinecraftUWP_1.26.6005.0_x64__8wekyb3d8bbwe").as_deref(), Some("1.26.6005.0"));
        assert_eq!(version_from_package_name("Microsoft.MinecraftUWP_1.2_x64__other"), None);
        assert_eq!(version_from_package_name("Microsoft.MinecraftEducation_1.0_x64__8wekyb3d8bbwe"), None);
    }

    #[test]
    fn imports_mcpack_and_mcaddon_and_lists_them() {
        let d = tempfile::tempdir().unwrap();
        let (data, backups) = (d.path().join("com.mojang"), d.path().join("backups"));
        let rp = d.path().join("shiny.mcpack");
        std::fs::write(&rp, pack_zip(&[("manifest.json", RP), ("texts/en_US.lang", "pack.name=Shiny Pack\n"), ("textures/a.png", "png")])).unwrap();
        let r = import_file(&data, &backups, &rp).unwrap();
        assert_eq!(r.installed, vec!["Shiny Pack (resource pack)"]);

        // .mcaddon with a behavior pack folder and a nested .mcpack (re-importing the same RP)
        let nested = pack_zip(&[("manifest.json", RP), ("textures/b.png", "png2")]);
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            z.start_file("Mobs BP/manifest.json", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(BP.as_bytes()).unwrap();
            z.start_file("Mobs BP/entities/zombie.json", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"{}").unwrap();
            z.start_file("shiny.mcpack", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(&nested).unwrap();
            z.finish().unwrap();
        }
        let addon = d.path().join("mobs.mcaddon");
        std::fs::write(&addon, buf.into_inner()).unwrap();
        let r = import_file(&data, &backups, &addon).unwrap();
        assert_eq!(r.installed, vec!["Mobs+ (behavior pack)"]);
        assert_eq!(r.replaced.len(), 1, "same uuid replaced, old copy in backups");
        assert!(backups.join("bedrock").is_dir());

        let packs = list_packs(&data);
        assert_eq!(packs.len(), 2);
        let shiny = packs.iter().find(|p| p.kind == "resource").unwrap();
        assert_eq!(shiny.version, "1.2.0");
        assert_eq!(shiny.description, "Shiny textures", "formatting codes stripped");
        assert!(data.join("behavior_packs").join("Mobs+_bbbb1111").join("entities/zombie.json").is_file());
        assert!(data.join("resource_packs").read_dir().unwrap().flatten().all(|e| !e.file_name().to_string_lossy().starts_with(".shadow-import")));

        let moved = remove_pack(&data, &backups, "behavior", "Mobs+_bbbb1111", false).unwrap();
        assert!(moved.join("manifest.json").is_file());
        assert_eq!(list_packs(&data).len(), 1);
    }

    #[test]
    fn lists_nested_and_legacy_layouts_bedrock_leaves_behind() {
        let d = tempfile::tempdir().unwrap();
        let data = d.path();
        let nested = data.join("resource_packs/ShieldIndi/Shield Indicator");
        std::fs::create_dir_all(nested.join("texts")).unwrap();
        std::fs::write(nested.join("manifest.json"), RP).unwrap();
        std::fs::write(nested.join("texts/en_US.lang"), "pack.name=§6Shield §lIndicator\n").unwrap();
        let legacy = data.join("behavior_packs/armorhud");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("pack_manifest.json"), BP).unwrap();
        std::fs::create_dir_all(data.join("behavior_packs/empty")).unwrap();
        let packs = list_packs(data);
        assert_eq!(packs.len(), 2);
        let shield = packs.iter().find(|p| p.kind == "resource").unwrap();
        assert_eq!(shield.name, "Shield Indicator", "lang value resolved and § codes stripped");
        assert_eq!(shield.folder, "ShieldIndi", "the top folder is what gets removed");
        assert!(remove_pack(data, &d.path().join("b"), "resource", "ShieldIndi", false).is_ok());
    }

    #[test]
    fn rejects_traversal_non_packs_and_bad_paths() {
        let d = tempfile::tempdir().unwrap();
        let (data, backups) = (d.path().join("com.mojang"), d.path().join("backups"));
        let evil = d.path().join("evil.mcpack");
        std::fs::write(&evil, pack_zip(&[("manifest.json", BP), ("../../outside.txt", "x")])).unwrap();
        assert!(import_file(&data, &backups, &evil).is_err());
        assert!(!d.path().join("outside.txt").exists());
        assert!(!data.join("behavior_packs").read_dir().map(|mut r| r.any(|e| e.unwrap().file_name().to_string_lossy().starts_with("Mobs"))).unwrap_or(false), "nothing half-installed");

        let empty = d.path().join("empty.mcpack");
        std::fs::write(&empty, pack_zip(&[("readme.txt", "hi")])).unwrap();
        assert!(import_file(&data, &backups, &empty).is_err());
        std::fs::write(d.path().join("x.txt"), "x").unwrap();
        assert!(import_file(&data, &backups, &d.path().join("x.txt")).is_err());
        assert!(remove_pack(&data, &backups, "behavior", "../../etc", false).is_err());
        assert!(remove_pack(&data, &backups, "world", "x", false).is_err());
    }
}
