//! Read-only detection of existing Minecraft installations.
//!
//! Scans the official launcher folder (and user-chosen folders) for launcher profiles with
//! their real game directories, installed versions/loaders, mods, resource packs, shader
//! packs, worlds, screenshots and configs. Instances of other launchers are listed too.
//! Account/credential files (`launcher_accounts*.json`, `*credentials*`) are never opened.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::manifest::{self, VersionJson};
use crate::util;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherProfile {
    pub key: String,
    pub name: String,
    pub kind: String,
    pub last_version_id: Option<String>,
    /// As written in launcher_profiles.json (None = the launcher folder itself)
    pub game_dir: Option<PathBuf>,
    /// The folder the game actually uses
    pub resolved_game_dir: PathBuf,
    pub java_dir: Option<String>,
    pub java_args: Option<String>,
    pub last_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DetectedVersion {
    pub id: String,
    pub inherits_from: Option<String>,
    pub loader: String,
    pub loader_version: Option<String>,
    pub has_jar: bool,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub folder: String,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameDirContents {
    pub path: PathBuf,
    pub exists: bool,
    pub mods: Vec<String>,
    pub resourcepacks: Vec<String>,
    pub shaderpacks: Vec<String>,
    pub worlds: Vec<World>,
    pub screenshots: usize,
    pub has_config: bool,
    pub has_options: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DetectedInstall {
    pub root: PathBuf,
    /// "official" | "custom" | "curseforge" | "prism" | "modrinth-app" | "multimc"
    pub source: String,
    pub label: String,
    pub launcher_profiles: Vec<LauncherProfile>,
    pub versions: Vec<DetectedVersion>,
    /// Contents of every distinct game directory referenced by this install
    pub game_dirs: Vec<GameDirContents>,
    /// For instances of other launchers: Minecraft version and loader if known
    pub instance_version: Option<String>,
    pub instance_loader: Option<String>,
    pub notes: Vec<String>,
}

fn list_names(dir: &Path, filter: &dyn Fn(&std::fs::DirEntry) -> bool) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().filter(|e| filter(e)).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    v.sort_by_key(|s| s.to_lowercase());
    v
}

pub fn scan_game_dir(dir: &Path) -> GameDirContents {
    let exists = dir.is_dir();
    let mods = list_names(&dir.join("mods"), &|e| {
        let n = e.file_name().to_string_lossy().to_ascii_lowercase();
        n.ends_with(".jar") || n.ends_with(".jar.disabled")
    });
    let packs = |sub: &str| list_names(&dir.join(sub), &|e| e.path().is_dir() || e.file_name().to_string_lossy().to_ascii_lowercase().ends_with(".zip"));
    let worlds = std::fs::read_dir(dir.join("saves"))
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().join("level.dat").is_file())
                .map(|e| World {
                    folder: e.file_name().to_string_lossy().into_owned(),
                    last_modified: std::fs::metadata(e.path().join("level.dat"))
                        .and_then(|m| m.modified())
                        .ok()
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                })
                .collect()
        })
        .unwrap_or_default();
    let screenshots = std::fs::read_dir(dir.join("screenshots")).map(|rd| rd.flatten().count()).unwrap_or(0);
    GameDirContents {
        path: dir.to_path_buf(),
        exists,
        mods,
        resourcepacks: packs("resourcepacks"),
        shaderpacks: packs("shaderpacks"),
        worlds,
        screenshots,
        has_config: dir.join("config").is_dir(),
        has_options: dir.join("options.txt").is_file(),
    }
}

/// Parse launcher_profiles.json (profiles only; never account data).
pub fn read_launcher_profiles(root: &Path) -> Vec<LauncherProfile> {
    let Ok(Some(v)) = util::read_json_opt::<serde_json::Value>(&root.join("launcher_profiles.json")) else { return vec![] };
    let Some(profiles) = v["profiles"].as_object() else { return vec![] };
    let mut out: Vec<LauncherProfile> = profiles
        .iter()
        .map(|(key, p)| {
            let kind = p["type"].as_str().unwrap_or("custom").to_string();
            let name = p["name"].as_str().filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(|| match kind.as_str() {
                "latest-release" => "Latest release".into(),
                "latest-snapshot" => "Latest snapshot".into(),
                _ => key.clone(),
            });
            let game_dir = p["gameDir"].as_str().filter(|s| !s.trim().is_empty()).map(PathBuf::from);
            LauncherProfile {
                key: key.clone(),
                name,
                kind,
                last_version_id: p["lastVersionId"].as_str().map(String::from),
                resolved_game_dir: game_dir.clone().unwrap_or_else(|| root.to_path_buf()),
                game_dir,
                java_dir: p["javaDir"].as_str().map(String::from),
                java_args: p["javaArgs"].as_str().map(String::from),
                last_used: p["lastUsed"].as_str().map(String::from),
            }
        })
        .collect();
    out.sort_by(|a, b| b.last_used.cmp(&a.last_used));
    out
}

pub fn read_versions(root: &Path) -> Vec<DetectedVersion> {
    let mut out = vec![];
    for e in std::fs::read_dir(root.join("versions")).into_iter().flatten().flatten() {
        let id = e.file_name().to_string_lossy().into_owned();
        let json = e.path().join(format!("{id}.json"));
        let Ok(v) = util::read_json::<VersionJson>(&json) else { continue };
        let (loader, loader_version) = manifest::detect_loader(&v);
        out.push(DetectedVersion {
            has_jar: e.path().join(format!("{id}.jar")).is_file(),
            inherits_from: v.inherits_from.clone(),
            loader: loader.into(),
            loader_version,
            kind: v.kind.clone(),
            id,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Scan a `.minecraft`-style folder.
pub fn scan_minecraft_root(root: &Path, source: &str, label: &str) -> Option<DetectedInstall> {
    if !root.is_dir() {
        return None;
    }
    let launcher_profiles = read_launcher_profiles(root);
    let versions = read_versions(root);
    let mut dirs: Vec<PathBuf> = vec![root.to_path_buf()];
    for p in &launcher_profiles {
        if !dirs.iter().any(|d| crate::paths::canonical_or_self(d) == crate::paths::canonical_or_self(&p.resolved_game_dir)) {
            dirs.push(p.resolved_game_dir.clone());
        }
    }
    let game_dirs: Vec<GameDirContents> = dirs.iter().map(|d| scan_game_dir(d)).collect();
    let mut notes = vec![];
    if !root.join("libraries").is_dir() && !versions.is_empty() {
        notes.push("This folder has version files but no libraries/assets (the Microsoft Store launcher keeps game files elsewhere). Shadow Client downloads its own verified copy when launching.".into());
    }
    if launcher_profiles.is_empty() && versions.is_empty() && !game_dirs[0].has_options {
        return None;
    }
    Some(DetectedInstall { root: root.to_path_buf(), source: source.into(), label: label.into(), launcher_profiles, versions, game_dirs, instance_version: None, instance_loader: None, notes })
}

fn instance(root: PathBuf, game_dir: PathBuf, source: &str, label: String, version: Option<String>, loader: Option<String>) -> DetectedInstall {
    DetectedInstall {
        game_dirs: vec![scan_game_dir(&game_dir)],
        root,
        source: source.into(),
        label,
        launcher_profiles: vec![],
        versions: vec![],
        instance_version: version,
        instance_loader: loader,
        notes: vec![],
    }
}

/// CurseForge app instances (`minecraftinstance.json`).
fn curseforge_instances(base: &Path) -> Vec<DetectedInstall> {
    let mut out = vec![];
    for e in std::fs::read_dir(base.join("minecraft").join("Instances")).into_iter().flatten().flatten() {
        let dir = e.path();
        let meta: Option<serde_json::Value> = util::read_json_opt(&dir.join("minecraftinstance.json")).ok().flatten();
        let version = meta.as_ref().and_then(|m| m["gameVersion"].as_str().map(String::from));
        let loader = meta.as_ref().and_then(|m| m["baseModLoader"]["name"].as_str().map(|s| s.split('-').next().unwrap_or(s).to_string()));
        let name = meta.as_ref().and_then(|m| m["name"].as_str().map(String::from)).unwrap_or_else(|| e.file_name().to_string_lossy().into_owned());
        out.push(instance(dir.clone(), dir, "curseforge", format!("CurseForge: {name}"), version, loader));
    }
    out
}

/// Prism Launcher / MultiMC instances (`mmc-pack.json`).
fn prism_instances(base: &Path, source: &str, title: &str) -> Vec<DetectedInstall> {
    let mut out = vec![];
    for e in std::fs::read_dir(base.join("instances")).into_iter().flatten().flatten() {
        let dir = e.path();
        let pack: Option<serde_json::Value> = util::read_json_opt(&dir.join("mmc-pack.json")).ok().flatten();
        let Some(pack) = pack else { continue };
        let comps = pack["components"].as_array().cloned().unwrap_or_default();
        let find = |uid: &str| comps.iter().find(|c| c["uid"] == uid).and_then(|c| c["version"].as_str().map(String::from));
        let version = find("net.minecraft");
        let loader = if find("net.fabricmc.fabric-loader").is_some() {
            Some("fabric".to_string())
        } else if find("org.quiltmc.quilt-loader").is_some() {
            Some("quilt".into())
        } else if find("net.neoforged").is_some() {
            Some("neoforge".into())
        } else if find("net.minecraftforge").is_some() {
            Some("forge".into())
        } else {
            Some("vanilla".into())
        };
        let game_dir = if dir.join(".minecraft").is_dir() { dir.join(".minecraft") } else { dir.join("minecraft") };
        out.push(instance(dir.clone(), game_dir, source, format!("{title}: {}", e.file_name().to_string_lossy()), version, loader));
    }
    out
}

/// Modrinth App profiles (folder listing only; its metadata lives in a database).
fn modrinth_app_profiles(base: &Path) -> Vec<DetectedInstall> {
    std::fs::read_dir(base.join("profiles"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| instance(e.path(), e.path(), "modrinth-app", format!("Modrinth App: {}", e.file_name().to_string_lossy()), None, None))
        .collect()
}

/// Detect installations in the default locations plus `extra` folders.
pub fn detect_all(official_root: &Path, extra: &[PathBuf]) -> Vec<DetectedInstall> {
    let mut out = vec![];
    if let Some(d) = scan_minecraft_root(official_root, "official", "Official Minecraft Launcher") {
        out.push(d);
    }
    for dir in extra {
        if crate::paths::canonical_or_self(dir) == crate::paths::canonical_or_self(official_root) {
            continue;
        }
        match scan_minecraft_root(dir, "custom", &format!("Folder: {}", dir.display())) {
            Some(d) => out.push(d),
            None if dir.is_dir() => out.push(instance(dir.clone(), dir.clone(), "custom", format!("Folder: {}", dir.display()), None, None)),
            None => {}
        }
    }
    if let Some(home) = dirs::home_dir() {
        out.extend(curseforge_instances(&home.join("curseforge")));
    }
    if let Some(appdata) = dirs::config_dir() {
        out.extend(prism_instances(&appdata.join("PrismLauncher"), "prism", "Prism"));
        out.extend(prism_instances(&appdata.join("MultiMC"), "multimc", "MultiMC"));
        out.extend(modrinth_app_profiles(&appdata.join("ModrinthApp")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(root: &Path) {
        std::fs::create_dir_all(root.join("versions").join("1.21.11")).unwrap();
        std::fs::write(root.join("versions/1.21.11/1.21.11.json"), r#"{"id":"1.21.11","type":"release","mainClass":"net.minecraft.client.main.Main"}"#).unwrap();
        std::fs::create_dir_all(root.join("versions").join("fabric-loader-0.17.3-1.21.11")).unwrap();
        std::fs::write(
            root.join("versions/fabric-loader-0.17.3-1.21.11/fabric-loader-0.17.3-1.21.11.json"),
            r#"{"id":"fabric-loader-0.17.3-1.21.11","inheritsFrom":"1.21.11","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient","libraries":[{"name":"net.fabricmc:fabric-loader:0.17.3"}]}"#,
        )
        .unwrap();
        let other = root.join("custom-dir");
        std::fs::create_dir_all(other.join("mods")).unwrap();
        std::fs::write(other.join("mods/sodium.jar"), b"x").unwrap();
        std::fs::create_dir_all(other.join("saves/World 1")).unwrap();
        std::fs::write(other.join("saves/World 1/level.dat"), b"x").unwrap();
        std::fs::create_dir_all(root.join("resourcepacks")).unwrap();
        std::fs::write(root.join("resourcepacks/Faithful.zip"), b"x").unwrap();
        // credentials must never be read
        std::fs::write(root.join("launcher_accounts_microsoft_store.json"), b"SECRET").unwrap();
        let profiles = serde_json::json!({"profiles": {
            "a": {"name": "", "type": "latest-release", "lastVersionId": "latest-release"},
            "b": {"name": "Fabric PvP", "type": "custom", "lastVersionId": "fabric-loader-0.17.3-1.21.11", "gameDir": other.to_string_lossy()}
        }});
        std::fs::write(root.join("launcher_profiles.json"), profiles.to_string()).unwrap();
    }

    #[test]
    fn detects_profiles_versions_and_game_dirs() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join(".minecraft");
        fixture(&root);
        let inst = scan_minecraft_root(&root, "official", "Official").unwrap();
        assert_eq!(inst.launcher_profiles.len(), 2);
        let fabric = inst.launcher_profiles.iter().find(|p| p.name == "Fabric PvP").unwrap();
        assert_eq!(fabric.resolved_game_dir, root.join("custom-dir"));
        assert!(inst.versions.iter().any(|v| v.loader == "fabric" && v.loader_version.as_deref() == Some("0.17.3")));
        let custom = inst.game_dirs.iter().find(|g| g.path == root.join("custom-dir")).unwrap();
        assert_eq!(custom.mods, vec!["sodium.jar"]);
        assert_eq!(custom.worlds[0].folder, "World 1");
        let main = inst.game_dirs.iter().find(|g| g.path == root).unwrap();
        assert_eq!(main.resourcepacks, vec!["Faithful.zip"]);
        assert!(inst.notes.iter().any(|n| n.contains("no libraries")));
        // detection is read-only
        assert_eq!(std::fs::read(root.join("launcher_accounts_microsoft_store.json")).unwrap(), b"SECRET");
        assert!(!serde_json::to_string(&inst).unwrap().contains("SECRET"));
    }

    #[test]
    fn missing_root_is_none() {
        let d = tempfile::tempdir().unwrap();
        assert!(scan_minecraft_root(&d.path().join("nope"), "official", "x").is_none());
    }
}
