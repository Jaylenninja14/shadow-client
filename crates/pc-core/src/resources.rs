//! Resource packs and shader packs: listing, activation, removal.
//!
//! * Resource packs are activated by editing the `resourcePacks` list in options.txt
//!   (after a backup), exactly as the in-game menu does.
//! * Shader packs are activated through Iris' `config/iris.properties` (`shaderPack`,
//!   `enableShaders`). Shaders are never enabled automatically.

use std::io::Read;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::minecraft::options::{self, OptionsFile};
use crate::{safepath, util, Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    pub file_name: String,
    pub is_folder: bool,
    pub description: Option<String>,
    pub pack_format: Option<u64>,
    pub icon: Option<String>,
    pub active: bool,
    pub size: u64,
}

fn read_zip_entry(path: &Path, name: &str, limit: u64) -> Option<Vec<u8>> {
    let f = std::fs::File::open(path).ok()?;
    let mut z = zip::ZipArchive::new(f).ok()?;
    let e = z.by_name(name).ok()?;
    let mut buf = vec![];
    e.take(limit).read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn mcmeta_description(v: &serde_json::Value) -> Option<String> {
    let d = &v["pack"]["description"];
    match d {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(a) => Some(a.iter().filter_map(|x| x.as_str().map(String::from).or_else(|| x["text"].as_str().map(String::from))).collect::<Vec<_>>().join("")),
        serde_json::Value::Object(_) => d["text"].as_str().map(String::from),
        _ => None,
    }
    .map(|s| strip_formatting(&s))
}

/// Remove Minecraft § formatting codes.
fn strip_formatting(s: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    for c in s.chars() {
        if skip {
            skip = false;
            continue;
        }
        if c == '§' {
            skip = true;
            continue;
        }
        out.push(c);
    }
    out
}

fn active_resource_packs(game_dir: &Path) -> Vec<String> {
    OptionsFile::load(&game_dir.join("options.txt"))
        .ok()
        .flatten()
        .and_then(|f| f.get("resourcePacks").map(String::from))
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .unwrap_or_default()
}

pub fn list_resource_packs(game_dir: &Path) -> Vec<PackInfo> {
    let active = active_resource_packs(game_dir);
    let dir = game_dir.join("resourcepacks");
    let mut out = vec![];
    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        let is_folder = path.is_dir();
        if !is_folder && !name.to_ascii_lowercase().ends_with(".zip") {
            continue;
        }
        let (meta, icon) = if is_folder {
            (std::fs::read(path.join("pack.mcmeta")).ok(), std::fs::read(path.join("pack.png")).ok())
        } else {
            (read_zip_entry(&path, "pack.mcmeta", 256 * 1024), read_zip_entry(&path, "pack.png", 256 * 1024))
        };
        let meta: Option<serde_json::Value> = meta.and_then(|b| serde_json::from_slice(&b).ok());
        out.push(PackInfo {
            active: active.iter().any(|a| a == &format!("file/{name}")),
            description: meta.as_ref().and_then(mcmeta_description),
            pack_format: meta.as_ref().and_then(|m| m["pack"]["pack_format"].as_u64()),
            icon: icon.filter(|b| b.starts_with(&[0x89, b'P'])).map(|b| format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(b))),
            size: if is_folder { util::dir_size(&path) } else { e.metadata().map(|m| m.len()).unwrap_or(0) },
            file_name: name,
            is_folder,
        });
    }
    out.sort_by_key(|p| p.file_name.to_lowercase());
    out
}

fn check_name(name: &str) -> Result<()> {
    let segs = safepath::validate_relative(name)?;
    if segs.len() != 1 {
        return Err(Error::UnsafePath(name.into()));
    }
    Ok(())
}

/// Activate/deactivate a resource pack. New packs are added on top (highest priority).
pub fn set_resource_pack_active(game_dir: &Path, profile_data: &Path, name: &str, active: bool) -> Result<()> {
    check_name(name)?;
    if !game_dir.join("resourcepacks").join(name).exists() {
        return Err(Error::NotFound(name.into()));
    }
    let path = game_dir.join("options.txt");
    let mut file = OptionsFile::load(&path)?.ok_or_else(|| Error::Invalid("Start this profile once so Minecraft creates options.txt, then activate packs.".into()))?;
    options::backup(game_dir, profile_data)?;
    let mut list: Vec<String> = file.get("resourcePacks").and_then(|v| serde_json::from_str(v).ok()).unwrap_or_else(|| vec!["vanilla".into()]);
    let id = format!("file/{name}");
    list.retain(|x| x != &id);
    if active {
        list.push(id.clone());
    }
    file.set("resourcePacks", &serde_json::to_string(&list)?);
    if let Some(inc) = file.get("incompatibleResourcePacks").map(String::from) {
        if let Ok(mut l) = serde_json::from_str::<Vec<String>>(&inc) {
            l.retain(|x| x != &id);
            file.set("incompatibleResourcePacks", &serde_json::to_string(&l)?);
        }
    }
    util::atomic_write(&path, file.render().as_bytes())
}

// ---------------------------------------------------------------- shaders

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShaderState {
    pub packs: Vec<PackInfo>,
    pub iris_config_found: bool,
    pub enabled: bool,
    pub active_pack: Option<String>,
}

fn iris_props(game_dir: &Path) -> PathBuf {
    game_dir.join("config").join("iris.properties")
}

fn read_props(path: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|l| match l.split_once('=') {
            Some((k, v)) if !l.trim_start().starts_with('#') => (k.trim().to_string(), v.trim().to_string()),
            _ => (String::new(), l.to_string()),
        })
        .collect()
}

fn write_props(path: &Path, props: &[(String, String)]) -> Result<()> {
    let mut s = String::new();
    for (k, v) in props {
        if k.is_empty() {
            s.push_str(v);
        } else {
            s.push_str(&format!("{k}={v}"));
        }
        s.push('\n');
    }
    util::atomic_write(path, s.as_bytes())
}

pub fn shader_state(game_dir: &Path) -> ShaderState {
    let props = read_props(&iris_props(game_dir));
    let get = |k: &str| props.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.clone());
    let active = get("shaderPack").filter(|s| !s.is_empty());
    let enabled = get("enableShaders").is_some_and(|v| v == "true");
    let mut packs = vec![];
    for e in std::fs::read_dir(game_dir.join("shaderpacks")).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let is_folder = e.path().is_dir();
        if !is_folder && !name.to_ascii_lowercase().ends_with(".zip") {
            continue;
        }
        packs.push(PackInfo {
            active: enabled && active.as_deref() == Some(name.as_str()),
            description: None,
            pack_format: None,
            icon: None,
            size: if is_folder { util::dir_size(&e.path()) } else { e.metadata().map(|m| m.len()).unwrap_or(0) },
            file_name: name,
            is_folder,
        });
    }
    packs.sort_by_key(|p| p.file_name.to_lowercase());
    ShaderState { packs, iris_config_found: iris_props(game_dir).exists(), enabled, active_pack: active }
}

/// Select a shader pack (or `None` to turn shaders off). Backs up iris.properties first.
pub fn set_shader(game_dir: &Path, profile_data: &Path, name: Option<&str>) -> Result<()> {
    let path = iris_props(game_dir);
    if path.exists() {
        let b = profile_data.join("backups").join("iris").join(format!("iris-{}.properties", util::timestamp_slug()));
        std::fs::create_dir_all(b.parent().unwrap())?;
        std::fs::copy(&path, &b)?;
    }
    let mut props = read_props(&path);
    let mut set = |k: &str, v: &str| match props.iter_mut().find(|(kk, _)| kk == k) {
        Some(p) => p.1 = v.to_string(),
        None => props.push((k.to_string(), v.to_string())),
    };
    match name {
        Some(n) => {
            check_name(n)?;
            if !game_dir.join("shaderpacks").join(n).exists() {
                return Err(Error::NotFound(n.into()));
            }
            set("shaderPack", n);
            set("enableShaders", "true");
        }
        None => set("enableShaders", "false"),
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    write_props(&path, &props)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn resource_pack_activation_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("g");
        std::fs::create_dir_all(g.join("resourcepacks")).unwrap();
        let zp = g.join("resourcepacks/Pack.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&zp).unwrap());
        z.start_file("pack.mcmeta", zip::write::SimpleFileOptions::default()).unwrap();
        let mcmeta = format!("{{\"pack\":{{\"pack_format\":34,\"description\":\"{}aGreen pack\"}}}}", char::from_u32(0xA7).unwrap());
        z.write_all(mcmeta.as_bytes()).unwrap();
        z.finish().unwrap();
        std::fs::write(g.join("options.txt"), "version:4000\nresourcePacks:[\"vanilla\"]\n").unwrap();
        set_resource_pack_active(&g, &d.path().join("p"), "Pack.zip", true).unwrap();
        let list = list_resource_packs(&g);
        assert!(list[0].active);
        assert_eq!(list[0].description.as_deref(), Some("Green pack"));
        assert_eq!(list[0].pack_format, Some(34));
        set_resource_pack_active(&g, &d.path().join("p"), "Pack.zip", false).unwrap();
        assert!(!list_resource_packs(&g)[0].active);
        assert!(set_resource_pack_active(&g, &d.path().join("p"), "../x.zip", true).is_err());
    }

    #[test]
    fn shader_toggle_preserves_other_props() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("g");
        std::fs::create_dir_all(g.join("shaderpacks")).unwrap();
        std::fs::write(g.join("shaderpacks/MakeUp.zip"), b"x").unwrap();
        std::fs::create_dir_all(g.join("config")).unwrap();
        std::fs::write(g.join("config/iris.properties"), "#comment\nmaxShadowRenderDistance=32\n").unwrap();
        set_shader(&g, &d.path().join("p"), Some("MakeUp.zip")).unwrap();
        let s = shader_state(&g);
        assert!(s.enabled && s.packs[0].active);
        let text = std::fs::read_to_string(g.join("config/iris.properties")).unwrap();
        assert!(text.contains("#comment") && text.contains("maxShadowRenderDistance=32"));
        set_shader(&g, &d.path().join("p"), None).unwrap();
        assert!(!shader_state(&g).enabled);
    }
}
