//! Shared HTTP client: identifying User-Agent, TTL disk cache for API responses, polite
//! handling of rate limits (HTTP 429) and an offline fallback to stale cache entries.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::{util, Error, Result};

pub struct Http {
    pub client: reqwest::Client,
    cache_dir: PathBuf,
    user_agent: RwLock<String>,
    default_ttl_secs: AtomicU64,
    api_permits: Semaphore,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    url: String,
    fetched_at: u64,
    body: String,
}

pub fn build_user_agent(contact: Option<&str>) -> String {
    match contact.map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => format!("ShadowClient/{} (Windows desktop launcher; {c})", crate::VERSION),
        None => format!("ShadowClient/{} (Windows desktop launcher)", crate::VERSION),
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Http {
    pub fn new(cache_dir: PathBuf, user_agent: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(60))
            .https_only(false) // enforced per request by `ensure_url_allowed` (loopback http for tests)
            .gzip(true)
            .build()?;
        Ok(Self {
            client,
            cache_dir,
            user_agent: RwLock::new(user_agent),
            default_ttl_secs: AtomicU64::new(30 * 60),
            api_permits: Semaphore::new(6),
        })
    }

    pub fn set_user_agent(&self, ua: String) {
        *self.user_agent.write().unwrap() = ua;
    }

    pub fn user_agent(&self) -> String {
        self.user_agent.read().unwrap().clone()
    }

    pub fn set_default_ttl(&self, ttl: Duration) {
        self.default_ttl_secs.store(ttl.as_secs().max(1), Ordering::Relaxed);
    }

    pub fn default_ttl(&self) -> Duration {
        Duration::from_secs(self.default_ttl_secs.load(Ordering::Relaxed))
    }

    fn cache_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", util::sha1_hex(key.as_bytes())))
    }

    fn read_cache(&self, key: &str) -> Option<CacheEntry> {
        let bytes = std::fs::read(self.cache_path(key)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    fn write_cache(&self, key: &str, url: &str, body: &str) {
        let entry = CacheEntry { url: url.into(), fetched_at: now_secs(), body: body.into() };
        if let Ok(bytes) = serde_json::to_vec(&entry) {
            let _ = std::fs::create_dir_all(&self.cache_dir);
            let _ = util::atomic_write(&self.cache_path(key), &bytes);
        }
    }

    pub fn ensure_url_allowed(url: &str) -> Result<()> {
        if crate::safepath::is_allowed_download_url(url) {
            Ok(())
        } else {
            Err(Error::Http(format!("Refusing non-HTTPS URL: {url}")))
        }
    }

    async fn send_text(&self, method: reqwest::Method, url: &str, body: Option<&serde_json::Value>, headers: &HeaderMap) -> Result<String> {
        Self::ensure_url_allowed(url)?;
        let _permit = self.api_permits.acquire().await.map_err(|_| Error::Cancelled)?;
        let mut attempt = 0;
        loop {
            attempt += 1;
            let mut req = self
                .client
                .request(method.clone(), url)
                .header(USER_AGENT, HeaderValue::from_str(&self.user_agent()).unwrap_or(HeaderValue::from_static("ShadowClient")))
                .headers(headers.clone())
                .timeout(Duration::from_secs(30));
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) if attempt < 3 && (e.is_timeout() || e.is_connect()) => {
                    tokio::time::sleep(Duration::from_millis(400 * attempt)).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            let status = resp.status();
            if status.as_u16() == 429 && attempt < 3 {
                let wait = resp
                    .headers()
                    .get("retry-after")
                    .or_else(|| resp.headers().get("x-ratelimit-reset"))
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.trim().parse::<u64>().ok())
                    .unwrap_or(5)
                    .clamp(1, 30);
                tracing::warn!("rate limited by {}, waiting {wait}s", host_of(url));
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            if status.is_server_error() && attempt < 3 {
                tokio::time::sleep(Duration::from_millis(700 * attempt)).await;
                continue;
            }
            let text = resp.text().await?;
            if !status.is_success() {
                let snippet: String = text.chars().take(300).collect();
                return Err(match status.as_u16() {
                    404 => Error::NotFound(format!("{} ({})", url, snippet.trim())),
                    401 | 403 => Error::Auth(format!("{} returned {}: {}", host_of(url), status, snippet.trim())),
                    _ => Error::Http(format!("{} returned {}: {}", host_of(url), status, snippet.trim())),
                });
            }
            return Ok(text);
        }
    }

    /// GET JSON. `ttl = Some(..)` enables the disk cache; stale entries are used when offline.
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str, ttl: Option<Duration>, headers: HeaderMap) -> Result<T> {
        let text = self.get_text_cached(url, ttl, headers).await?;
        Ok(serde_json::from_str(&text)?)
    }

    pub async fn get_text_cached(&self, url: &str, ttl: Option<Duration>, headers: HeaderMap) -> Result<String> {
        let key = format!("GET {url}");
        if let Some(ttl) = ttl {
            if let Some(entry) = self.read_cache(&key) {
                if now_secs().saturating_sub(entry.fetched_at) < ttl.as_secs() {
                    return Ok(entry.body);
                }
            }
        }
        match self.send_text(reqwest::Method::GET, url, None, &headers).await {
            Ok(text) => {
                if ttl.is_some() {
                    self.write_cache(&key, url, &text);
                }
                Ok(text)
            }
            Err(e @ Error::Http(_)) if ttl.is_some() => {
                if let Some(entry) = self.read_cache(&key) {
                    tracing::warn!("network error for {}, using cached copy: {e}", host_of(url));
                    return Ok(entry.body);
                }
                Err(e)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn post_json<T: DeserializeOwned>(&self, url: &str, body: &serde_json::Value, ttl: Option<Duration>, headers: HeaderMap) -> Result<T> {
        let key = format!("POST {url} {body}");
        if let Some(ttl) = ttl {
            if let Some(entry) = self.read_cache(&key) {
                if now_secs().saturating_sub(entry.fetched_at) < ttl.as_secs() {
                    return Ok(serde_json::from_str(&entry.body)?);
                }
            }
        }
        let text = self.send_text(reqwest::Method::POST, url, Some(body), &headers).await?;
        if ttl.is_some() {
            self.write_cache(&key, url, &text);
        }
        Ok(serde_json::from_str(&text)?)
    }

    /// POST without cache, returning the raw response so callers can inspect status codes
    /// (used by the OAuth device-code polling loop).
    pub async fn post_form_raw(&self, url: &str, form: &[(&str, &str)]) -> Result<(u16, String)> {
        Self::ensure_url_allowed(url)?;
        let resp = self
            .client
            .post(url)
            .header(USER_AGENT, self.user_agent())
            .form(form)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        let status = resp.status().as_u16();
        Ok((status, resp.text().await?))
    }

    pub async fn send_json_raw(&self, method: reqwest::Method, url: &str, body: Option<&serde_json::Value>, headers: HeaderMap) -> Result<(u16, String)> {
        Self::ensure_url_allowed(url)?;
        let mut req = self.client.request(method, url).header(USER_AGENT, self.user_agent()).headers(headers).timeout(Duration::from_secs(30));
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        let status = resp.status().as_u16();
        Ok((status, resp.text().await?))
    }

    pub fn cache_size(&self) -> u64 {
        util::dir_size(&self.cache_dir)
    }

    pub fn clear_cache(&self) -> Result<u64> {
        let size = self.cache_size();
        if self.cache_dir.exists() {
            for e in std::fs::read_dir(&self.cache_dir)?.flatten() {
                if e.path().extension().is_some_and(|x| x == "json") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        Ok(size)
    }
}

pub fn host_of(url: &str) -> String {
    url::Url::parse(url).ok().and_then(|u| u.host_str().map(String::from)).unwrap_or_else(|| "server".into())
}

pub fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
    let mut h = HeaderMap::new();
    for (k, v) in pairs {
        if let Ok(v) = HeaderValue::from_str(v) {
            h.insert(*k, v);
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn caches_and_falls_back_when_offline() {
        let mut server = mockito::Server::new_async().await;
        let m = server.mock("GET", "/data").with_status(200).with_body(r#"{"v":1}"#).expect(1).create_async().await;
        let dir = tempfile::tempdir().unwrap();
        let http = Http::new(dir.path().to_path_buf(), build_user_agent(None)).unwrap();
        let url = format!("{}/data", server.url());
        let a: serde_json::Value = http.get_json(&url, Some(Duration::from_secs(60)), HeaderMap::new()).await.unwrap();
        let b: serde_json::Value = http.get_json(&url, Some(Duration::from_secs(60)), HeaderMap::new()).await.unwrap();
        assert_eq!(a, b);
        m.assert_async().await; // second call served from cache

        // Expired cache + server returning 500 three times ⇒ stale cache is used.
        let m500 = server.mock("GET", "/data").with_status(500).expect_at_least(1).create_async().await;
        drop(m);
        let c: std::result::Result<serde_json::Value, _> = http.get_json(&url, Some(Duration::from_secs(0)), HeaderMap::new()).await;
        // 5xx is an Http error ⇒ stale fallback
        assert_eq!(c.unwrap()["v"], 1);
        m500.assert_async().await;
        assert!(http.cache_size() > 0);
        http.clear_cache().unwrap();
        assert_eq!(http.cache_size(), 0);
    }

    #[tokio::test]
    async fn corrupt_cache_entry_is_ignored() {
        let mut server = mockito::Server::new_async().await;
        let _m = server.mock("GET", "/x").with_body(r#"{"ok":true}"#).create_async().await;
        let dir = tempfile::tempdir().unwrap();
        let http = Http::new(dir.path().to_path_buf(), build_user_agent(Some("me@example.com"))).unwrap();
        let url = format!("{}/x", server.url());
        std::fs::write(http.cache_path(&format!("GET {url}")), b"garbage").unwrap();
        let v: serde_json::Value = http.get_json(&url, Some(Duration::from_secs(600)), HeaderMap::new()).await.unwrap();
        assert_eq!(v["ok"], true);
    }

    #[tokio::test]
    async fn rejects_plain_http_to_remote_hosts() {
        let dir = tempfile::tempdir().unwrap();
        let http = Http::new(dir.path().to_path_buf(), build_user_agent(None)).unwrap();
        let r: Result<serde_json::Value> = http.get_json("http://example.com/x", None, HeaderMap::new()).await;
        assert!(r.is_err());
    }
}
