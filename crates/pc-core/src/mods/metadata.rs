//! Reading mod metadata from jars (fabric.mod.json, quilt.mod.json, mods.toml,
//! neoforge.mods.toml) and evaluating Fabric-style version predicates.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModMeta {
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    /// "fabric" | "quilt" | "forge" | "neoforge"
    pub loader: String,
    /// dependency id → list of alternative predicates (OR)
    pub depends: BTreeMap<String, Vec<String>>,
    pub breaks: BTreeMap<String, Vec<String>>,
    pub icon_path: Option<String>,
    pub environment: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    /// Ids provided by this mod (aliases, jar-in-jar is not inspected)
    pub provides: Vec<String>,
}

fn read_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str, limit: u64) -> Option<Vec<u8>> {
    let entry = zip.by_name(name).ok()?;
    let mut buf = Vec::new();
    entry.take(limit).read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn string_or_list(v: &serde_json::Value) -> Vec<String> {
    match v {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(a) => a.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
        _ => vec![],
    }
}

fn dep_map(v: Option<&serde_json::Value>) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    if let Some(serde_json::Value::Object(m)) = v {
        for (k, val) in m {
            let preds = string_or_list(val);
            out.insert(k.clone(), if preds.is_empty() { vec!["*".into()] } else { preds });
        }
    }
    out
}

fn parse_fabric(bytes: &[u8]) -> Option<ModMeta> {
    // Some mods ship fabric.mod.json with a UTF-8 BOM or trailing commas; tolerate the BOM.
    let text = String::from_utf8_lossy(bytes);
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let icon = match &v["icon"] {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(m) => {
            let mut sizes: Vec<(u32, String)> = m.iter().filter_map(|(k, v)| Some((k.parse().ok()?, v.as_str()?.to_string()))).collect();
            sizes.sort();
            sizes.iter().rev().find(|(s, _)| *s <= 128).or(sizes.first()).map(|(_, p)| p.clone())
        }
        _ => None,
    };
    let authors = v["authors"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from).or_else(|| x["name"].as_str().map(String::from))).collect())
        .unwrap_or_default();
    let license = match &v["license"] {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(a) => Some(a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", ")),
        _ => None,
    };
    Some(ModMeta {
        mod_id: v["id"].as_str()?.to_string(),
        name: v["name"].as_str().or(v["id"].as_str()).unwrap_or_default().to_string(),
        version: v["version"].as_str().unwrap_or("?").to_string(),
        description: v["description"].as_str().map(String::from),
        authors,
        loader: "fabric".into(),
        depends: dep_map(v.get("depends")),
        breaks: dep_map(v.get("breaks")),
        icon_path: icon,
        environment: v["environment"].as_str().map(String::from),
        license,
        homepage: v["contact"]["homepage"].as_str().map(String::from),
        provides: string_or_list(&v["provides"]),
    })
}

fn parse_quilt(bytes: &[u8]) -> Option<ModMeta> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let ql = &v["quilt_loader"];
    let meta = &ql["metadata"];
    let mut depends = BTreeMap::new();
    let parse_deps = |arr: &serde_json::Value, out: &mut BTreeMap<String, Vec<String>>| {
        if let Some(a) = arr.as_array() {
            for d in a {
                if let Some(id) = d.as_str() {
                    out.insert(id.to_string(), vec!["*".into()]);
                } else if let Some(id) = d["id"].as_str() {
                    if d["optional"].as_bool() == Some(true) {
                        continue;
                    }
                    let id = id.split(':').next_back().unwrap_or(id).to_string();
                    let preds = string_or_list(&d["versions"]);
                    out.insert(id, if preds.is_empty() { vec!["*".into()] } else { preds });
                }
            }
        }
    };
    parse_deps(&ql["depends"], &mut depends);
    let mut breaks = BTreeMap::new();
    parse_deps(&ql["breaks"], &mut breaks);
    Some(ModMeta {
        mod_id: ql["id"].as_str()?.to_string(),
        name: meta["name"].as_str().or(ql["id"].as_str()).unwrap_or_default().to_string(),
        version: ql["version"].as_str().unwrap_or("?").to_string(),
        description: meta["description"].as_str().map(String::from),
        authors: meta["contributors"].as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default(),
        loader: "quilt".into(),
        depends,
        breaks,
        icon_path: meta["icon"].as_str().map(String::from),
        environment: v["minecraft"]["environment"].as_str().map(String::from),
        license: meta["license"].as_str().map(String::from),
        homepage: meta["contact"]["homepage"].as_str().map(String::from),
        provides: vec![],
    })
}

fn parse_mods_toml(bytes: &[u8], loader: &str, manifest_version: Option<&str>) -> Option<ModMeta> {
    let text = String::from_utf8_lossy(bytes);
    let v: toml::Value = toml::from_str(&text).ok()?;
    let m = v.get("mods")?.as_array()?.first()?;
    let id = m.get("modId")?.as_str()?.to_string();
    let mut version = m.get("version").and_then(|x| x.as_str()).unwrap_or("?").to_string();
    if version.contains("${file.jarVersion}") {
        version = manifest_version.unwrap_or("?").to_string();
    }
    let mut depends = BTreeMap::new();
    let mut breaks = BTreeMap::new();
    if let Some(deps) = v.get("dependencies").and_then(|d| d.get(&id)).and_then(|d| d.as_array()) {
        for d in deps {
            let Some(dep_id) = d.get("modId").and_then(|x| x.as_str()) else { continue };
            let range = d.get("versionRange").and_then(|x| x.as_str()).unwrap_or("*").to_string();
            let kind = d.get("type").and_then(|x| x.as_str()).map(|s| s.to_ascii_lowercase());
            let mandatory = d.get("mandatory").and_then(|x| x.as_bool());
            match (kind.as_deref(), mandatory) {
                (Some("incompatible"), _) => {
                    breaks.insert(dep_id.to_string(), vec![range]);
                }
                (Some("required"), _) | (None, Some(true)) => {
                    depends.insert(dep_id.to_string(), vec![range]);
                }
                _ => {}
            }
        }
    }
    Some(ModMeta {
        mod_id: id.clone(),
        name: m.get("displayName").and_then(|x| x.as_str()).unwrap_or(&id).to_string(),
        version,
        description: m.get("description").and_then(|x| x.as_str()).map(|s| s.trim().to_string()),
        authors: m.get("authors").and_then(|x| x.as_str()).map(|s| vec![s.to_string()]).unwrap_or_default(),
        loader: loader.into(),
        depends,
        breaks,
        icon_path: m.get("logoFile").and_then(|x| x.as_str()).or(v.get("logoFile").and_then(|x| x.as_str())).map(String::from),
        environment: None,
        license: v.get("license").and_then(|x| x.as_str()).map(String::from),
        homepage: m.get("displayURL").and_then(|x| x.as_str()).map(String::from),
        provides: vec![],
    })
}

/// Read mod metadata from a jar. `Ok(None)` if the jar has no recognised metadata.
pub fn read_jar(path: &Path) -> Result<Option<ModMeta>> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    const LIMIT: u64 = 1024 * 1024;
    if let Some(b) = read_entry(&mut zip, "fabric.mod.json", LIMIT) {
        let mut meta = parse_fabric(&b);
        // Jar-in-jar: nested mods (e.g. Fabric API modules, bundled libraries) satisfy
        // dependencies too, so record their ids as "provided".
        if let Some(m) = meta.as_mut() {
            let nested: Vec<String> = serde_json::from_slice::<serde_json::Value>(&b)
                .ok()
                .and_then(|v| v["jars"].as_array().map(|a| a.iter().filter_map(|j| j["file"].as_str().map(String::from)).collect()))
                .unwrap_or_default();
            for inner in nested.iter().take(200) {
                if let Some(bytes) = read_entry(&mut zip, inner, 64 * 1024 * 1024) {
                    if let Ok(mut iz) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) {
                        if let Ok(e) = iz.by_name("fabric.mod.json") {
                            let mut buf = Vec::new();
                            if e.take(LIMIT).read_to_end(&mut buf).is_ok() {
                                if let Some(nm) = parse_fabric(&buf) {
                                    m.provides.push(nm.mod_id);
                                    m.provides.extend(nm.provides);
                                }
                            }
                        }
                    }
                }
            }
        }
        return Ok(meta);
    }
    if let Some(b) = read_entry(&mut zip, "quilt.mod.json", LIMIT) {
        return Ok(parse_quilt(&b));
    }
    let manifest_version = read_entry(&mut zip, "META-INF/MANIFEST.MF", LIMIT).and_then(|b| {
        String::from_utf8_lossy(&b).lines().find_map(|l| l.strip_prefix("Implementation-Version:").map(|v| v.trim().to_string()))
    });
    if let Some(b) = read_entry(&mut zip, "META-INF/neoforge.mods.toml", LIMIT) {
        return Ok(parse_mods_toml(&b, "neoforge", manifest_version.as_deref()));
    }
    if let Some(b) = read_entry(&mut zip, "META-INF/mods.toml", LIMIT) {
        return Ok(parse_mods_toml(&b, "forge", manifest_version.as_deref()));
    }
    Ok(None)
}

/// Read a PNG icon from the jar as a data URL (max 256 KB).
pub fn read_icon(path: &Path, icon_path: &str) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let name = icon_path.trim_start_matches('/');
    let bytes = read_entry(&mut zip, name, 256 * 1024)?;
    if !bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return None;
    }
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

// ---------------------------------------------------------------- version predicates

#[derive(Debug, Clone, PartialEq)]
struct Ver {
    nums: Vec<u64>,
    pre: Option<String>,
}

fn parse_ver(s: &str) -> Option<Ver> {
    let s = s.trim().trim_start_matches('v');
    let s = s.split('+').next()?; // drop build metadata
    let (core, pre) = match s.find('-') {
        Some(i) => (&s[..i], Some(s[i + 1..].to_string())),
        None => (s, None),
    };
    let nums: Option<Vec<u64>> = core.split('.').map(|p| p.parse().ok()).collect();
    let nums = nums?;
    if nums.is_empty() {
        return None;
    }
    Some(Ver { nums, pre })
}

fn cmp_ver(a: &Ver, b: &Ver) -> std::cmp::Ordering {
    let n = a.nums.len().max(b.nums.len());
    for i in 0..n {
        let x = a.nums.get(i).copied().unwrap_or(0);
        let y = b.nums.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    match (&a.pre, &b.pre) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(x), Some(y)) => x.cmp(y),
    }
}

/// Compare two version strings (numeric segments, pre-releases sort before releases).
pub fn compare_versions(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    Some(cmp_ver(&parse_ver(a)?, &parse_ver(b)?))
}

fn single(version: &Ver, pred: &str) -> Option<bool> {
    let p = pred.trim();
    if p.is_empty() || p == "*" {
        return Some(true);
    }
    // Wildcards: 1.21.x / 1.21.*
    if p.ends_with(".x") || p.ends_with(".X") || p.ends_with(".*") {
        let prefix = parse_ver(&p[..p.len() - 2])?;
        return Some(version.nums.len() >= prefix.nums.len() && version.nums[..prefix.nums.len()] == prefix.nums[..]);
    }
    let (op, rest) = ["<=", ">=", "<", ">", "=", "~", "^"].iter().find_map(|op| p.strip_prefix(op).map(|r| (*op, r))).unwrap_or(("", p));
    let target = parse_ver(rest)?;
    let ord = cmp_ver(version, &target);
    use std::cmp::Ordering::*;
    Some(match op {
        "" | "=" => ord == Equal || (target.pre.is_none() && version.pre.is_none() && ord == Equal),
        ">=" => ord != Less,
        "<=" => ord != Greater,
        ">" => ord == Greater,
        "<" => ord == Less,
        "~" => ord != Less && version.nums.first() == target.nums.first() && version.nums.get(1).copied().unwrap_or(0) == target.nums.get(1).copied().unwrap_or(0),
        "^" => ord != Less && version.nums.first() == target.nums.first(),
        _ => return None,
    })
}

/// Fabric predicate: space-separated terms are AND. Maven-style ranges (`[1.21,1.22)`) are
/// supported for Forge metadata. `None` means "could not evaluate".
pub fn matches_predicate(version: &str, predicate: &str) -> Option<bool> {
    let v = parse_ver(version)?;
    let p = predicate.trim();
    if (p.starts_with('[') || p.starts_with('(')) && (p.ends_with(']') || p.ends_with(')')) {
        let inner = &p[1..p.len() - 1];
        let (lo, hi) = inner.split_once(',').unwrap_or((inner, inner));
        let lo_ok = if lo.trim().is_empty() { true } else {
            let l = parse_ver(lo)?;
            let o = cmp_ver(&v, &l);
            if p.starts_with('[') { o != std::cmp::Ordering::Less } else { o == std::cmp::Ordering::Greater }
        };
        let hi_ok = if hi.trim().is_empty() { true } else {
            let h = parse_ver(hi)?;
            let o = cmp_ver(&v, &h);
            if p.ends_with(']') { o != std::cmp::Ordering::Greater } else { o == std::cmp::Ordering::Less }
        };
        return Some(lo_ok && hi_ok);
    }
    let mut all = true;
    for term in p.split_whitespace() {
        all &= single(&v, term)?;
    }
    Some(all)
}

/// Any of the alternatives matches. `None` if none could be evaluated.
pub fn matches_any(version: &str, predicates: &[String]) -> Option<bool> {
    let mut evaluated = false;
    for p in predicates {
        match matches_predicate(version, p) {
            Some(true) => return Some(true),
            Some(false) => evaluated = true,
            None => {}
        }
    }
    if evaluated { Some(false) } else { None }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    pub fn make_jar(path: &Path, fabric_json: &str) {
        let f = std::fs::File::create(path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file("fabric.mod.json", opts).unwrap();
        z.write_all(fabric_json.as_bytes()).unwrap();
        z.start_file("assets/x/icon.png", opts).unwrap();
        z.write_all(&[0x89, b'P', b'N', b'G', 1, 2, 3]).unwrap();
        z.finish().unwrap();
    }

    #[test]
    fn reads_fabric_jar() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("m.jar");
        make_jar(&p, r#"{"schemaVersion":1,"id":"marlowcrystal","version":"2.0.0","name":"Marlow","icon":"assets/x/icon.png",
            "authors":["Bram",{"name":"Marlow"}],"license":"MIT","depends":{"minecraft":"1.21.11","fabric-api":"*"},"breaks":{"optifabric":"*"}}"#);
        let m = read_jar(&p).unwrap().unwrap();
        assert_eq!(m.mod_id, "marlowcrystal");
        assert_eq!(m.authors, vec!["Bram", "Marlow"]);
        assert_eq!(m.depends["minecraft"], vec!["1.21.11"]);
        assert!(m.breaks.contains_key("optifabric"));
        assert!(read_icon(&p, m.icon_path.as_deref().unwrap()).unwrap().starts_with("data:image/png;base64,"));
    }

    #[test]
    fn reads_forge_toml() {
        let toml = r#"
modLoader="javafml"
license="MIT"
[[mods]]
modId="examplemod"
version="${file.jarVersion}"
displayName="Example"
[[dependencies.examplemod]]
modId="minecraft"
mandatory=true
versionRange="[1.20.1,1.21)"
[[dependencies.examplemod]]
modId="rubidium"
type="incompatible"
versionRange="*"
"#;
        let m = parse_mods_toml(toml.as_bytes(), "forge", Some("1.2.3")).unwrap();
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.depends["minecraft"], vec!["[1.20.1,1.21)"]);
        assert!(m.breaks.contains_key("rubidium"));
    }

    #[test]
    fn predicates() {
        assert_eq!(matches_predicate("1.21.11", "1.21.11"), Some(true));
        assert_eq!(matches_predicate("1.21.10", "1.21.11"), Some(false));
        assert_eq!(matches_predicate("1.21.11", ">=1.21.5"), Some(true));
        assert_eq!(matches_predicate("1.21.4", ">=1.21.5"), Some(false));
        assert_eq!(matches_predicate("1.21.4", "~1.21.2"), Some(true));
        assert_eq!(matches_predicate("1.22", "~1.21.2"), Some(false));
        assert_eq!(matches_predicate("1.21.11", "1.21.x"), Some(true));
        assert_eq!(matches_predicate("26.2", ">=1.21.5 <26.3"), Some(true));
        assert_eq!(matches_predicate("1.20.4", "[1.20.1,1.21)"), Some(true));
        assert_eq!(matches_predicate("1.21", "[1.20.1,1.21)"), Some(false));
        assert_eq!(matches_any("1.21.9", &["1.21.5".into(), "1.21.9".into()]), Some(true));
        assert_eq!(matches_any("26.2", &["1.21.5".into(), "1.21.11".into()]), Some(false));
        assert_eq!(matches_any("26.3-snapshot-5", &["garbage-pred!".into()]), None);
        assert_eq!(compare_versions("26.2", "1.21.11"), Some(std::cmp::Ordering::Greater));
        assert_eq!(compare_versions("1.21.11-pre1", "1.21.11"), Some(std::cmp::Ordering::Less));
    }
}
