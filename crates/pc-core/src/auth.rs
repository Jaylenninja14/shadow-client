//! Accounts.
//!
//! * Microsoft accounts use the OAuth 2.0 **device code** flow (the user signs in on
//!   microsoft.com; this app never sees the password), then Xbox Live → XSTS → Minecraft
//!   services, followed by an entitlement/profile check.
//! * Tokens are stored with DPAPI in the secret store and refreshed with the refresh token.
//! * Offline profiles exist only for local testing: singleplayer / LAN, clearly labelled, and
//!   never presented as a way around ownership checks.
//! * Nothing here reads, copies or modifies the official launcher's sessions.

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::context::Ctx;
use crate::http::headers;
use crate::secrets::ACCOUNTS_KEY;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// Minecraft profile UUID without dashes
    pub id: String,
    pub username: String,
    /// "msa" | "offline"
    pub kind: String,
    pub xuid: Option<String>,
    pub client_id: Option<String>,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredAccount {
    account: Account,
    refresh_token: Option<String>,
    access_token: Option<String>,
    expires_at: Option<i64>,
}

/// What the game needs from an account.
#[derive(Clone)]
pub struct LaunchAuth {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub xuid: String,
    pub client_id: String,
    pub offline: bool,
}

impl std::fmt::Debug for LaunchAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaunchAuth").field("username", &self.username).field("uuid", &self.uuid).field("offline", &self.offline).finish_non_exhaustive()
    }
}

fn load_all(ctx: &Ctx) -> Result<Vec<StoredAccount>> {
    match ctx.secrets.get(ACCOUNTS_KEY)? {
        Some(s) => Ok(serde_json::from_str(&s)?),
        None => Ok(vec![]),
    }
}

fn save_all(ctx: &Ctx, list: &[StoredAccount]) -> Result<()> {
    ctx.secrets.set(ACCOUNTS_KEY, &serde_json::to_string(list)?)
}

pub fn list_accounts(ctx: &Ctx) -> Result<Vec<Account>> {
    Ok(load_all(ctx)?.into_iter().map(|s| s.account).collect())
}

pub fn remove_account(ctx: &Ctx, id: &str) -> Result<()> {
    let mut all = load_all(ctx)?;
    all.retain(|a| a.account.id != id);
    save_all(ctx, &all)
}

fn upsert(ctx: &Ctx, stored: StoredAccount) -> Result<()> {
    let mut all = load_all(ctx)?;
    all.retain(|a| a.account.id != stored.account.id);
    all.push(stored);
    save_all(ctx, &all)
}

// ---------------------------------------------------------------- offline

pub fn offline_uuid(name: &str) -> String {
    use md5::Digest;
    let mut hash: [u8; 16] = md5::Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    hash[6] = (hash[6] & 0x0f) | 0x30; // version 3
    hash[8] = (hash[8] & 0x3f) | 0x80; // IETF variant
    hex::encode(hash)
}

pub fn add_offline(ctx: &Ctx, username: &str) -> Result<Account> {
    let name = username.trim();
    let valid = (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid {
        return Err(Error::Invalid("Offline names must be 3–16 characters: letters, digits or underscore".into()));
    }
    let account = Account { id: offline_uuid(name), username: name.into(), kind: "offline".into(), xuid: None, client_id: None, added_at: crate::util::now_iso() };
    upsert(ctx, StoredAccount { account: account.clone(), refresh_token: None, access_token: None, expires_at: None })?;
    Ok(account)
}

// ---------------------------------------------------------------- Microsoft

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
    #[serde(skip_serializing)]
    pub device_code: String,
}

#[derive(Deserialize)]
struct DeviceCodeResp {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct TokenResp {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
    error_description: Option<String>,
}

pub fn require_client_id(client_id: Option<&str>) -> Result<String> {
    match client_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => Ok(id.to_string()),
        None => Err(Error::MissingConfig(
            "Microsoft sign-in needs an Azure application (client) ID approved for Minecraft. Add it under Settings → Accounts (see docs/AUTH_SETUP.md), or use the official launcher.".into(),
        )),
    }
}

pub async fn start_device_login(ctx: &Ctx, client_id: &str) -> Result<DeviceCode> {
    let (status, body) = ctx
        .http
        .post_form_raw(&ctx.api.msa_device_code, &[("client_id", client_id), ("scope", "XboxLive.signin offline_access")])
        .await?;
    if status != 200 {
        let detail = serde_json::from_str::<OAuthError>(&body).map(|e| e.error_description.unwrap_or(e.error)).unwrap_or_else(|_| format!("HTTP {status}"));
        return Err(Error::Auth(format!("Microsoft rejected the sign-in request: {}", first_line(&detail))));
    }
    let r: DeviceCodeResp = serde_json::from_str(&body)?;
    Ok(DeviceCode {
        message: r.message.unwrap_or_else(|| format!("Open {} and enter the code {}", r.verification_uri, r.user_code)),
        user_code: r.user_code,
        verification_uri: r.verification_uri,
        expires_in: r.expires_in,
        interval: r.interval.unwrap_or(5).max(1),
        device_code: r.device_code,
    })
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").chars().take(200).collect()
}

/// Poll until the user finishes signing in, then complete the Xbox/Minecraft chain.
pub async fn finish_device_login(ctx: &Ctx, client_id: &str, code: &DeviceCode, cancel: CancellationToken) -> Result<Account> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(code.expires_in);
    let mut interval = code.interval;
    loop {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(interval)) => {}
            _ = cancel.cancelled() => return Err(Error::Cancelled),
        }
        if std::time::Instant::now() > deadline {
            return Err(Error::Auth("The sign-in code expired. Start again.".into()));
        }
        let (status, body) = ctx
            .http
            .post_form_raw(
                &ctx.api.msa_token,
                &[("grant_type", "urn:ietf:params:oauth:grant-type:device_code"), ("client_id", client_id), ("device_code", &code.device_code)],
            )
            .await?;
        if status == 200 {
            let t: TokenResp = serde_json::from_str(&body)?;
            return complete_chain(ctx, client_id, &t.access_token, t.refresh_token).await;
        }
        let err = serde_json::from_str::<OAuthError>(&body).map(|e| e.error).unwrap_or_default();
        match err.as_str() {
            "authorization_pending" => continue,
            "slow_down" => interval += 5,
            "authorization_declined" => return Err(Error::Auth("Sign-in was declined.".into())),
            "expired_token" => return Err(Error::Auth("The sign-in code expired. Start again.".into())),
            _ => return Err(Error::Auth(format!("Microsoft sign-in failed ({})", if err.is_empty() { format!("HTTP {status}") } else { err }))),
        }
    }
}

/// Translate XSTS XErr codes into actionable messages.
pub fn xsts_error_message(xerr: u64) -> String {
    match xerr {
        2148916227 => "This Xbox account is banned.".into(),
        2148916229 => "This account is restricted by parental controls. An adult must allow online play in the Xbox family settings.".into(),
        2148916233 => "This Microsoft account has no Xbox profile yet. Sign in once at xbox.com to create one, then try again.".into(),
        2148916234 => "The Xbox terms of use have not been accepted for this account. Sign in at xbox.com to accept them.".into(),
        2148916235 => "Xbox Live is not available in this account's country/region.".into(),
        2148916236 | 2148916237 => "This account needs adult verification (South Korea). Complete it on xbox.com.".into(),
        2148916238 => "This is a child account. It must be added to a Microsoft family by an adult before it can play.".into(),
        other => format!("Xbox Live refused sign-in (XErr {other})."),
    }
}

async fn post_json_status(ctx: &Ctx, url: &str, body: serde_json::Value, bearer: Option<&str>) -> Result<(u16, String)> {
    let mut h = headers(&[("accept", "application/json")]);
    if let Some(b) = bearer {
        h.insert("authorization", format!("Bearer {b}").parse().map_err(|_| Error::Auth("bad token".into()))?);
    }
    ctx.http.send_json_raw(reqwest::Method::POST, url, Some(&body), h).await
}

async fn get_bearer(ctx: &Ctx, url: &str, bearer: &str) -> Result<(u16, String)> {
    let mut h = headers(&[("accept", "application/json")]);
    h.insert("authorization", format!("Bearer {bearer}").parse().map_err(|_| Error::Auth("bad token".into()))?);
    ctx.http.send_json_raw(reqwest::Method::GET, url, None, h).await
}

struct McLogin {
    access_token: String,
    expires_in: i64,
    xuid: Option<String>,
}

async fn xbox_to_minecraft(ctx: &Ctx, msa_access_token: &str) -> Result<McLogin> {
    // 1. Xbox Live user token
    let (s, body) = post_json_status(
        ctx,
        &ctx.api.xbox_user,
        json!({
            "Properties": {"AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={msa_access_token}")},
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
        None,
    )
    .await?;
    if s != 200 {
        return Err(Error::Auth(format!("Xbox Live sign-in failed (HTTP {s}).")));
    }
    let v: serde_json::Value = serde_json::from_str(&body)?;
    let xbl = v["Token"].as_str().ok_or_else(|| Error::Auth("Xbox Live returned no token".into()))?.to_string();
    let uhs = v["DisplayClaims"]["xui"][0]["uhs"].as_str().ok_or_else(|| Error::Auth("Xbox Live returned no user hash".into()))?.to_string();

    // 2. XSTS token for Minecraft services
    let (s, body) = post_json_status(
        ctx,
        &ctx.api.xbox_xsts,
        json!({"Properties": {"SandboxId": "RETAIL", "UserTokens": [xbl]}, "RelyingParty": "rp://api.minecraftservices.com/", "TokenType": "JWT"}),
        None,
    )
    .await?;
    if s == 401 {
        let xerr = serde_json::from_str::<serde_json::Value>(&body).ok().and_then(|v| v["XErr"].as_u64()).unwrap_or(0);
        return Err(Error::Auth(xsts_error_message(xerr)));
    }
    if s != 200 {
        return Err(Error::Auth(format!("Xbox security token request failed (HTTP {s}).")));
    }
    let v: serde_json::Value = serde_json::from_str(&body)?;
    let xsts = v["Token"].as_str().ok_or_else(|| Error::Auth("XSTS returned no token".into()))?.to_string();
    let xuid = v["DisplayClaims"]["xui"][0]["xid"].as_str().map(String::from);

    // 3. Minecraft services login
    let (s, body) = post_json_status(
        ctx,
        &format!("{}/authentication/login_with_xbox", ctx.api.mc_services),
        json!({"identityToken": format!("XBL3.0 x={uhs};{xsts}")}),
        None,
    )
    .await?;
    if s == 403 || s == 401 {
        return Err(Error::Auth(
            "Minecraft services rejected this app's sign-in. The Azure application ID has probably not been approved for Minecraft yet (see docs/AUTH_SETUP.md).".into(),
        ));
    }
    if s != 200 {
        return Err(Error::Auth(format!("Minecraft services login failed (HTTP {s}).")));
    }
    let v: serde_json::Value = serde_json::from_str(&body)?;
    Ok(McLogin {
        access_token: v["access_token"].as_str().ok_or_else(|| Error::Auth("no Minecraft access token".into()))?.to_string(),
        expires_in: v["expires_in"].as_i64().unwrap_or(86400),
        xuid,
    })
}

async fn fetch_profile(ctx: &Ctx, mc_token: &str) -> Result<(String, String)> {
    // Entitlements first (purchase), then the profile (Game Pass accounts may have empty
    // entitlements but a valid profile).
    let (s, body) = get_bearer(ctx, &format!("{}/entitlements/mcstore", ctx.api.mc_services), mc_token).await?;
    let owns = s == 200
        && serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["items"].as_array().map(|a| !a.is_empty()))
            .unwrap_or(false);
    let (s, body) = get_bearer(ctx, &format!("{}/minecraft/profile", ctx.api.mc_services), mc_token).await?;
    if s == 404 {
        return Err(Error::Auth(if owns {
            "This account owns Minecraft but has no profile name yet. Open minecraft.net to choose one.".into()
        } else {
            "This Microsoft account does not own Minecraft: Java Edition.".into()
        }));
    }
    if s != 200 {
        return Err(Error::Auth(format!("Could not read the Minecraft profile (HTTP {s}).")));
    }
    let v: serde_json::Value = serde_json::from_str(&body)?;
    let id = v["id"].as_str().ok_or_else(|| Error::Auth("profile has no id".into()))?.to_string();
    let name = v["name"].as_str().ok_or_else(|| Error::Auth("profile has no name".into()))?.to_string();
    Ok((id, name))
}

async fn complete_chain(ctx: &Ctx, client_id: &str, msa_access: &str, refresh: Option<String>) -> Result<Account> {
    let login = xbox_to_minecraft(ctx, msa_access).await?;
    let (id, name) = fetch_profile(ctx, &login.access_token).await?;
    let account = Account { id, username: name, kind: "msa".into(), xuid: login.xuid, client_id: Some(client_id.into()), added_at: crate::util::now_iso() };
    upsert(
        ctx,
        StoredAccount {
            account: account.clone(),
            refresh_token: refresh,
            access_token: Some(login.access_token),
            expires_at: Some(chrono::Utc::now().timestamp() + login.expires_in),
        },
    )?;
    Ok(account)
}

/// Return launch credentials, refreshing Microsoft tokens if they are about to expire.
pub async fn launch_auth(ctx: &Ctx, account_id: &str) -> Result<LaunchAuth> {
    let all = load_all(ctx)?;
    let stored = all.into_iter().find(|a| a.account.id == account_id).ok_or_else(|| Error::Auth("The selected account no longer exists. Pick another account.".into()))?;
    if stored.account.kind == "offline" {
        return Ok(LaunchAuth {
            username: stored.account.username.clone(),
            uuid: stored.account.id.clone(),
            access_token: "offline".into(),
            user_type: "legacy".into(),
            xuid: "0".into(),
            client_id: String::new(),
            offline: true,
        });
    }
    let now = chrono::Utc::now().timestamp();
    let fresh = stored.expires_at.is_some_and(|e| e - 300 > now) && stored.access_token.is_some();
    let (token, account) = if fresh {
        (stored.access_token.clone().unwrap(), stored.account.clone())
    } else {
        let refresh = stored.refresh_token.clone().ok_or_else(|| Error::Auth("Your Microsoft sign-in expired. Sign in again.".into()))?;
        let client_id = require_client_id(stored.account.client_id.as_deref())?;
        let (s, body) = ctx
            .http
            .post_form_raw(
                &ctx.api.msa_token,
                &[("grant_type", "refresh_token"), ("client_id", &client_id), ("refresh_token", &refresh), ("scope", "XboxLive.signin offline_access")],
            )
            .await?;
        if s != 200 {
            return Err(Error::Auth("Your Microsoft sign-in expired or was revoked. Sign in again.".into()));
        }
        let t: TokenResp = serde_json::from_str(&body)?;
        let acc = complete_chain(ctx, &client_id, &t.access_token, t.refresh_token.or(Some(refresh))).await?;
        let all = load_all(ctx)?;
        let tok = all.iter().find(|a| a.account.id == acc.id).and_then(|a| a.access_token.clone()).ok_or_else(|| Error::Auth("token missing after refresh".into()))?;
        (tok, acc)
    };
    Ok(LaunchAuth {
        username: account.username.clone(),
        uuid: account.id.clone(),
        access_token: token,
        user_type: "msa".into(),
        xuid: account.xuid.clone().unwrap_or_else(|| "0".into()),
        client_id: account.client_id.clone().unwrap_or_default(),
        offline: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_matches_vanilla_algorithm() {
        // Known value: UUID.nameUUIDFromBytes("OfflinePlayer:Notch") = b50ad385-829d-3141-a216-7e7d7539ba7f
        assert_eq!(offline_uuid("Notch"), "b50ad385829d3141a2167e7d7539ba7f");
    }

    #[test]
    fn xerr_messages() {
        assert!(xsts_error_message(2148916233).contains("no Xbox profile"));
        assert!(xsts_error_message(2148916238).contains("child account"));
        assert!(xsts_error_message(1).contains("XErr 1"));
    }

    #[test]
    fn client_id_required() {
        assert!(matches!(require_client_id(None), Err(Error::MissingConfig(_))));
        assert!(matches!(require_client_id(Some("  ")), Err(Error::MissingConfig(_))));
        assert_eq!(require_client_id(Some("abc")).unwrap(), "abc");
    }
}
