//! CurseForge API client (https://docs.curseforge.com/).
//!
//! Requires an API key supplied by the user (Settings → Accounts & services). The key is stored
//! with DPAPI, sent only in the `x-api-key` header to api.curseforge.com, and never logged.
//! Files whose `downloadUrl` is null are not distributable through the API: the launcher shows
//! the official project page instead of trying to work around the restriction.

use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::context::Ctx;
use crate::secrets::CURSEFORGE_KEY;
use crate::{Error, Result};

pub const MINECRAFT_GAME_ID: u32 = 432;
pub const CLASS_MODS: u32 = 6;
pub const CLASS_MODPACKS: u32 = 4471;
pub const CLASS_RESOURCEPACKS: u32 = 12;
pub const CLASS_SHADERS: u32 = 6552;

pub fn loader_type(loader: &str) -> Option<u32> {
    Some(match loader {
        "forge" => 1,
        "fabric" => 4,
        "quilt" => 5,
        "neoforge" => 6,
        _ => return None,
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfSearchQuery {
    /// CurseForge game; Minecraft Java when unset (Bedrock: crate::bedrock::CURSEFORGE_GAME_ID)
    #[serde(default)]
    pub game_id: Option<u32>,
    pub query: String,
    pub class_id: u32,
    pub game_version: Option<String>,
    pub loader: Option<String>,
    pub category_id: Option<u32>,
    /// "relevance" | "popularity" | "downloads" | "updated" | "name"
    pub sort: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfLinks {
    pub website_url: Option<String>,
    pub source_url: Option<String>,
    pub issues_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfLogo {
    pub thumbnail_url: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfAuthor {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfCategory {
    pub id: u32,
    pub name: String,
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfFileIndex {
    pub game_version: String,
    pub file_id: u64,
    pub filename: String,
    pub release_type: u32,
    pub mod_loader: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfMod {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: Option<f64>,
    pub links: Option<CfLinks>,
    pub logo: Option<CfLogo>,
    #[serde(default)]
    pub authors: Vec<CfAuthor>,
    #[serde(default)]
    pub categories: Vec<CfCategory>,
    pub date_modified: Option<String>,
    pub date_created: Option<String>,
    #[serde(default)]
    pub latest_files_indexes: Vec<CfFileIndex>,
    pub allow_mod_distribution: Option<bool>,
    pub class_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfHash {
    pub value: String,
    /// 1 = sha1, 2 = md5
    pub algo: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfDependency {
    pub mod_id: u64,
    /// 1 embedded, 2 optional, 3 required, 4 tool, 5 incompatible, 6 include
    pub relation_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfFile {
    pub id: u64,
    pub mod_id: u64,
    pub display_name: String,
    pub file_name: String,
    /// 1 release, 2 beta, 3 alpha
    pub release_type: u32,
    pub file_date: String,
    pub file_length: Option<u64>,
    pub download_count: Option<f64>,
    pub download_url: Option<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub hashes: Vec<CfHash>,
    #[serde(default)]
    pub dependencies: Vec<CfDependency>,
    pub is_available: Option<bool>,
}

impl CfFile {
    pub fn sha1(&self) -> Option<String> {
        self.hashes.iter().find(|h| h.algo == 1).map(|h| h.value.to_ascii_lowercase())
    }
    /// CurseForge lists loaders in `gameVersions` ("Fabric", "Forge", "NeoForge", "Quilt").
    pub fn supports_loader(&self, loader: &str) -> bool {
        self.game_versions.iter().any(|g| g.eq_ignore_ascii_case(loader))
    }
    pub fn supports_mc(&self, mc: &str) -> bool {
        self.game_versions.iter().any(|g| g == mc)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfPagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfSearchResult {
    pub data: Vec<CfMod>,
    pub pagination: CfPagination,
}

#[derive(Deserialize)]
struct Data<T> {
    data: T,
}

fn key(ctx: &Ctx) -> Result<String> {
    let from_env = env_key();
    match from_env.or(ctx.secrets.get(CURSEFORGE_KEY)?) {
        Some(k) => Ok(k),
        None => Err(Error::MissingConfig(
            "CurseForge needs an API key. Get one at console.curseforge.com and add it under Settings → Accounts & services (see docs/CURSEFORGE_SETUP.md).".into(),
        )),
    }
}

pub fn has_key(ctx: &Ctx) -> bool {
    env_key().is_some() || ctx.secrets.has(CURSEFORGE_KEY)
}

fn auth_headers(ctx: &Ctx) -> Result<HeaderMap> {
    let k = key(ctx)?;
    let mut h = HeaderMap::new();
    h.insert("x-api-key", k.parse().map_err(|_| Error::Invalid("The CurseForge API key contains invalid characters".into()))?);
    h.insert("accept", "application/json".parse().unwrap());
    Ok(h)
}

fn map_auth(e: Error) -> Error {
    match e {
        Error::Auth(_) => Error::Auth("CurseForge rejected the API key. Check it under Settings → Accounts & services.".into()),
        other => other,
    }
}

pub async fn search(ctx: &Ctx, q: &CfSearchQuery) -> Result<CfSearchResult> {
    let h = auth_headers(ctx)?;
    let mut url = url::Url::parse(&format!("{}/v1/mods/search", ctx.api.curseforge)).map_err(|e| Error::msg(e.to_string()))?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("gameId", &q.game_id.unwrap_or(MINECRAFT_GAME_ID).to_string())
            .append_pair("classId", &q.class_id.max(1).to_string())
            .append_pair("index", &q.offset.to_string())
            .append_pair("pageSize", &q.limit.clamp(1, 50).to_string())
            .append_pair("sortOrder", "desc");
        if !q.query.trim().is_empty() {
            qp.append_pair("searchFilter", q.query.trim());
        }
        if let Some(v) = q.game_version.as_ref().filter(|v| !v.is_empty()) {
            qp.append_pair("gameVersion", v);
        }
        if let Some(t) = q.loader.as_deref().and_then(loader_type) {
            qp.append_pair("modLoaderType", &t.to_string());
        }
        if let Some(c) = q.category_id {
            qp.append_pair("categoryId", &c.to_string());
        }
        let sort = match q.sort.as_deref() {
            Some("popularity") => 2,
            Some("updated") => 3,
            Some("name") => 4,
            Some("downloads") => 6,
            _ => 1, // featured/relevance
        };
        qp.append_pair("sortField", &sort.to_string());
    }
    ctx.http.get_json(url.as_str(), Some(Duration::from_secs(300)), h).await.map_err(map_auth)
}

/// The project classes (e.g. Addons, Texture Packs) of a CurseForge game.
pub async fn classes(ctx: &Ctx, game_id: u32) -> Result<Vec<CfCategory>> {
    let h = auth_headers(ctx)?;
    let url = format!("{}/v1/categories?gameId={game_id}&classesOnly=true", ctx.api.curseforge);
    let d: Data<Vec<CfCategory>> = ctx.http.get_json(&url, Some(Duration::from_secs(3600)), h).await.map_err(map_auth)?;
    Ok(d.data)
}

pub async fn get_mod(ctx: &Ctx, mod_id: u64) -> Result<CfMod> {
    let h = auth_headers(ctx)?;
    let d: Data<CfMod> = ctx.http.get_json(&format!("{}/v1/mods/{mod_id}", ctx.api.curseforge), Some(Duration::from_secs(3600)), h).await.map_err(map_auth)?;
    Ok(d.data)
}

pub async fn get_mods(ctx: &Ctx, ids: &[u64]) -> Result<Vec<CfMod>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let h = auth_headers(ctx)?;
    let d: Data<Vec<CfMod>> = ctx.http.post_json(&format!("{}/v1/mods", ctx.api.curseforge), &json!({"modIds": ids}), Some(Duration::from_secs(3600)), h).await.map_err(map_auth)?;
    Ok(d.data)
}

pub async fn mod_files(ctx: &Ctx, mod_id: u64, mc: Option<&str>, loader: Option<&str>) -> Result<Vec<CfFile>> {
    let h = auth_headers(ctx)?;
    let mut url = url::Url::parse(&format!("{}/v1/mods/{mod_id}/files", ctx.api.curseforge)).map_err(|e| Error::msg(e.to_string()))?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("pageSize", "50");
        if let Some(v) = mc {
            qp.append_pair("gameVersion", v);
        }
        if let Some(t) = loader.and_then(loader_type) {
            qp.append_pair("modLoaderType", &t.to_string());
        }
    }
    let d: Data<Vec<CfFile>> = ctx.http.get_json(url.as_str(), Some(Duration::from_secs(900)), h).await.map_err(map_auth)?;
    Ok(d.data)
}

pub async fn get_files(ctx: &Ctx, file_ids: &[u64]) -> Result<Vec<CfFile>> {
    if file_ids.is_empty() {
        return Ok(vec![]);
    }
    let h = auth_headers(ctx)?;
    let d: Data<Vec<CfFile>> =
        ctx.http.post_json(&format!("{}/v1/mods/files", ctx.api.curseforge), &json!({"fileIds": file_ids}), Some(Duration::from_secs(3600)), h).await.map_err(map_auth)?;
    Ok(d.data)
}

/// Best file for a Minecraft version + loader: release > beta > alpha, newest first.
pub fn pick_best(files: &[CfFile], mc: &str, loader: Option<&str>) -> Option<CfFile> {
    let mut ok: Vec<&CfFile> = files
        .iter()
        .filter(|f| f.is_available.unwrap_or(true))
        .filter(|f| f.supports_mc(mc))
        .filter(|f| loader.is_none_or(|l| f.supports_loader(l)))
        .collect();
    ok.sort_by(|a, b| a.release_type.cmp(&b.release_type).then(b.file_date.cmp(&a.file_date)));
    ok.first().map(|f| (*f).clone())
}

pub fn project_url(m: &CfMod) -> String {
    m.links
        .as_ref()
        .and_then(|l| l.website_url.clone())
        .unwrap_or_else(|| format!("https://www.curseforge.com/minecraft/mc-mods/{}", m.slug))
}

/// The API key from the environment (`SHADOW_CURSEFORGE_API_KEY`, or `SUNNY_`, the old name).
fn env_key() -> Option<String> {
    ["SHADOW_CURSEFORGE_API_KEY", "SUNNY_CURSEFORGE_API_KEY"].iter().find_map(|v| std::env::var(v).ok().filter(|k| !k.trim().is_empty()))
}
