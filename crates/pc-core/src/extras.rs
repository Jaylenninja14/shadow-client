//! Smaller launcher features: the update check against the project's GitHub releases, a
//! profile's screenshots, and Bedrock world backups and its saved server list.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::{util, Error, Result};

// ------------------------------------------------------------------ update check

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub newer: bool,
    /// Where to get the update by hand: the website's download section (the GitHub release page
    /// only when branding.json has no website).
    pub url: Option<String>,
    pub notes: Option<String>,
    /// The release's installer, when it has one with a published SHA-256.
    pub installer: Option<UpdateInstaller>,
    /// The release's MSI, for copies installed from the MSI (also with its published SHA-256).
    #[serde(default)]
    pub msi: Option<UpdateInstaller>,
    /// Set by the app: true when this copy can install the update itself (an installed copy, not portable).
    #[serde(default)]
    pub can_install: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstaller {
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

/// The installer asset's name in every release (the website links to the same name).
const INSTALLER_ASSET: &str = "Shadow-Client-Setup.exe";
/// The MSI asset's name in every release.
const MSI_ASSET: &str = "Shadow-Client.msi";

/// The GitHub repository releases are published to ("owner/name"), from branding.json.
fn release_repo() -> Option<String> {
    serde_json::from_str::<serde_json::Value>(include_str!("../../../branding.json"))
        .ok()?
        .get("githubRepo")?
        .as_str()
        .filter(|r| r.split('/').count() == 2 && r.chars().all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c)))
        .map(String::from)
}

/// The website's download section ("<websiteUrl>#download"), from branding.json; https only.
fn website_download() -> Option<String> {
    serde_json::from_str::<serde_json::Value>(include_str!("../../../branding.json"))
        .ok()?
        .get("websiteUrl")?
        .as_str()
        .filter(|u| u.starts_with("https://"))
        .map(|u| format!("{}#download", u.trim_end_matches('#')))
}

/// Is a newer Shadow Client published? Checks at most once every 10 minutes (cached), so a new release
/// shows up in open launchers soon after it is published.
pub async fn check_update(ctx: &Ctx) -> Result<UpdateInfo> {
    // Dev builds only: SHADOW_UPDATE_API points the check at a local test release (to see the update screens).
    #[cfg(debug_assertions)]
    if let Ok(api) = std::env::var("SHADOW_UPDATE_API") {
        let prefix = std::env::var("SHADOW_UPDATE_DOWNLOADS").unwrap_or_else(|_| "https://github.com/".into());
        return check_update_at(ctx, &api, &prefix).await;
    }
    match release_repo() {
        Some(repo) => {
            let api = format!("https://api.github.com/repos/{repo}/releases/latest");
            check_update_at(ctx, &api, &format!("https://github.com/{repo}/releases/download/")).await
        }
        None => Ok(UpdateInfo { current: crate::VERSION.into(), latest: None, newer: false, url: None, notes: None, installer: None, msi: None, can_install: false }),
    }
}

/// `download_prefix`: the only place the installer may come from (the repository's release downloads).
async fn check_update_at(ctx: &Ctx, url: &str, download_prefix: &str) -> Result<UpdateInfo> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::ACCEPT, reqwest::header::HeaderValue::from_static("application/vnd.github+json"));
    let v: serde_json::Value = ctx.http.get_json(url, Some(std::time::Duration::from_secs(600)), headers).await?;
    let tag = v["tag_name"].as_str().unwrap_or_default().trim_start_matches('v').to_string();
    let newer = crate::mods::metadata::compare_versions(&tag, crate::VERSION) == Some(std::cmp::Ordering::Greater);
    let asset = |name: &str| {
        v["assets"].as_array().into_iter().flatten().find(|a| a["name"] == name).and_then(|a| {
            let url = a["browser_download_url"].as_str().filter(|u| u.starts_with(download_prefix))?;
            let sha256 = a["digest"].as_str()?.strip_prefix("sha256:")?.to_ascii_lowercase();
            let size = a["size"].as_u64().filter(|s| *s > 0)?;
            (sha256.len() == 64 && sha256.chars().all(|c| c.is_ascii_hexdigit())).then(|| UpdateInstaller { url: url.into(), sha256, size })
        })
    };
    let installer = asset(INSTALLER_ASSET);
    let msi = asset(MSI_ASSET);
    Ok(UpdateInfo {
        current: crate::VERSION.into(),
        latest: (!tag.is_empty()).then_some(tag),
        newer,
        url: website_download().or_else(|| v["html_url"].as_str().filter(|u| u.starts_with("https://github.com/")).map(String::from)),
        notes: v["body"].as_str().map(|b| b.chars().take(800).collect()),
        installer,
        msi,
        can_install: false,
    })
}

/// The product name from branding.json ("Shadow Client"), as installers register it.
fn product_name() -> Option<String> {
    serde_json::from_str::<serde_json::Value>(include_str!("../../../branding.json")).ok()?.get("productName")?.as_str().map(String::from)
}

/// Was this copy installed from the MSI? Windows Installer lists such programs under Uninstall with
/// `WindowsInstaller = 1`; when it recorded an install folder, it has to be this copy's.
#[cfg(windows)]
pub fn installed_by_msi(exe_dir: &Path) -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    let Some(product) = product_name() else { return false };
    let places = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];
    for (root, path) in places {
        let Ok(list) = winreg::RegKey::predef(root).open_subkey(path) else { continue };
        for name in list.enum_keys().flatten() {
            let Ok(k) = list.open_subkey(&name) else { continue };
            let display: String = k.get_value("DisplayName").unwrap_or_default();
            let by_msi: u32 = k.get_value("WindowsInstaller").unwrap_or(0);
            if by_msi != 1 || display != product {
                continue;
            }
            let at: String = k.get_value("InstallLocation").unwrap_or_default();
            let at = at.trim().trim_end_matches('\\');
            // without a recorded folder, only the MSI's own one (…\<product name>) counts
            let here = if at.is_empty() { exe_dir.file_name().is_some_and(|n| n.to_string_lossy() == product) } else { exe_dir.starts_with(at) };
            if here {
                return true;
            }
        }
    }
    false
}

#[cfg(not(windows))]
pub fn installed_by_msi(_exe_dir: &Path) -> bool {
    false
}

/// Download the newest release's installer (`msi`: its MSI) into the launcher's temp folder and check its
/// SHA-256 against the one GitHub publishes for it. Returns the file's path; nothing is run here.
pub async fn download_update(ctx: &Ctx, msi: bool) -> Result<PathBuf> {
    let info = check_update(ctx).await?;
    download_installer(ctx, &info, msi).await
}

async fn download_installer(ctx: &Ctx, info: &UpdateInfo, msi: bool) -> Result<PathBuf> {
    if !info.newer {
        return Err(Error::Invalid("You already have the newest Shadow Client.".into()));
    }
    let (Some(latest), Some(inst)) = (info.latest.as_deref(), if msi { info.msi.as_ref() } else { info.installer.as_ref() }) else {
        return Err(Error::NotFound("This release has no installer to download. Get it from the website instead.".into()));
    };
    if !latest.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return Err(Error::Invalid(format!("Unexpected version name: {latest}")));
    }
    let file = if msi { format!("Shadow-Client-{latest}.msi") } else { format!("Shadow-Client-{latest}-setup.exe") };
    let dest = ctx.paths.tmp_dir().join("updates").join(file);
    if sha256_file(&dest).await.as_deref() == Some(inst.sha256.as_str()) {
        return Ok(dest);
    }
    let _ = tokio::fs::remove_file(&dest).await;
    crate::download::fetch_to_file(&ctx.http, &crate::download::DownloadItem::new(&inst.url, &dest).size(Some(inst.size))).await?;
    let actual = sha256_file(&dest).await.unwrap_or_default();
    if actual != inst.sha256 {
        let _ = tokio::fs::remove_file(&dest).await;
        return Err(Error::HashMismatch { file: (if msi { MSI_ASSET } else { INSTALLER_ASSET }).into(), expected: inst.sha256.clone(), actual });
    }
    Ok(dest)
}

async fn sha256_file(path: &Path) -> Option<String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        use sha2::Digest;
        let mut file = std::fs::File::open(&path).ok()?;
        let mut hasher = sha2::Sha256::new();
        std::io::copy(&mut file, &mut hasher).ok()?;
        Some(hex::encode(hasher.finalize()))
    })
    .await
    .ok()
    .flatten()
}

// ------------------------------------------------------------------ screenshots

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub taken: Option<String>,
}

/// A profile's screenshots, newest first.
pub fn screenshots(game_dir: &Path) -> Vec<Screenshot> {
    let mut out: Vec<(std::time::SystemTime, Screenshot)> = std::fs::read_dir(game_dir.join("screenshots"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let lower = name.to_ascii_lowercase();
            if !(lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")) {
                return None;
            }
            let meta = e.metadata().ok()?;
            let modified = meta.modified().ok()?;
            let taken = chrono::DateTime::<chrono::Local>::from(modified).format("%Y-%m-%d %H:%M").to_string();
            Some((modified, Screenshot { name, path: e.path(), size: meta.len(), taken: Some(taken) }))
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.into_iter().map(|(_, s)| s).collect()
}

/// Move a screenshot to the launcher's backups (recoverable), never deleting it.
pub fn remove_screenshot(game_dir: &Path, backups: &Path, name: &str) -> Result<PathBuf> {
    if name.contains(['/', '\\']) || name.starts_with('.') {
        return Err(Error::Invalid("Not a screenshot name".into()));
    }
    let src = game_dir.join("screenshots").join(name);
    if !src.is_file() {
        return Err(Error::NotFound(name.into()));
    }
    let to = backups.join("screenshots").join(util::timestamp_slug()).join(name);
    util::move_path(&src, &to)?;
    Ok(to)
}

// ------------------------------------------------------------------ Bedrock worlds

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockWorld {
    pub folder: String,
    pub name: String,
    pub size: u64,
    pub played: Option<String>,
    /// world_icon.jpeg as a data: URL (small)
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldBackup {
    pub file: String,
    pub path: PathBuf,
    pub size: u64,
    pub created: Option<String>,
}

fn worlds_dir(data_dir: &Path, user: Option<&str>) -> Result<PathBuf> {
    let (_, pe) = crate::bedrock_menu::account_dir(data_dir, user)?;
    let dir = pe.parent().map(|p| p.join("minecraftWorlds")).unwrap_or_default();
    Ok(if dir.is_dir() { dir } else { data_dir.join("minecraftWorlds") })
}

fn dir_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

pub fn bedrock_worlds(data_dir: &Path, user: Option<&str>) -> Result<Vec<BedrockWorld>> {
    use base64::Engine;
    let dir = worlds_dir(data_dir, user)?;
    let mut out: Vec<(std::time::SystemTime, BedrockWorld)> = vec![];
    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let p = e.path();
        if !p.join("level.dat").is_file() {
            continue;
        }
        let name = std::fs::read_to_string(p.join("levelname.txt")).map(|s| s.trim().to_string()).unwrap_or_else(|_| e.file_name().to_string_lossy().into_owned());
        let played = std::fs::metadata(p.join("level.dat")).and_then(|m| m.modified()).ok();
        let icon = std::fs::read(p.join("world_icon.jpeg"))
            .ok()
            .filter(|b| b.len() < 300_000)
            .map(|b| format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(b)));
        out.push((
            played.unwrap_or(std::time::UNIX_EPOCH),
            BedrockWorld {
                folder: e.file_name().to_string_lossy().into_owned(),
                name,
                size: dir_size(&p),
                played: played.map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d %H:%M").to_string()),
                icon,
            },
        ));
    }
    out.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(out.into_iter().map(|(_, w)| w).collect())
}

fn world_backup_dir(backups: &Path) -> PathBuf {
    backups.join("bedrock").join("worlds")
}

/// Zip a world into a .mcworld in the launcher's backups. Bedrock imports that file as a copy, so
/// restoring never overwrites the world you have.
pub fn backup_bedrock_world(data_dir: &Path, user: Option<&str>, folder: &str, backups: &Path) -> Result<PathBuf> {
    backup_world_inner(data_dir, user, folder, backups, crate::bedrock_menu::is_running())
}

fn backup_world_inner(data_dir: &Path, user: Option<&str>, folder: &str, backups: &Path, running: bool) -> Result<PathBuf> {
    if running {
        return Err(Error::Invalid("Close Minecraft Bedrock first: worlds can't be copied safely while it has them open.".into()));
    }
    if folder.contains(['/', '\\']) || folder.starts_with('.') {
        return Err(Error::Invalid("Not a world folder".into()));
    }
    let world = worlds_dir(data_dir, user)?.join(folder);
    if !world.join("level.dat").is_file() {
        return Err(Error::NotFound(folder.into()));
    }
    let name = std::fs::read_to_string(world.join("levelname.txt")).map(|s| s.trim().to_string()).unwrap_or_else(|_| folder.to_string());
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect::<String>().trim().to_string();
    let out_dir = world_backup_dir(backups);
    std::fs::create_dir_all(&out_dir)?;
    let target = out_dir.join(format!("{}-{}.mcworld", if safe.is_empty() { "world" } else { &safe }, util::timestamp_slug()));
    let part = target.with_extension("mcworld.part");
    {
        let file = std::fs::File::create(&part)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        add_dir(&mut zip, &world, &world, opts)?;
        zip.finish().map_err(|e| Error::msg(e.to_string()))?;
    }
    std::fs::rename(&part, &target)?;
    Ok(target)
}

fn add_dir(zip: &mut zip::ZipWriter<std::fs::File>, root: &Path, dir: &Path, opts: zip::write::SimpleFileOptions) -> Result<()> {
    for e in std::fs::read_dir(dir)?.flatten() {
        let p = e.path();
        let rel = p.strip_prefix(root).map_err(|e| Error::msg(e.to_string()))?.to_string_lossy().replace('\\', "/");
        if p.is_dir() {
            add_dir(zip, root, &p, opts)?;
        } else {
            zip.start_file(rel, opts).map_err(|e| Error::msg(e.to_string()))?;
            zip.write_all(&std::fs::read(&p)?)?;
        }
    }
    Ok(())
}

pub fn bedrock_world_backups(backups: &Path) -> Vec<WorldBackup> {
    let mut out: Vec<WorldBackup> = std::fs::read_dir(world_backup_dir(backups))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".mcworld"))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            Some(WorldBackup {
                file: e.file_name().to_string_lossy().into_owned(),
                path: e.path(),
                size: meta.len(),
                created: meta.modified().ok().map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d %H:%M").to_string()),
            })
        })
        .collect();
    out.sort_by(|a, b| b.created.cmp(&a.created));
    out
}

// ------------------------------------------------------------------ Bedrock server list

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BedrockServer {
    pub index: u32,
    pub name: String,
    pub address: String,
    pub port: u16,
}

/// Bedrock keeps added servers in minecraftpe/external_servers.txt: "index:name:address:port:time".
fn servers_file(data_dir: &Path, user: Option<&str>) -> Result<(String, PathBuf)> {
    let (id, pe) = crate::bedrock_menu::account_dir(data_dir, user)?;
    Ok((id, pe.join("external_servers.txt")))
}

pub fn bedrock_servers(data_dir: &Path, user: Option<&str>) -> Result<Vec<BedrockServer>> {
    let (_, file) = servers_file(data_dir, user)?;
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    Ok(text
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.trim_end_matches('\r').splitn(5, ':').collect();
            if f.len() < 4 {
                return None;
            }
            Some(BedrockServer { index: f[0].parse().ok()?, name: f[1].to_string(), address: f[2].to_string(), port: f[3].parse().ok()? })
        })
        .collect())
}

fn write_servers(file: &Path, servers: &[BedrockServer]) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let text: String = servers.iter().enumerate().map(|(i, s)| format!("{}:{}:{}:{}:{}\n", i + 1, s.name, s.address, s.port, now)).collect();
    util::atomic_write(file, text.as_bytes())
}

pub fn add_bedrock_server(data_dir: &Path, user: Option<&str>, backups: &Path, name: &str, address: &str, port: u16) -> Result<Vec<BedrockServer>> {
    edit_servers(data_dir, user, backups, crate::bedrock_menu::is_running(), |list| {
        let name = name.trim();
        let address = address.trim();
        if name.is_empty() || name.contains(':') || address.is_empty() || address.contains([':', ' ']) || port == 0 {
            return Err(Error::Invalid("Use a name without \":\", a host name or IP address, and a port (Bedrock's is 19132).".into()));
        }
        list.push(BedrockServer { index: 0, name: name.into(), address: address.into(), port });
        Ok(())
    })
}

pub fn remove_bedrock_server(data_dir: &Path, user: Option<&str>, backups: &Path, index: u32) -> Result<Vec<BedrockServer>> {
    edit_servers(data_dir, user, backups, crate::bedrock_menu::is_running(), |list| {
        list.retain(|s| s.index != index);
        Ok(())
    })
}

fn edit_servers(
    data_dir: &Path,
    user: Option<&str>,
    backups: &Path,
    running: bool,
    change: impl FnOnce(&mut Vec<BedrockServer>) -> Result<()>,
) -> Result<Vec<BedrockServer>> {
    if running {
        return Err(Error::Invalid("Close Minecraft Bedrock first: it saves its server list when it closes and would undo this.".into()));
    }
    let (id, file) = servers_file(data_dir, user)?;
    let mut list = bedrock_servers(data_dir, user)?;
    change(&mut list)?;
    crate::bedrock_menu::backup_file(&file, &crate::bedrock_menu::backup_dir(backups, &id), "external_servers.txt")?;
    write_servers(&file, &list)?;
    bedrock_servers(data_dir, user)
}

/// Open Bedrock and join a server (Bedrock's own minecraft://connect link).
pub fn bedrock_join(address: &str, port: u16) -> Result<()> {
    let host = address.trim();
    if host.is_empty() || port == 0 || !host.chars().all(|c| c.is_ascii_alphanumeric() || ".-_".contains(c)) {
        return Err(Error::Invalid("Use a host name or IP address and a port".into()));
    }
    let url = format!("minecraft://connect/?serverUrl={host}&serverPort={port}");
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe").arg(&url).spawn().map_err(|e| Error::msg(format!("Could not open Bedrock: {e}")))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err(Error::Incompatible("Minecraft Bedrock can only be started on Windows.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bedrock_layout(root: &Path) -> PathBuf {
        let data = root.join("Users/Shared/games/com.mojang");
        let pe = root.join("Users/42/games/com.mojang/minecraftpe");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&pe).unwrap();
        std::fs::write(pe.join("options.txt"), "gfx_vsync:1\n").unwrap();
        std::fs::write(pe.join("external_servers.txt"), "1:My Server:play.example.net:19132:1700000000\n").unwrap();
        let w = root.join("Users/42/games/com.mojang/minecraftWorlds/abc=");
        std::fs::create_dir_all(w.join("db")).unwrap();
        std::fs::write(w.join("level.dat"), b"level").unwrap();
        std::fs::write(w.join("levelname.txt"), "Shadow Base").unwrap();
        std::fs::write(w.join("db/000001.ldb"), b"chunks").unwrap();
        data
    }

    #[test]
    fn worlds_list_and_back_up_as_mcworld() {
        let d = tempfile::tempdir().unwrap();
        let data = bedrock_layout(d.path());
        let backups = d.path().join("backups");
        let worlds = bedrock_worlds(&data, None).unwrap();
        assert_eq!(worlds.len(), 1);
        assert_eq!((worlds[0].name.as_str(), worlds[0].folder.as_str()), ("Shadow Base", "abc="));
        assert!(backup_world_inner(&data, None, "abc=", &backups, true).is_err(), "refused while Bedrock runs");
        assert!(backup_world_inner(&data, None, "../x", &backups, false).is_err());
        let file = backup_world_inner(&data, None, "abc=", &backups, false).unwrap();
        assert!(file.file_name().unwrap().to_string_lossy().starts_with("Shadow Base-"));
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&file).unwrap()).unwrap();
        assert!(zip.by_name("level.dat").is_ok() && zip.by_name("db/000001.ldb").is_ok());
        assert_eq!(bedrock_world_backups(&backups).len(), 1);
    }

    #[test]
    fn server_list_add_and_remove_with_backup() {
        let d = tempfile::tempdir().unwrap();
        let data = bedrock_layout(d.path());
        let backups = d.path().join("backups");
        assert_eq!(bedrock_servers(&data, None).unwrap()[0].name, "My Server");
        let list = edit_servers(&data, None, &backups, false, |l| {
            l.push(BedrockServer { index: 0, name: "Shadow".into(), address: "127.0.0.1".into(), port: 19133 });
            Ok(())
        })
        .unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!((list[1].index, list[1].port), (2, 19133));
        assert!(edit_servers(&data, None, &backups, true, |_| Ok(())).is_err());
        let list = edit_servers(&data, None, &backups, false, |l| {
            l.retain(|s| s.index != 1);
            Ok(())
        })
        .unwrap();
        assert_eq!((list.len(), list[0].name.as_str(), list[0].index), (1, "Shadow", 1));
        assert!(crate::bedrock_menu::backup_dir(&backups, "42").join("external_servers.txt.original").is_file());
    }

    #[test]
    fn screenshots_newest_first_and_removed_to_backups() {
        let d = tempfile::tempdir().unwrap();
        let shots = d.path().join("screenshots");
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(shots.join("a.png"), b"x").unwrap();
        std::fs::write(shots.join("notes.txt"), b"x").unwrap();
        assert_eq!(screenshots(d.path()).len(), 1);
        assert!(remove_screenshot(d.path(), &d.path().join("b"), "../a.png").is_err());
        let moved = remove_screenshot(d.path(), &d.path().join("b"), "a.png").unwrap();
        assert!(moved.is_file() && screenshots(d.path()).is_empty());
    }

    #[tokio::test]
    async fn update_check_compares_versions() {
        let mut s = mockito::Server::new_async().await;
        let _m = s
            .mock("GET", "/repos/o/r/releases/latest")
            .with_body(r#"{"tag_name":"v9.9.0","html_url":"https://github.com/o/r/releases/tag/v9.9.0","body":"New things"}"#)
            .create_async()
            .await;
        let d = tempfile::tempdir().unwrap();
        let (ctx, _) = crate::testutil::ctx(d.path(), &s.url());
        let u = check_update_at(&ctx, &format!("{}/repos/o/r/releases/latest", s.url()), "https://github.com/o/r/releases/download/").await.unwrap();
        assert!(u.newer && u.latest.as_deref() == Some("9.9.0"));
        // the manual download goes to the website, not the GitHub release page
        let url = u.url.as_deref().unwrap();
        assert!(url.starts_with("https://") && url.ends_with("#download") && !url.contains("/releases/"), "{url}");
        assert!(u.installer.is_none() && !u.can_install);
    }

    #[tokio::test]
    async fn update_installer_is_downloaded_and_checked() {
        use sha2::Digest;
        let mut s = mockito::Server::new_async().await;
        let body = b"pretend installer".to_vec();
        let good = hex::encode(sha2::Sha256::digest(&body));
        let prefix = format!("{}/o/r/releases/download/", s.url());
        let release = |digest: &str| {
            serde_json::json!({"tag_name":"v9.9.0","html_url":"https://github.com/o/r/releases/tag/v9.9.0","body":"x","assets":[
                {"name":"Shadow-Client.msi","browser_download_url":format!("{prefix}v9.9.0/Shadow-Client.msi"),"size":5,"digest":format!("sha256:{good}")},
                {"name":"Shadow-Client-Setup.exe","browser_download_url":format!("{prefix}v9.9.0/Shadow-Client-Setup.exe"),"size":body.len(),"digest":digest}]})
            .to_string()
        };
        let _m1 = s.mock("GET", "/good/releases/latest").with_body(release(&format!("sha256:{good}"))).create_async().await;
        let _m2 = s.mock("GET", "/bad/releases/latest").with_body(release(&format!("sha256:{}", "0".repeat(64)))).create_async().await;
        let _m3 = s.mock("GET", "/o/r/releases/download/v9.9.0/Shadow-Client-Setup.exe").with_body(body.clone()).expect_at_least(1).create_async().await;
        let d = tempfile::tempdir().unwrap();
        let (ctx, _) = crate::testutil::ctx(d.path(), &s.url());

        let u = check_update_at(&ctx, &format!("{}/good/releases/latest", s.url()), &prefix).await.unwrap();
        let inst = u.installer.as_ref().expect("installer found");
        let msi = u.msi.as_ref().expect("the MSI is found too");
        assert!(msi.url.ends_with("/Shadow-Client.msi") && msi.sha256 == good && msi.size == 5);
        assert!(inst.url.ends_with("/Shadow-Client-Setup.exe") && inst.sha256 == good && inst.size == body.len() as u64);
        let path = download_installer(&ctx, &u, false).await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), body);
        assert!(path.ends_with("Shadow-Client-9.9.0-setup.exe"));

        // A file that doesn't match the published SHA-256 is deleted, never handed back.
        let bad = check_update_at(&ctx, &format!("{}/bad/releases/latest", s.url()), &prefix).await.unwrap();
        assert!(matches!(download_installer(&ctx, &bad, false).await, Err(Error::HashMismatch { .. })));
        assert!(!path.exists());

        // An installer hosted anywhere but the repository's own release downloads is ignored.
        let elsewhere = check_update_at(&ctx, &format!("{}/good/releases/latest", s.url()), "https://github.com/o/r/releases/download/").await.unwrap();
        assert!(elsewhere.installer.is_none());
        assert!(download_installer(&ctx, &elsewhere, false).await.is_err());
        assert!(download_installer(&ctx, &elsewhere, true).await.is_err());
    }
}
