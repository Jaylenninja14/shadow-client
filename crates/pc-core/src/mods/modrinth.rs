//! Modrinth API v2 client (https://docs.modrinth.com/). Uses the shared HTTP client, which
//! sends an identifying User-Agent, caches responses and backs off on HTTP 429.

use std::collections::HashMap;
use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::context::Ctx;
use crate::{Error, Result};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub query: String,
    /// "mod" | "modpack" | "resourcepack" | "shader"
    pub project_type: String,
    pub game_version: Option<String>,
    /// loader category, e.g. "fabric"
    pub loader: Option<String>,
    pub categories: Vec<String>,
    /// "relevance" | "downloads" | "follows" | "newest" | "updated"
    pub sort: Option<String>,
    pub offset: u32,
    pub limit: u32,
    /// Only client-usable projects
    pub client_side_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: Option<String>,
    pub title: String,
    pub description: String,
    pub author: Option<String>,
    pub icon_url: Option<String>,
    pub downloads: Option<u64>,
    pub follows: Option<u64>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub display_categories: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
    pub latest_version: Option<String>,
    pub date_modified: Option<String>,
    pub project_type: Option<String>,
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub id: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: Option<String>,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    pub project_type: String,
    pub downloads: Option<u64>,
    pub followers: Option<u64>,
    pub icon_url: Option<String>,
    pub license: Option<License>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    pub source_url: Option<String>,
    pub issues_url: Option<String>,
    pub wiki_url: Option<String>,
    pub updated: Option<String>,
    pub published: Option<String>,
    pub team: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hashes {
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub hashes: Hashes,
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    /// "required" | "optional" | "incompatible" | "embedded"
    pub dependency_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub name: Option<String>,
    pub version_number: String,
    /// "release" | "beta" | "alpha"
    pub version_type: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    pub date_published: String,
    pub downloads: Option<u64>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub files: Vec<VersionFile>,
}

impl Version {
    pub fn primary_file(&self) -> Option<&VersionFile> {
        self.files.iter().find(|f| f.primary).or_else(|| self.files.first())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub role: Option<String>,
    pub user: TeamUser,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamUser {
    pub username: String,
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(Error::Invalid(format!("invalid Modrinth id {id:?}")));
    }
    Ok(())
}

fn api(ctx: &Ctx, path: &str) -> String {
    format!("{}{}", ctx.api.modrinth, path)
}

pub async fn search(ctx: &Ctx, q: &SearchQuery) -> Result<SearchResult> {
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{}", if q.project_type.is_empty() { "mod" } else { &q.project_type })]];
    if let Some(v) = q.game_version.as_ref().filter(|v| !v.is_empty()) {
        facets.push(vec![format!("versions:{v}")]);
    }
    if let Some(l) = q.loader.as_ref().filter(|v| !v.is_empty()) {
        if l == "quilt" {
            facets.push(vec!["categories:quilt".into(), "categories:fabric".into()]);
        } else {
            facets.push(vec![format!("categories:{l}")]);
        }
    }
    for c in &q.categories {
        facets.push(vec![format!("categories:{c}")]);
    }
    if q.client_side_only {
        facets.push(vec!["client_side:required".into(), "client_side:optional".into()]);
    }
    let index = match q.sort.as_deref() {
        Some("downloads") => "downloads",
        Some("follows") => "follows",
        Some("newest") => "newest",
        Some("updated") => "updated",
        _ => "relevance",
    };
    let limit = q.limit.clamp(1, 100);
    let facets = serde_json::to_string(&facets)?;
    let mut url = url::Url::parse(&api(ctx, "/search")).map_err(|e| Error::msg(e.to_string()))?;
    url.query_pairs_mut()
        .append_pair("query", q.query.trim())
        .append_pair("facets", &facets)
        .append_pair("index", index)
        .append_pair("offset", &q.offset.to_string())
        .append_pair("limit", &limit.to_string());
    ctx.http.get_json(url.as_str(), Some(Duration::from_secs(300)), HeaderMap::new()).await
}

pub async fn project(ctx: &Ctx, id_or_slug: &str) -> Result<Project> {
    validate_id(id_or_slug)?;
    ctx.http.get_json(&api(ctx, &format!("/project/{id_or_slug}")), Some(Duration::from_secs(3600)), HeaderMap::new()).await
}

pub async fn projects(ctx: &Ctx, ids: &[String]) -> Result<Vec<Project>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    for id in ids {
        validate_id(id)?;
    }
    let mut url = url::Url::parse(&api(ctx, "/projects")).map_err(|e| Error::msg(e.to_string()))?;
    url.query_pairs_mut().append_pair("ids", &serde_json::to_string(ids)?);
    ctx.http.get_json(url.as_str(), Some(Duration::from_secs(3600)), HeaderMap::new()).await
}

pub async fn members(ctx: &Ctx, project_id: &str) -> Result<Vec<TeamMember>> {
    validate_id(project_id)?;
    ctx.http.get_json(&api(ctx, &format!("/project/{project_id}/members")), Some(Duration::from_secs(3600)), HeaderMap::new()).await
}

/// Versions of a project, optionally filtered by loaders and game versions (newest first).
pub async fn project_versions(ctx: &Ctx, project_id: &str, loaders: &[&str], game_versions: &[&str]) -> Result<Vec<Version>> {
    validate_id(project_id)?;
    let mut url = url::Url::parse(&api(ctx, &format!("/project/{project_id}/version"))).map_err(|e| Error::msg(e.to_string()))?;
    if !loaders.is_empty() {
        url.query_pairs_mut().append_pair("loaders", &serde_json::to_string(loaders)?);
    }
    if !game_versions.is_empty() {
        url.query_pairs_mut().append_pair("game_versions", &serde_json::to_string(game_versions)?);
    }
    ctx.http.get_json(url.as_str(), Some(Duration::from_secs(900)), HeaderMap::new()).await
}

pub async fn version(ctx: &Ctx, version_id: &str) -> Result<Version> {
    validate_id(version_id)?;
    ctx.http.get_json(&api(ctx, &format!("/version/{version_id}")), Some(Duration::from_secs(3600)), HeaderMap::new()).await
}

/// Identify files by hash. Returns hash → version for the files Modrinth knows.
pub async fn versions_by_hashes(ctx: &Ctx, sha1s: &[String]) -> Result<HashMap<String, Version>> {
    if sha1s.is_empty() {
        return Ok(HashMap::new());
    }
    ctx.http
        .post_json(&api(ctx, "/version_files"), &json!({"hashes": sha1s, "algorithm": "sha1"}), Some(Duration::from_secs(3600)), HeaderMap::new())
        .await
}

/// Latest compatible version for each file hash (update check).
pub async fn updates_by_hashes(ctx: &Ctx, sha1s: &[String], loaders: &[&str], game_versions: &[&str]) -> Result<HashMap<String, Version>> {
    if sha1s.is_empty() {
        return Ok(HashMap::new());
    }
    ctx.http
        .post_json(
            &api(ctx, "/version_files/update"),
            &json!({"hashes": sha1s, "algorithm": "sha1", "loaders": loaders, "game_versions": game_versions}),
            Some(Duration::from_secs(600)),
            HeaderMap::new(),
        )
        .await
}

/// Choose the best version: prefer release over beta over alpha, newest first.
pub fn pick_best(versions: &[Version], mc: &str, loaders: &[&str]) -> Option<Version> {
    let rank = |t: &str| match t {
        "release" => 0,
        "beta" => 1,
        _ => 2,
    };
    let mut ok: Vec<&Version> = versions
        .iter()
        .filter(|v| v.game_versions.iter().any(|g| g == mc))
        .filter(|v| loaders.is_empty() || v.loaders.iter().any(|l| loaders.contains(&l.as_str())))
        .filter(|v| v.primary_file().is_some())
        .collect();
    // Prefer an exact loader match (fabric for Fabric) over a compatible one (fabric on Quilt).
    ok.sort_by(|a, b| {
        let la = loaders.first().map(|l| !a.loaders.iter().any(|x| x == l)).unwrap_or(false);
        let lb = loaders.first().map(|l| !b.loaders.iter().any(|x| x == l)).unwrap_or(false);
        rank(&a.version_type).cmp(&rank(&b.version_type)).then(la.cmp(&lb)).then(b.date_published.cmp(&a.date_published))
    });
    ok.first().map(|v| (*v).clone())
}

pub fn project_url(project_type: &str, slug_or_id: &str) -> String {
    format!("https://modrinth.com/{}/{}", if project_type.is_empty() { "mod" } else { project_type }, slug_or_id)
}
