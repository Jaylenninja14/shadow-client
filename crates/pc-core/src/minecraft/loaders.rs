//! Fabric and Quilt metadata (official meta services).

use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

use super::manifest::VersionJson;
use super::Endpoints;
use crate::http::Http;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    Neoforge,
}

impl LoaderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoaderKind::Vanilla => "vanilla",
            LoaderKind::Fabric => "fabric",
            LoaderKind::Quilt => "quilt",
            LoaderKind::Forge => "forge",
            LoaderKind::Neoforge => "neoforge",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "vanilla" | "minecraft" => LoaderKind::Vanilla,
            "fabric" | "fabric-loader" => LoaderKind::Fabric,
            "quilt" | "quilt-loader" => LoaderKind::Quilt,
            "forge" => LoaderKind::Forge,
            "neoforge" => LoaderKind::Neoforge,
            _ => return None,
        })
    }
    /// Whether this launcher can install the loader itself.
    pub fn installable(&self) -> bool {
        matches!(self, LoaderKind::Vanilla | LoaderKind::Fabric | LoaderKind::Quilt)
    }
    /// Mod loaders whose mods run on this loader (Quilt runs most Fabric mods).
    pub fn accepted_mod_loaders(&self) -> Vec<&'static str> {
        match self {
            LoaderKind::Vanilla => vec![],
            LoaderKind::Fabric => vec!["fabric"],
            LoaderKind::Quilt => vec!["quilt", "fabric"],
            LoaderKind::Forge => vec!["forge"],
            LoaderKind::Neoforge => vec!["neoforge"],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVersionSupport {
    pub version: String,
    pub stable: bool,
}

#[derive(Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}
#[derive(Deserialize)]
struct FabricLoaderInfo {
    version: String,
    #[serde(default)]
    stable: Option<bool>,
}

fn base(ep: &Endpoints, kind: LoaderKind) -> Result<String> {
    match kind {
        LoaderKind::Fabric => Ok(format!("{}/v2", ep.fabric_meta)),
        LoaderKind::Quilt => Ok(format!("{}/v3", ep.quilt_meta)),
        other => Err(Error::Incompatible(format!(
            "{} cannot be installed by Shadow Client. Use the official {} installer, then manage the resulting version as an existing installation.",
            other.as_str(),
            other.as_str()
        ))),
    }
}

/// Minecraft versions supported by the loader.
pub async fn supported_game_versions(http: &Http, ep: &Endpoints, kind: LoaderKind) -> Result<Vec<GameVersionSupport>> {
    let url = format!("{}/versions/game", base(ep, kind)?);
    let list: Vec<serde_json::Value> = http.get_json(&url, Some(Duration::from_secs(3600)), HeaderMap::new()).await?;
    Ok(list
        .into_iter()
        .filter_map(|v| {
            Some(GameVersionSupport {
                version: v.get("version")?.as_str()?.to_string(),
                stable: v.get("stable").and_then(|s| s.as_bool()).unwrap_or(true),
            })
        })
        .collect())
}

/// Loader versions available for a Minecraft version, newest first.
pub async fn loader_versions(http: &Http, ep: &Endpoints, kind: LoaderKind, mc: &str) -> Result<Vec<LoaderVersion>> {
    let url = format!("{}/versions/loader/{}", base(ep, kind)?, urlencode(mc));
    let list: Vec<FabricLoaderEntry> = http.get_json(&url, Some(Duration::from_secs(1800)), HeaderMap::new()).await?;
    if list.is_empty() {
        return Err(Error::Incompatible(format!("{} does not support Minecraft {mc} yet", kind.as_str())));
    }
    Ok(list
        .into_iter()
        .map(|e| LoaderVersion {
            stable: e.loader.stable.unwrap_or(!e.loader.version.contains("beta")),
            version: e.loader.version,
        })
        .collect())
}

/// Latest stable loader for `mc` (falls back to the newest one if none is marked stable).
pub async fn recommended_loader(http: &Http, ep: &Endpoints, kind: LoaderKind, mc: &str) -> Result<String> {
    let list = loader_versions(http, ep, kind, mc).await?;
    Ok(list.iter().find(|l| l.stable).unwrap_or(&list[0]).version.clone())
}

/// Launcher profile JSON (inherits from the vanilla version).
pub async fn profile_json(http: &Http, ep: &Endpoints, kind: LoaderKind, mc: &str, loader: &str) -> Result<VersionJson> {
    let url = format!("{}/versions/loader/{}/{}/profile/json", base(ep, kind)?, urlencode(mc), urlencode(loader));
    let v: VersionJson = http.get_json(&url, Some(Duration::from_secs(86400)), HeaderMap::new()).await?;
    if v.inherits_from.as_deref() != Some(mc) {
        return Err(Error::Invalid(format!("{} profile for {mc} inherits from {:?}", kind.as_str(), v.inherits_from)));
    }
    Ok(v)
}

pub fn version_id(kind: LoaderKind, mc: &str, loader: &str) -> String {
    match kind {
        LoaderKind::Fabric => format!("fabric-loader-{loader}-{mc}"),
        LoaderKind::Quilt => format!("quilt-loader-{loader}-{mc}"),
        _ => mc.to_string(),
    }
}

fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
