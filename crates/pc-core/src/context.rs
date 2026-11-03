//! Shared services handed to every engine operation.

use std::sync::Arc;

use crate::download::DownloadManager;
use crate::events::Sink;
use crate::http::Http;
use crate::minecraft::Endpoints;
use crate::paths::AppPaths;
use crate::secrets::SecretStore;
use crate::Result;

/// Base URLs of third-party APIs (overridable for tests).
#[derive(Debug, Clone)]
pub struct ApiEndpoints {
    pub minecraft: Endpoints,
    pub modrinth: String,
    pub curseforge: String,
    pub msa_device_code: String,
    pub msa_token: String,
    pub xbox_user: String,
    pub xbox_xsts: String,
    pub mc_services: String,
    pub mojang_api: String,
    pub session_server: String,
}

impl Default for ApiEndpoints {
    fn default() -> Self {
        Self {
            minecraft: Endpoints::default(),
            modrinth: "https://api.modrinth.com/v2".into(),
            curseforge: "https://api.curseforge.com".into(),
            msa_device_code: "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode".into(),
            msa_token: "https://login.microsoftonline.com/consumers/oauth2/v2.0/token".into(),
            xbox_user: "https://user.auth.xboxlive.com/user/authenticate".into(),
            xbox_xsts: "https://xsts.auth.xboxlive.com/xsts/authorize".into(),
            mc_services: "https://api.minecraftservices.com".into(),
            mojang_api: "https://api.mojang.com".into(),
            session_server: "https://sessionserver.mojang.com".into(),
        }
    }
}

impl ApiEndpoints {
    pub fn mock(base: &str) -> Self {
        Self {
            minecraft: Endpoints::mock(base),
            modrinth: format!("{base}/modrinth/v2"),
            curseforge: format!("{base}/curseforge"),
            msa_device_code: format!("{base}/msa/devicecode"),
            msa_token: format!("{base}/msa/token"),
            xbox_user: format!("{base}/xbox/user"),
            xbox_xsts: format!("{base}/xbox/xsts"),
            mc_services: format!("{base}/mc"),
            mojang_api: format!("{base}/mojang"),
            session_server: format!("{base}/session"),
        }
    }
}

#[derive(Clone)]
pub struct Ctx {
    pub paths: AppPaths,
    pub http: Arc<Http>,
    pub downloads: Arc<DownloadManager>,
    pub api: ApiEndpoints,
    pub sink: Sink,
    pub secrets: Arc<SecretStore>,
}

impl Ctx {
    pub fn new(paths: AppPaths, api: ApiEndpoints, sink: Sink, user_agent: String) -> Result<Self> {
        paths.ensure()?;
        let http = Arc::new(Http::new(paths.http_cache_dir(), user_agent)?);
        let downloads = Arc::new(DownloadManager::new(http.clone(), sink.clone(), Some(paths.downloads_history_file())));
        let secrets = Arc::new(SecretStore::new(paths.secrets_file()));
        Ok(Self { paths, http, downloads, api, sink, secrets })
    }
}
