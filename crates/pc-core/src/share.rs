//! Share a profile's mods as a link. The link carries the pack itself (name, Minecraft version,
//! loader and the exact Modrinth versions of its mods, resource packs and shaders), compressed into
//! the part after `#`, so nothing is uploaded anywhere: the website's share page shows what's in it
//! and hands it to Shadow Client through the `shadowclient://` link, which installs it into a new
//! profile with the usual compatibility and hash checks.
//!
//! Only content Shadow Client knows the exact download of can be shared (installed from Modrinth or
//! CurseForge, or a manual jar Modrinth recognised by its hash); everything else is listed as left out.

use std::io::{Read, Write};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::mods::index::ModIndex;
use crate::mods::manage::{InstalledMod, ProfileScope};
use crate::mods::resolver::{self, InstallPlan, InstalledSet};
use crate::mods::{ContentKind, Target};
use crate::{Error, Result};

/// The custom link scheme Windows hands to Shadow Client (`shadowclient://share/<code>`).
pub const SCHEME: &str = "shadowclient";
const FORMAT: u8 = 1;
const MAX_ITEMS: usize = 400;
/// The decompressed pack is small JSON; anything bigger isn't one of ours.
const MAX_JSON: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharePack {
    #[serde(rename = "f")]
    pub format: u8,
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "m")]
    pub mc_version: String,
    #[serde(rename = "l")]
    pub loader: String,
    #[serde(rename = "i")]
    pub items: Vec<ShareItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShareItem {
    /// "m" mod, "r" resource pack, "s" shader
    #[serde(rename = "k")]
    pub kind: String,
    /// Modrinth project id
    #[serde(rename = "p")]
    pub project: String,
    /// Modrinth version id
    #[serde(rename = "v")]
    pub version: String,
    /// Name to show before installing
    #[serde(rename = "t")]
    pub title: String,
    /// Where it comes from: absent for Modrinth, "c" for CurseForge (then `project` is the mod id
    /// and `version` the file id)
    #[serde(rename = "s", default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

const CURSEFORGE: &str = "c";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareExport {
    pub code: String,
    pub link: String,
    pub included: Vec<String>,
    /// Content that can't be shared (not from Modrinth), by name
    pub skipped: Vec<String>,
}

fn kind_code(k: ContentKind) -> &'static str {
    match k {
        ContentKind::Mod => "m",
        ContentKind::Resourcepack => "r",
        ContentKind::Shader => "s",
    }
}

fn kind_of(code: &str) -> Option<ContentKind> {
    match code {
        "m" => Some(ContentKind::Mod),
        "r" => Some(ContentKind::Resourcepack),
        "s" => Some(ContentKind::Shader),
        _ => None,
    }
}

fn website() -> Option<String> {
    serde_json::from_str::<serde_json::Value>(include_str!("../../../branding.json"))
        .ok()?
        .get("websiteUrl")?
        .as_str()
        .filter(|u| u.starts_with("https://"))
        .map(|u| if u.ends_with('/') { u.to_string() } else { format!("{u}/") })
}

/// The link people share: the website's share page (it explains the pack and opens Shadow Client),
/// or the app link itself when there's no website.
pub fn link_for(code: &str) -> String {
    match website() {
        Some(site) => format!("{site}share.html#{code}"),
        None => format!("{SCHEME}://share/{code}"),
    }
}

pub fn encode(pack: &SharePack) -> Result<String> {
    let json = serde_json::to_vec(pack).map_err(|e| Error::msg(e.to_string()))?;
    let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    z.write_all(&json)?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(z.finish()?))
}

/// Read a pack from a share link, an app link or the bare code.
pub fn decode(text: &str) -> Result<SharePack> {
    let t = text.trim();
    let code = if let Some(i) = t.rfind('#') {
        &t[i + 1..]
    } else if let Some(i) = t.find("://share/") {
        &t[i + "://share/".len()..]
    } else {
        t
    };
    let code = code.trim_end_matches('/');
    let bad = || Error::Invalid("That isn't a Shadow Client share link (or it was cut off when copying).".into());
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(code.trim_end_matches('=')).map_err(|_| bad())?;
    let mut json = Vec::new();
    flate2::read::DeflateDecoder::new(&bytes[..]).take(MAX_JSON).read_to_end(&mut json).map_err(|_| bad())?;
    let pack: SharePack = serde_json::from_slice(&json).map_err(|_| bad())?;
    validate(&pack)?;
    Ok(pack)
}

fn validate(p: &SharePack) -> Result<()> {
    let id = |s: &str| !s.is_empty() && s.len() <= 16 && s.chars().all(|c| c.is_ascii_alphanumeric());
    let fail = |m: &str| Err(Error::Invalid(format!("This share link can't be used: {m}.")));
    if p.format != FORMAT {
        return fail("it was made by a newer Shadow Client; update Shadow Client first");
    }
    if p.name.trim().is_empty() || p.name.chars().count() > 64 {
        return fail("the pack name is missing or too long");
    }
    if p.mc_version.is_empty() || p.mc_version.len() > 32 || !p.mc_version.chars().all(|c| c.is_ascii_alphanumeric() || ".-_ ".contains(c)) {
        return fail("the Minecraft version is unreadable");
    }
    if !matches!(p.loader.as_str(), "vanilla" | "fabric" | "quilt" | "forge" | "neoforge") {
        return fail("the mod loader is unknown");
    }
    if p.items.len() > MAX_ITEMS {
        return fail("it lists too many files");
    }
    let numeric = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    let source_ok = |i: &ShareItem| match i.source.as_deref() {
        None => true,
        Some(CURSEFORGE) => numeric(&i.project) && numeric(&i.version),
        Some(_) => false,
    };
    if p.items.iter().any(|i| kind_of(&i.kind).is_none() || !id(&i.project) || !id(&i.version) || !source_ok(i) || i.title.chars().count() > 120) {
        return fail("one of its entries is unreadable");
    }
    Ok(())
}

/// Build the share link for a profile from its installed content. `mods` should already be
/// identified (Modrinth hash lookup) so manual jars Modrinth knows can be shared too. Modrinth is
/// preferred; CurseForge installs Modrinth doesn't recognise go in as CurseForge files.
pub fn export(scope: &ProfileScope, name: &str, mods: &[InstalledMod]) -> Result<ShareExport> {
    let mut items = Vec::new();
    let mut included = Vec::new();
    let mut skipped = Vec::new();
    for m in mods.iter().filter(|m| m.enabled && !m.builtin) {
        let from = |src: &str| {
            let s = m.source.as_ref().filter(|s| s.source == src)?;
            Some((s.project_id.clone()?, s.version_id.clone()?))
        };
        let ids = from("modrinth")
            .map(|(p, v)| (p, v, None))
            .or_else(|| m.identified.as_ref().map(|i| (i.project_id.clone(), i.version_id.clone(), None)))
            .or_else(|| from("curseforge").map(|(p, v)| (p, v, Some(CURSEFORGE.to_string()))));
        match ids {
            Some((project, version, source)) => {
                let title = m.display_name();
                items.push(ShareItem { kind: "m".into(), project, version, title: title.chars().take(120).collect(), source });
                included.push(title);
            }
            None => skipped.push(m.display_name()),
        }
    }
    // resource packs and shaders installed from Modrinth or CurseForge (manual ones can't be looked up)
    let index = ModIndex::load(&scope.data_dir);
    for e in index.entries.iter().filter(|e| e.kind != ContentKind::Mod) {
        if !scope.folder(e.kind).join(&e.file_name).exists() {
            continue;
        }
        match (&e.project_id, &e.version_id) {
            (Some(p), Some(v)) if e.source == "modrinth" || e.source == "curseforge" => {
                let title = e.title.clone().unwrap_or_else(|| e.file_name.clone());
                let source = (e.source == "curseforge").then(|| CURSEFORGE.to_string());
                items.push(ShareItem { kind: kind_code(e.kind).into(), project: p.clone(), version: v.clone(), title: title.chars().take(120).collect(), source });
                included.push(title);
            }
            _ => skipped.push(e.title.clone().unwrap_or_else(|| e.file_name.clone())),
        }
    }
    if items.is_empty() {
        return Err(Error::Invalid("Nothing in this profile can be shared yet: only mods, resource packs and shaders from Modrinth or CurseForge can go in a share link.".into()));
    }
    let pack = SharePack {
        format: FORMAT,
        name: name.chars().take(64).collect(),
        mc_version: scope.target.mc_version.clone(),
        loader: scope.target.loader.as_str().to_string(),
        items,
    };
    let code = encode(&pack)?;
    Ok(ShareExport { link: link_for(&code), code, included, skipped })
}

/// Everything in the pack (and what it needs), as one plan for the user to confirm.
pub async fn plan(ctx: &Ctx, target: &Target, installed: &InstalledSet, pack: &SharePack) -> Result<InstallPlan> {
    let mut plan = InstallPlan { target: Some(target.clone()), ..Default::default() };
    for item in &pack.items {
        let Some(kind) = kind_of(&item.kind) else { continue };
        let resolved = match item.source.as_deref() {
            Some(CURSEFORGE) => {
                let (Ok(mod_id), Ok(file_id)) = (item.project.parse(), item.version.parse()) else { continue };
                resolver::resolve_curseforge(ctx, target, kind, mod_id, Some(file_id), installed).await
            }
            _ => resolver::resolve_modrinth(ctx, target, kind, &item.project, Some(&item.version), installed, false).await,
        };
        match resolved {
            Ok(p) if p.blocked => plan.warnings.push(format!("{} skipped: {}", item.title, p.blocked_reason.unwrap_or_default())),
            Ok(p) => resolver::merge_plans(&mut plan, p),
            Err(e) => plan.warnings.push(format!("{} skipped: {e}", item.title)),
        }
    }
    plan.finalize(installed);
    Ok(plan)
}

/// Let Windows open `shadowclient://` links with this copy of Shadow Client (per user, no admin).
#[cfg(windows)]
pub fn register_scheme(exe: &std::path::Path) -> Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(format!(r"Software\Classes\{SCHEME}"))?;
    key.set_value("", &"URL:Shadow Client share link")?;
    key.set_value("URL Protocol", &"")?;
    let exe = exe.display().to_string();
    let (icon, _) = key.create_subkey("DefaultIcon")?;
    icon.set_value("", &format!("\"{exe}\",0"))?;
    let (cmd, _) = key.create_subkey(r"shell\open\command")?;
    cmd.set_value("", &format!("\"{exe}\" \"%1\""))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn register_scheme(_exe: &std::path::Path) -> Result<()> {
    Ok(())
}

/// A `shadowclient://` link among the program's arguments (how Windows passes a clicked link).
pub fn link_in_args<I: IntoIterator<Item = String>>(args: I) -> Option<String> {
    args.into_iter().find(|a| a.to_ascii_lowercase().starts_with(&format!("{SCHEME}://")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> SharePack {
        SharePack {
            format: FORMAT,
            name: "PvP pack".into(),
            mc_version: "1.21.11".into(),
            loader: "fabric".into(),
            items: vec![
                ShareItem { kind: "m".into(), project: "AANobbMI".into(), version: "abc123XY".into(), title: "Sodium".into(), source: None },
                ShareItem { kind: "r".into(), project: "zzzz1111".into(), version: "yyyy2222".into(), title: "Clear Glass".into(), source: None },
                ShareItem { kind: "m".into(), project: "419699".into(), version: "7339256".into(), title: "Architectury".into(), source: Some("c".into()) },
            ],
        }
    }

    #[test]
    fn links_round_trip_in_every_form() {
        let code = encode(&pack()).unwrap();
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'), "safe in a URL: {code}");
        assert_eq!(decode(&code).unwrap(), pack());
        assert_eq!(decode(&link_for(&code)).unwrap(), pack());
        assert!(link_for(&code).starts_with("https://") && link_for(&code).contains("share.html#"));
        assert_eq!(decode(&format!("shadowclient://share/{code}/")).unwrap(), pack());
        assert_eq!(decode(&format!("  {code}\n")).unwrap(), pack());
    }

    #[test]
    fn bad_links_are_refused_clearly() {
        assert!(decode("hello").is_err());
        assert!(decode("https://example.com/share.html#").is_err());
        let mut p = pack();
        p.items[0].project = "../../etc".into();
        assert!(decode(&encode(&p).unwrap()).is_err(), "ids must be plain");
        let mut p = pack();
        p.loader = "evil".into();
        assert!(decode(&encode(&p).unwrap()).is_err());
        let mut p = pack();
        p.format = 9;
        assert!(decode(&encode(&p).unwrap()).unwrap_err().to_string().contains("newer Shadow Client"));
        let mut p = pack();
        p.items[2].version = "abc".into();
        assert!(decode(&encode(&p).unwrap()).is_err(), "CurseForge ids are numbers");
        let mut p = pack();
        p.items[0].source = Some("x".into());
        assert!(decode(&encode(&p).unwrap()).is_err(), "unknown source");
        // a zip bomb doesn't get past the size limit
        let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        z.write_all(&vec![b' '; 4 * 1024 * 1024]).unwrap();
        let bomb = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(z.finish().unwrap());
        assert!(decode(&bomb).is_err());
    }

    #[test]
    fn finds_the_app_link_in_arguments() {
        let args = vec!["C:\\Shadow\\shadow-client.exe".to_string(), "ShadowClient://share/abc".to_string()];
        assert_eq!(link_in_args(args).as_deref(), Some("ShadowClient://share/abc"));
        assert!(link_in_args(vec!["x.exe".to_string()]).is_none());
    }
}
