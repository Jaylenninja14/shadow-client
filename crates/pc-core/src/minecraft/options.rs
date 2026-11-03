//! `options.txt` editing.
//!
//! Lines are kept in order and unknown keys are preserved untouched. Only keys this module
//! understands are written, version-gated so older/newer formats are respected. A backup is
//! taken before every change, and the very first version is kept as `options.original.txt`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::mods::metadata::compare_versions;
use crate::settings::GraphicsSettings;
use crate::{util, Error, Result};

#[derive(Debug, Clone, PartialEq)]
enum Line {
    Kv(String, String),
    Other(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OptionsFile {
    lines: Vec<Line>,
}

impl OptionsFile {
    pub fn parse(text: &str) -> Self {
        let lines = text
            .lines()
            .map(|l| match l.split_once(':') {
                Some((k, v)) if !k.is_empty() && !k.contains(' ') => Line::Kv(k.to_string(), v.to_string()),
                _ => Line::Other(l.to_string()),
            })
            .collect();
        Self { lines }
    }

    pub fn load(path: &Path) -> Result<Option<Self>> {
        match std::fs::read_to_string(path) {
            Ok(t) => Ok(Some(Self::parse(&t))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn render(&self) -> String {
        let mut s = String::new();
        for l in &self.lines {
            match l {
                Line::Kv(k, v) => {
                    s.push_str(k);
                    s.push(':');
                    s.push_str(v);
                }
                Line::Other(o) => s.push_str(o),
            }
            s.push('\n');
        }
        s
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.lines.iter().find_map(|l| match l {
            Line::Kv(k, v) if k == key => Some(v.as_str()),
            _ => None,
        })
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn set(&mut self, key: &str, value: &str) {
        for l in self.lines.iter_mut() {
            if let Line::Kv(k, v) = l {
                if k == key {
                    *v = value.to_string();
                    return;
                }
            }
        }
        self.lines.push(Line::Kv(key.to_string(), value.to_string()));
    }

    pub fn keys(&self) -> Vec<String> {
        self.lines.iter().filter_map(|l| if let Line::Kv(k, _) = l { Some(k.clone()) } else { None }).collect()
    }
}

/// Is `mc` at least `min`? Unknown formats (e.g. old weekly snapshots) are treated as modern.
fn at_least(mc: &str, min: &str) -> bool {
    compare_versions(mc, min).is_none_or(|o| o != std::cmp::Ordering::Less)
}

/// Compute the key/value changes for graphics settings. Keys are only included when the
/// version supports them (or the existing file already has them).
pub fn graphics_changes(g: &GraphicsSettings, mc: &str, existing: Option<&OptionsFile>) -> Vec<(String, String)> {
    let has = |k: &str| existing.is_some_and(|e| e.has(k));
    let mut out: Vec<(String, String)> = vec![];
    let mut put = |k: &str, v: String| out.push((k.to_string(), v));
    put("renderDistance", g.render_distance.to_string());
    if has("simulationDistance") || at_least(mc, "1.18") {
        put("simulationDistance", g.simulation_distance.max(5).to_string());
    }
    put("particles", g.particles.min(2).to_string());
    put("maxFps", g.max_fps.clamp(10, 260).to_string());
    put("enableVsync", g.vsync.to_string());
    let fancy = g.graphics == "fancy";
    if has("graphicsMode") || at_least(mc, "1.16") {
        put("graphicsMode", if fancy { "1".into() } else { "0".into() });
    } else {
        put("fancyGraphics", fancy.to_string());
    }
    if at_least(mc, "1.13") || existing.and_then(|e| e.get("renderClouds")).is_some_and(|v| v.starts_with('"')) {
        let v = match g.clouds.as_str() {
            "off" => "\"false\"",
            "fast" => "\"fast\"",
            _ => "\"true\"",
        };
        put("renderClouds", v.into());
    } else {
        put("renderClouds", (g.clouds != "off").to_string());
    }
    // Smooth lighting: boolean since 1.19, numeric (0..2) before.
    let ao_numeric = existing.and_then(|e| e.get("ao")).is_some_and(|v| v.chars().all(|c| c.is_ascii_digit()));
    if ao_numeric || !at_least(mc, "1.19") {
        put("ao", if g.smooth_lighting { "2".into() } else { "0".into() });
    } else {
        put("ao", g.smooth_lighting.to_string());
    }
    put("mipmapLevels", g.mipmap_levels.min(4).to_string());
    put("entityShadows", g.entity_shadows.to_string());
    if has("biomeBlendRadius") || at_least(mc, "1.13") {
        put("biomeBlendRadius", g.biome_blend.min(7).to_string());
    }
    if has("entityDistanceScaling") || at_least(mc, "1.16") {
        put("entityDistanceScaling", format!("{:.2}", g.entity_distance as f64 / 100.0));
    }
    // Menu blur costs a lot on integrated GPUs; only touch it when the key exists.
    if has("menuBackgroundBlurriness") {
        let low = g.graphics == "fast";
        put("menuBackgroundBlurriness", if low { "0".into() } else { "5".into() });
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub changed: Vec<(String, String, String)>,
    pub backup: Option<PathBuf>,
    pub created: bool,
}

fn backup_dir(profile_data: &Path) -> PathBuf {
    profile_data.join("backups").join("options")
}

/// Back up options.txt (if present). The first ever backup is also kept as the original.
pub fn backup(game_dir: &Path, profile_data: &Path) -> Result<Option<PathBuf>> {
    let src = game_dir.join("options.txt");
    if !src.exists() {
        return Ok(None);
    }
    let dir = backup_dir(profile_data);
    std::fs::create_dir_all(&dir)?;
    let original = dir.join("options.original.txt");
    if !original.exists() {
        std::fs::copy(&src, &original)?;
    }
    let dest = util::unique_path(&dir.join(format!("options-{}.txt", util::timestamp_slug())));
    std::fs::copy(&src, &dest)?;
    // keep the 15 newest timestamped backups
    let mut stamped: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("options-")))
        .collect();
    stamped.sort();
    while stamped.len() > 15 {
        let _ = std::fs::remove_file(stamped.remove(0));
    }
    Ok(Some(dest))
}

/// Apply graphics settings to `<game_dir>/options.txt`.
/// `data_version` is written when the file has to be created, so Minecraft does not run
/// its legacy data fixers over the new file.
pub fn apply_graphics(game_dir: &Path, profile_data: &Path, g: &GraphicsSettings, mc: &str, data_version: Option<u32>) -> Result<ApplyReport> {
    crate::performance::presets::validate_graphics(g)?;
    let path = game_dir.join("options.txt");
    let existing = OptionsFile::load(&path)?;
    let backup = backup(game_dir, profile_data)?;
    let created = existing.is_none();
    let mut file = match existing.clone() {
        Some(f) => f,
        None => {
            let mut f = OptionsFile::default();
            match data_version {
                Some(v) => f.set("version", &v.to_string()),
                None => return Err(Error::Invalid("options.txt does not exist yet; start the game once (or launch from Shadow Client, which creates it safely)".into())),
            }
            f
        }
    };
    let mut changed = vec![];
    for (k, v) in graphics_changes(g, mc, existing.as_ref()) {
        let old = file.get(&k).unwrap_or("").to_string();
        if old != v {
            changed.push((k.clone(), old, v.clone()));
            file.set(&k, &v);
        }
    }
    if !changed.is_empty() || created {
        std::fs::create_dir_all(game_dir)?;
        util::atomic_write(&path, file.render().as_bytes())?;
    }
    Ok(ApplyReport { changed, backup, created })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsBackup {
    pub path: PathBuf,
    pub name: String,
    pub original: bool,
}

pub fn list_backups(profile_data: &Path) -> Vec<OptionsBackup> {
    let mut out: Vec<OptionsBackup> = std::fs::read_dir(backup_dir(profile_data))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            OptionsBackup { original: name == "options.original.txt", path: e.path(), name }
        })
        .collect();
    out.sort_by(|a, b| b.original.cmp(&a.original).then(b.name.cmp(&a.name)));
    out
}

/// Restore a backup (current file is backed up first so the restore itself can be undone).
pub fn restore(game_dir: &Path, profile_data: &Path, backup_name: &str) -> Result<()> {
    let segs = crate::safepath::validate_relative(backup_name)?;
    if segs.len() != 1 {
        return Err(Error::UnsafePath(backup_name.into()));
    }
    let src = backup_dir(profile_data).join(backup_name);
    if !src.is_file() {
        return Err(Error::NotFound(backup_name.into()));
    }
    backup(game_dir, profile_data)?;
    std::fs::copy(&src, game_dir.join("options.txt"))?;
    Ok(())
}

/// Read Minecraft's data version from the client jar's version.json.
pub fn data_version_from_jar(jar: &Path) -> Option<u32> {
    use std::io::Read;
    let f = std::fs::File::open(jar).ok()?;
    let mut z = zip::ZipArchive::new(f).ok()?;
    let mut e = z.by_name("version.json").ok()?;
    let mut s = String::new();
    e.read_to_string(&mut s).ok()?;
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    v["world_version"].as_u64().map(|x| x as u32)
}

/// Current graphics values from an options.txt (for display). Missing keys are None.
pub fn read_graphics(game_dir: &Path) -> Result<Option<serde_json::Value>> {
    let Some(f) = OptionsFile::load(&game_dir.join("options.txt"))? else { return Ok(None) };
    let keys = [
        "renderDistance", "simulationDistance", "particles", "maxFps", "enableVsync", "graphicsMode", "fancyGraphics", "renderClouds", "ao",
        "mipmapLevels", "entityShadows", "biomeBlendRadius", "entityDistanceScaling", "menuBackgroundBlurriness", "fullscreen",
    ];
    let mut m = serde_json::Map::new();
    for k in keys {
        if let Some(v) = f.get(k) {
            m.insert(k.to_string(), serde_json::Value::String(v.to_string()));
        }
    }
    Ok(Some(serde_json::Value::Object(m)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "version:4325\nrenderDistance:12\nsimulationDistance:12\nao:true\nresourcePacks:[\"vanilla\",\"file/x.zip\"]\nkey_key.attack:key.mouse.left\nsomeModKey:keep me\nrenderClouds:\"true\"\ngraphicsMode:1\n";

    #[test]
    fn roundtrip_preserves_unknown_lines() {
        let f = OptionsFile::parse(SAMPLE);
        assert_eq!(f.render(), SAMPLE);
        assert_eq!(f.get("resourcePacks"), Some("[\"vanilla\",\"file/x.zip\"]"));
    }

    #[test]
    fn apply_backs_up_and_changes_only_known_keys() {
        let d = tempfile::tempdir().unwrap();
        let game = d.path().join("game");
        let data = d.path().join("data");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("options.txt"), SAMPLE).unwrap();
        let g = crate::performance::presets::preset_graphics("low", None);
        let r = apply_graphics(&game, &data, &g, "1.21.11", None).unwrap();
        assert!(r.backup.is_some());
        let after = OptionsFile::load(&game.join("options.txt")).unwrap().unwrap();
        assert_eq!(after.get("renderDistance"), Some("6"));
        assert_eq!(after.get("someModKey"), Some("keep me"));
        assert_eq!(after.get("key_key.attack"), Some("key.mouse.left"));
        assert_eq!(after.get("renderClouds"), Some("\"false\""));
        assert_eq!(after.get("ao"), Some("false"));
        // original kept, restore works
        let backups = list_backups(&data);
        assert!(backups.iter().any(|b| b.original));
        restore(&game, &data, "options.original.txt").unwrap();
        assert_eq!(std::fs::read_to_string(game.join("options.txt")).unwrap(), SAMPLE);
        assert!(restore(&game, &data, "../../evil").is_err());
    }

    #[test]
    fn legacy_formats() {
        let old = OptionsFile::parse("ao:2\nfancyGraphics:true\nrenderClouds:true\n");
        let g = crate::performance::presets::preset_graphics("low", None);
        let ch = graphics_changes(&g, "1.8.9", Some(&old));
        let get = |k: &str| ch.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.clone());
        assert_eq!(get("ao").as_deref(), Some("0"));
        assert_eq!(get("fancyGraphics").as_deref(), Some("false"));
        assert_eq!(get("renderClouds").as_deref(), Some("false"));
        assert!(get("simulationDistance").is_none());
        assert!(get("graphicsMode").is_none());
    }

    #[test]
    fn creating_requires_data_version() {
        let d = tempfile::tempdir().unwrap();
        let g = crate::performance::presets::preset_graphics("balanced", None);
        assert!(apply_graphics(&d.path().join("g"), &d.path().join("p"), &g, "1.21.11", None).is_err());
        let r = apply_graphics(&d.path().join("g"), &d.path().join("p"), &g, "1.21.11", Some(4671)).unwrap();
        assert!(r.created);
        let f = OptionsFile::load(&d.path().join("g").join("options.txt")).unwrap().unwrap();
        assert_eq!(f.get("version"), Some("4671"));
    }
}
