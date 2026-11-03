//! Official Minecraft Launcher integration.
//!
//! * Detects the Microsoft Store and classic launchers and opens them.
//! * Optionally registers a Shadow Client Fabric/Quilt profile with the official launcher
//!   the same way the official Fabric installer does: copy the loader's version JSON into
//!   `versions/` and add one entry to `launcher_profiles.json` (after taking a backup). No
//!   other launcher file is touched, and account/session files are never read.
//! * With "only this profile" on, the launcher's other installations are moved into
//!   `launcher_profiles.shadowclient-hidden.json` next to it (nothing is deleted), so the opened
//!   profile is the only one next to PLAY; [`show_all`] puts them back.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{util, Error, Result};

pub const STORE_PACKAGE: &str = "Microsoft.4297127D64EC6_8wekyb3d8bbwe";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OfficialLauncher {
    pub installed: bool,
    /// "store" | "classic" | "none"
    pub kind: String,
    pub path: Option<PathBuf>,
    pub minecraft_dir: PathBuf,
    pub minecraft_dir_exists: bool,
}

pub fn detect(minecraft_dir: &Path) -> OfficialLauncher {
    let store = dirs::data_local_dir().map(|d| d.join("Packages").join(STORE_PACKAGE)).filter(|p| p.is_dir());
    let classic = [std::env::var_os("ProgramFiles(x86)"), std::env::var_os("ProgramFiles")]
        .into_iter()
        .flatten()
        .map(|pf| PathBuf::from(pf).join("Minecraft Launcher").join("MinecraftLauncher.exe"))
        .find(|p| p.is_file());
    let (installed, kind, path) = match (store, classic) {
        (Some(p), _) => (true, "store", Some(p)),
        (None, Some(p)) => (true, "classic", Some(p)),
        _ => (false, "none", None),
    };
    OfficialLauncher { installed, kind: kind.into(), path, minecraft_dir: minecraft_dir.to_path_buf(), minecraft_dir_exists: minecraft_dir.is_dir() }
}

/// Open the official launcher (or its download page if it is not installed).
pub fn open(l: &OfficialLauncher) -> Result<String> {
    match l.kind.as_str() {
        "store" => {
            std::process::Command::new("explorer.exe")
                .arg(format!("shell:AppsFolder\\{STORE_PACKAGE}!Minecraft"))
                .spawn()
                .map_err(|e| Error::msg(format!("Could not open the Minecraft Launcher: {e}")))?;
            Ok("Opening the Minecraft Launcher…".into())
        }
        "classic" => {
            let p = l.path.clone().ok_or_else(|| Error::NotFound("MinecraftLauncher.exe".into()))?;
            std::process::Command::new(p).spawn().map_err(|e| Error::msg(format!("Could not open the Minecraft Launcher: {e}")))?;
            Ok("Opening the Minecraft Launcher…".into())
        }
        _ => Err(Error::NotFound("The official Minecraft Launcher is not installed. Get it from https://www.minecraft.net/download".into())),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterReport {
    pub profile_key: String,
    pub version_id: String,
    pub backup: Option<PathBuf>,
    /// The user's own installations put aside by "only this profile"
    #[serde(default)]
    pub hidden: usize,
}

/// Launcher profile files, in the order they are updated. The Microsoft Store launcher can keep its
/// own file; whichever of these exist get the entry.
const PROFILE_FILES: [&str; 2] = ["launcher_profiles.json", "launcher_profiles_microsoft_store.json"];
/// Where installations wait while only a Shadow Client profile is shown (in the launcher's folder).
pub const HIDDEN_FILE: &str = "launcher_profiles.shadowclient-hidden.json";
/// Entries Shadow Client added itself (one per Shadow Client profile opened there).
const OWN_PREFIX: &str = "shadowclient-";
/// Backups kept per profile file; older Shadow Client backups are removed.
const KEEP_BACKUPS: usize = 10;

/// Installations put aside, per launcher profile file.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Hidden {
    #[serde(default)]
    note: String,
    #[serde(default)]
    profiles: serde_json::Map<String, serde_json::Value>,
}

impl Hidden {
    fn load(minecraft_dir: &Path) -> Result<Hidden> {
        Ok(util::read_json_opt::<Hidden>(&minecraft_dir.join(HIDDEN_FILE))?.unwrap_or_default())
    }

    fn save(&mut self, minecraft_dir: &Path) -> Result<()> {
        self.note = "Minecraft Launcher installations Shadow Client put aside so only the profile it opened shows next to PLAY. \
                     Turn off \"Only show the profile you open\" in Shadow Client (Settings → General) to put them back."
            .into();
        util::write_json(&minecraft_dir.join(HIDDEN_FILE), self)
    }

    fn file(&mut self, file_name: &str) -> &mut serde_json::Map<String, serde_json::Value> {
        let slot = self.profiles.entry(file_name.to_string()).or_insert_with(|| serde_json::json!({}));
        if !slot.is_object() {
            *slot = serde_json::json!({});
        }
        slot.as_object_mut().expect("just made an object")
    }
}

fn is_latest(p: &serde_json::Value) -> Option<&str> {
    p["type"].as_str().filter(|t| t.starts_with("latest-"))
}

/// Copy a launcher file next to itself before changing it, keeping the newest few such copies.
fn backup(path: &Path) -> Result<PathBuf> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let file_name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let prefix = format!("{file_name}.shadowclient-");
    let mut to = dir.join(format!("{prefix}{}.bak", util::timestamp_slug()));
    let mut n = 1;
    while to.exists() {
        n += 1;
        to = dir.join(format!("{prefix}{}-{n}.bak", util::timestamp_slug()));
    }
    std::fs::copy(path, &to)?;
    let mut old: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)?
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.starts_with(&prefix) && n.ends_with(".bak") && e.path() != to
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    old.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, p) in old.into_iter().skip(KEEP_BACKUPS - 1) {
        let _ = std::fs::remove_file(p);
    }
    Ok(to)
}

/// Move every installation but `keep` out of `doc`: the user's own into `hidden`, Shadow Client's
/// other entries away for good (they come back when that profile is opened). A "Latest release"
/// the launcher made again while one is already put aside is dropped instead of kept twice.
/// Returns (anything changed, how many of the user's installations were put aside).
fn put_aside(doc: &mut serde_json::Value, keep: &str, hidden: &mut serde_json::Map<String, serde_json::Value>) -> (bool, usize) {
    let Some(profiles) = doc["profiles"].as_object_mut() else { return (false, 0) };
    let others: Vec<String> = profiles.keys().filter(|k| *k != keep).cloned().collect();
    let mut moved = 0;
    for k in &others {
        let p = profiles.remove(k).expect("listed above");
        if k.starts_with(OWN_PREFIX) {
            continue;
        }
        if let Some(t) = is_latest(&p) {
            if hidden.iter().any(|(hk, hp)| hk != k && is_latest(hp) == Some(t)) {
                continue;
            }
        }
        hidden.insert(k.clone(), p);
        moved += 1;
    }
    (!others.is_empty(), moved)
}

/// Register a loader profile with the official launcher, or refresh it if it is already there.
/// `version_json` is the loader's own profile JSON (inheriting from vanilla); `None` for vanilla,
/// which the launcher downloads itself. A file is backed up before the entry is added or changed;
/// when it is already up to date only its `lastUsed` moves to now, so the launcher picks it first.
/// `only_this`: put the launcher's other installations aside (see [`show_all`]).
pub fn register_profile(minecraft_dir: &Path, name: &str, version_id: &str, version_json: Option<&serde_json::Value>, game_dir: &Path, icon: Option<&str>, only_this: bool) -> Result<RegisterReport> {
    let files: Vec<PathBuf> = PROFILE_FILES.iter().map(|f| minecraft_dir.join(f)).filter(|p| p.is_file()).collect();
    if files.is_empty() {
        return Err(Error::NotFound("launcher_profiles.json was not found. Start the official launcher once, then try again.".into()));
    }
    let segs = crate::safepath::validate_relative(version_id)?;
    if segs.len() != 1 {
        return Err(Error::UnsafePath(version_id.into()));
    }
    // Read and check every file before changing any of them.
    let mut docs = Vec::new();
    for path in &files {
        let doc: serde_json::Value = util::read_json(path)?;
        if !doc["profiles"].is_object() {
            return Err(Error::Invalid(format!("{} has an unexpected format; nothing was changed.", path.file_name().unwrap_or_default().to_string_lossy())));
        }
        docs.push(doc);
    }

    // Version JSON, without nulls: the launcher reads `"assetIndex": null` as "inherit nothing" and then
    // can't prepare assets. An existing file is only replaced when it has nulls (one written by 0.2.1 before
    // this fix); real Mojang/Fabric files never contain them.
    if let Some(json) = version_json {
        let vdir = minecraft_dir.join("versions").join(version_id);
        let vfile = vdir.join(format!("{version_id}.json"));
        let broken = vfile.exists() && util::read_json::<serde_json::Value>(&vfile).map(|old| contains_null(&old)).unwrap_or(false);
        if !vfile.exists() || broken {
            std::fs::create_dir_all(&vdir)?;
            util::write_json(&vfile, &without_nulls(json))?;
        }
    }

    let key = format!("shadowclient-{}", util::sha1_hex(format!("{name}{}", game_dir.display()).as_bytes())[..12].to_string());
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let game_dir_s = game_dir.to_string_lossy();
    let mut first_backup = None;
    let mut hidden = if only_this { Some(Hidden::load(minecraft_dir)?) } else { None };
    let mut moved = 0;
    for (path, mut doc) in files.iter().zip(docs) {
        let old = &doc["profiles"][&key];
        let same = old["name"] == name && old["lastVersionId"] == version_id && old["gameDir"] == game_dir_s.as_ref();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let mut aside = false;
        if let Some(h) = hidden.as_mut() {
            let (changed, n) = put_aside(&mut doc, &key, h.file(&file_name));
            aside = changed;
            moved += n;
            if n > 0 {
                // saved before the launcher file changes, so an installation is never only in memory
                h.save(minecraft_dir)?;
            }
        }
        if !same || aside {
            first_backup.get_or_insert(backup(path)?);
        }
        let old = &doc["profiles"][&key];
        if same {
            doc["profiles"][&key]["lastUsed"] = now.clone().into();
        } else {
            let created = old["created"].as_str().map(str::to_string).unwrap_or_else(|| now.clone());
            doc["profiles"][&key] = serde_json::json!({
                "name": name,
                "type": "custom",
                "created": created,
                "lastUsed": now,
                "icon": icon.unwrap_or("Furnace"),
                "lastVersionId": version_id,
                "gameDir": game_dir_s,
            });
        }
        util::write_json(path, &doc)?;
    }
    Ok(RegisterReport { profile_key: key, version_id: version_id.into(), backup: first_backup, hidden: moved })
}

/// Put back every installation "only this profile" moved aside, then remove the file that held them.
/// One the user has again under the same key, or a "Latest release" the launcher made again, isn't
/// added twice. Returns how many came back.
pub fn show_all(minecraft_dir: &Path) -> Result<usize> {
    if !minecraft_dir.join(HIDDEN_FILE).is_file() {
        return Ok(0);
    }
    let hidden = Hidden::load(minecraft_dir)?;
    let mut restored = 0;
    for f in PROFILE_FILES {
        let path = minecraft_dir.join(f);
        let Some(aside) = hidden.profiles.get(f).and_then(|v| v.as_object()).filter(|m| !m.is_empty()) else { continue };
        if !path.is_file() {
            continue;
        }
        let mut doc: serde_json::Value = util::read_json(&path)?;
        let Some(profiles) = doc["profiles"].as_object_mut() else {
            return Err(Error::Invalid(format!("{f} has an unexpected format; your installations are still in {HIDDEN_FILE}.")));
        };
        let mut back = 0;
        for (k, p) in aside {
            let again = is_latest(p).is_some_and(|t| profiles.values().any(|q| is_latest(q) == Some(t)));
            if again || profiles.contains_key(k) {
                continue;
            }
            profiles.insert(k.clone(), p.clone());
            back += 1;
        }
        if back > 0 {
            backup(&path)?;
            util::write_json(&path, &doc)?;
            restored += back;
        }
    }
    std::fs::remove_file(minecraft_dir.join(HIDDEN_FILE))?;
    Ok(restored)
}

/// A copy of `value` with every null field and array element removed, at any depth.
pub fn without_nulls(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), without_nulls(v))).collect(),
        ),
        serde_json::Value::Array(items) => serde_json::Value::Array(items.iter().filter(|v| !v.is_null()).map(without_nulls).collect()),
        other => other.clone(),
    }
}

fn contains_null(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::Object(map) => map.values().any(contains_null),
        serde_json::Value::Array(items) => items.iter().any(contains_null),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_only_adds_one_profile_and_backs_up() {
        let d = tempfile::tempdir().unwrap();
        let mc = d.path().join(".minecraft");
        std::fs::create_dir_all(&mc).unwrap();
        let original = r#"{"profiles":{"x":{"name":"Mine","type":"custom","lastVersionId":"1.21.11"}},"settings":{"keepLauncherOpen":true},"version":6}"#;
        std::fs::write(mc.join("launcher_profiles.json"), original).unwrap();
        let vj = serde_json::json!({"id":"fabric-loader-0.17.3-1.21.11","inheritsFrom":"1.21.11"});
        let r = register_profile(&mc, "PC Fabric", "fabric-loader-0.17.3-1.21.11", Some(&vj), &d.path().join("inst"), None, false).unwrap();
        let doc: serde_json::Value = util::read_json(&mc.join("launcher_profiles.json")).unwrap();
        assert_eq!(doc["profiles"]["x"]["name"], "Mine", "existing profile untouched");
        assert_eq!(doc["settings"]["keepLauncherOpen"], true, "settings untouched");
        assert_eq!(doc["profiles"][&r.profile_key]["lastVersionId"], "fabric-loader-0.17.3-1.21.11");
        assert_eq!(std::fs::read_to_string(r.backup.unwrap()).unwrap(), original);
        assert!(mc.join("versions/fabric-loader-0.17.3-1.21.11/fabric-loader-0.17.3-1.21.11.json").exists());
        assert!(register_profile(&mc, "x", "../evil", Some(&vj), d.path(), None, false).is_err());
    }

    #[test]
    fn registering_again_only_moves_last_used() {
        let d = tempfile::tempdir().unwrap();
        let mc = d.path().join(".minecraft");
        std::fs::create_dir_all(&mc).unwrap();
        std::fs::write(mc.join("launcher_profiles.json"), r#"{"profiles":{}}"#).unwrap();
        let vj = serde_json::json!({"id":"fabric-loader-0.17.3-1.21.11","inheritsFrom":"1.21.11"});
        let inst = d.path().join("inst");
        let first = register_profile(&mc, "PvP (Shadow Client)", "fabric-loader-0.17.3-1.21.11", Some(&vj), &inst, None, false).unwrap();
        assert!(first.backup.is_some());
        let created = util::read_json::<serde_json::Value>(&mc.join("launcher_profiles.json")).unwrap()["profiles"][&first.profile_key]["created"].clone();
        let again = register_profile(&mc, "PvP (Shadow Client)", "fabric-loader-0.17.3-1.21.11", Some(&vj), &inst, None, false).unwrap();
        assert_eq!(again.profile_key, first.profile_key, "same entry, not a second one");
        assert!(again.backup.is_none(), "nothing changed, so no new backup");
        let doc: serde_json::Value = util::read_json(&mc.join("launcher_profiles.json")).unwrap();
        assert_eq!(doc["profiles"].as_object().unwrap().len(), 1);
        assert_eq!(doc["profiles"][&first.profile_key]["created"], created);
        let backups = std::fs::read_dir(&mc).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".bak")).count();
        assert_eq!(backups, 1);
    }

    #[test]
    fn store_launcher_file_gets_the_entry_too() {
        let d = tempfile::tempdir().unwrap();
        let mc = d.path().join(".minecraft");
        std::fs::create_dir_all(&mc).unwrap();
        std::fs::write(mc.join("launcher_profiles.json"), r#"{"profiles":{}}"#).unwrap();
        std::fs::write(mc.join("launcher_profiles_microsoft_store.json"), r#"{"profiles":{"s":{"name":"Store"}}}"#).unwrap();
        let r = register_profile(&mc, "Vanilla (Shadow Client)", "1.21.11", None, &d.path().join("inst"), None, false).unwrap();
        for f in PROFILE_FILES {
            let doc: serde_json::Value = util::read_json(&mc.join(f)).unwrap();
            assert_eq!(doc["profiles"][&r.profile_key]["lastVersionId"], "1.21.11", "{f}");
        }
        let store: serde_json::Value = util::read_json(&mc.join("launcher_profiles_microsoft_store.json")).unwrap();
        assert_eq!(store["profiles"]["s"]["name"], "Store");
        assert!(!mc.join("versions").exists(), "vanilla writes no version file");
    }

    #[test]
    fn version_file_has_no_nulls_and_old_broken_files_are_replaced() {
        let d = tempfile::tempdir().unwrap();
        let mc = d.path().join(".minecraft");
        std::fs::create_dir_all(mc.join("versions/fabric-loader-0.20.1-26.1")).unwrap();
        std::fs::write(mc.join("launcher_profiles.json"), r#"{"profiles":{}}"#).unwrap();
        // What 0.2.1 wrote: nulls that make the launcher fail with "Unable to prepare assets for download".
        let vfile = mc.join("versions/fabric-loader-0.20.1-26.1/fabric-loader-0.20.1-26.1.json");
        std::fs::write(&vfile, r#"{"id":"fabric-loader-0.20.1-26.1","inheritsFrom":"26.1","assetIndex":null,"libraries":[{"name":"a:b:1","rules":null}]}"#).unwrap();
        let vj = serde_json::json!({"id":"fabric-loader-0.20.1-26.1","inheritsFrom":"26.1","assetIndex":null,"assets":null,
            "libraries":[{"name":"org.ow2.asm:asm:9.10.1","url":"https://maven.fabricmc.net/","downloads":null,"rules":null}]});
        register_profile(&mc, "X (Shadow Client)", "fabric-loader-0.20.1-26.1", Some(&vj), &d.path().join("inst"), None, false).unwrap();
        let written: serde_json::Value = util::read_json(&vfile).unwrap();
        assert!(!contains_null(&written), "{written}");
        assert!(written.get("assetIndex").is_none() && written.get("assets").is_none());
        assert_eq!(written["libraries"][0]["url"], "https://maven.fabricmc.net/");
        assert!(written["libraries"][0].get("rules").is_none());
        assert_eq!(written["inheritsFrom"], "26.1");
        // A normal file (no nulls) that is already there is left alone.
        std::fs::write(&vfile, r#"{"id":"fabric-loader-0.20.1-26.1","inheritsFrom":"26.1","custom":true}"#).unwrap();
        register_profile(&mc, "X (Shadow Client)", "fabric-loader-0.20.1-26.1", Some(&vj), &d.path().join("inst"), None, false).unwrap();
        let kept: serde_json::Value = util::read_json(&vfile).unwrap();
        assert_eq!(kept["custom"], true);
    }

    #[test]
    fn only_this_profile_puts_the_others_aside_and_show_all_brings_them_back() {
        let d = tempfile::tempdir().unwrap();
        let mc = d.path().join(".minecraft");
        std::fs::create_dir_all(&mc).unwrap();
        let original = r#"{"profiles":{
            "a1":{"name":"","type":"latest-release","lastVersionId":"latest-release"},
            "b2":{"name":"My Forge","type":"custom","lastVersionId":"1.20.1-forge"},
            "shadowclient-000000000000":{"name":"Old (Shadow Client)","type":"custom","lastVersionId":"1.21.11"}},
            "settings":{"keepLauncherOpen":true},"version":3}"#;
        std::fs::write(mc.join("launcher_profiles.json"), original).unwrap();
        let vj = serde_json::json!({"id":"fabric-loader-0.17.3-1.21.11","inheritsFrom":"1.21.11"});
        let r = register_profile(&mc, "PvP (Shadow Client)", "fabric-loader-0.17.3-1.21.11", Some(&vj), &d.path().join("pvp"), None, true).unwrap();
        assert_eq!(r.hidden, 2, "the user's two installations");
        let doc: serde_json::Value = util::read_json(&mc.join("launcher_profiles.json")).unwrap();
        let keys: Vec<&String> = doc["profiles"].as_object().unwrap().keys().collect();
        assert_eq!(keys, vec![&r.profile_key], "only the opened profile is left");
        assert_eq!(doc["settings"]["keepLauncherOpen"], true, "settings untouched");
        assert_eq!(std::fs::read_to_string(r.backup.unwrap()).unwrap(), original);

        // Opening another Shadow Client profile swaps the entry; the user's installations stay put aside.
        let r2 = register_profile(&mc, "Survival (Shadow Client)", "1.21.11", None, &d.path().join("surv"), None, true).unwrap();
        assert_eq!(r2.hidden, 0);
        let mut doc: serde_json::Value = util::read_json(&mc.join("launcher_profiles.json")).unwrap();
        assert_eq!(doc["profiles"].as_object().unwrap().len(), 1);
        assert!(doc["profiles"][&r2.profile_key].is_object());

        // The launcher made "Latest release" again meanwhile: it isn't kept twice.
        doc["profiles"]["c3"] = serde_json::json!({"name":"","type":"latest-release","lastVersionId":"latest-release"});
        util::write_json(&mc.join("launcher_profiles.json"), &doc).unwrap();
        assert_eq!(register_profile(&mc, "Survival (Shadow Client)", "1.21.11", None, &d.path().join("surv"), None, true).unwrap().hidden, 0);
        let hidden: serde_json::Value = util::read_json(&mc.join(HIDDEN_FILE)).unwrap();
        assert_eq!(hidden["profiles"]["launcher_profiles.json"].as_object().unwrap().len(), 2);

        assert_eq!(show_all(&mc).unwrap(), 2);
        let doc: serde_json::Value = util::read_json(&mc.join("launcher_profiles.json")).unwrap();
        assert_eq!(doc["profiles"]["b2"]["name"], "My Forge");
        assert_eq!(doc["profiles"]["a1"]["type"], "latest-release");
        assert!(doc["profiles"][&r2.profile_key].is_object(), "the Shadow Client profile stays");
        assert!(doc["profiles"].get("shadowclient-000000000000").is_none());
        assert!(!mc.join(HIDDEN_FILE).exists());
        assert_eq!(show_all(&mc).unwrap(), 0, "nothing left to bring back");
    }

    #[test]
    fn show_all_doesnt_double_latest_release_or_overwrite() {
        let d = tempfile::tempdir().unwrap();
        let mc = d.path().join(".minecraft");
        std::fs::create_dir_all(&mc).unwrap();
        std::fs::write(mc.join("launcher_profiles.json"), r#"{"profiles":{"a1":{"type":"latest-release"},"b2":{"name":"Mine","type":"custom"}}}"#).unwrap();
        register_profile(&mc, "X (Shadow Client)", "1.21.11", None, &d.path().join("x"), None, true).unwrap();
        // the launcher remade Latest release under a new key, and the user made "b2" again with other settings
        std::fs::write(mc.join("launcher_profiles.json"), r#"{"profiles":{"z9":{"type":"latest-release"},"b2":{"name":"Mine v2","type":"custom"}}}"#).unwrap();
        assert_eq!(show_all(&mc).unwrap(), 0);
        let doc: serde_json::Value = util::read_json(&mc.join("launcher_profiles.json")).unwrap();
        assert_eq!(doc["profiles"].as_object().unwrap().len(), 2);
        assert_eq!(doc["profiles"]["b2"]["name"], "Mine v2");
    }

    #[test]
    fn backups_are_kept_to_the_newest_few() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("launcher_profiles.json");
        std::fs::write(&f, "{}").unwrap();
        for _ in 0..(KEEP_BACKUPS + 3) {
            backup(&f).unwrap();
        }
        let n = std::fs::read_dir(d.path()).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".bak")).count();
        assert_eq!(n, KEEP_BACKUPS);
    }

    #[test]
    fn refuses_without_profiles_file() {
        let d = tempfile::tempdir().unwrap();
        assert!(register_profile(d.path(), "x", "v", None, d.path(), None, false).is_err());
    }
}
