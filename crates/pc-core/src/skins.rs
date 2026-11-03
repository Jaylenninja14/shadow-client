//! Skin library: create/edit happens in the UI; this module stores, validates, imports,
//! exports and applies skins.
//!
//! * Library: `skins/<id>.png` + `skins/library.json` (name, model, timestamps, source).
//! * Import from a file or from a player's *public* skin (Mojang profile API).
//! * Apply to a Microsoft account through the official Minecraft services endpoint.
//! * Apply locally: a generated resource pack that replaces the default player skins in one
//!   profile. It only changes what this PC shows (useful offline/singleplayer); other players
//!   never see it.

use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::minecraft::options::{self, OptionsFile};
use crate::mods::metadata::compare_versions;
use crate::{util, Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkinEntry {
    pub id: String,
    pub name: String,
    /// "classic" (4 px arms) | "slim" (3 px arms)
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    /// "created" | "file" | "player:<name>"
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinWithData {
    #[serde(flatten)]
    pub entry: SkinEntry,
    pub data_url: String,
}

fn dir(ctx: &Ctx) -> PathBuf {
    ctx.paths.root.join("skins")
}

fn library_file(ctx: &Ctx) -> PathBuf {
    dir(ctx).join("library.json")
}

fn load(ctx: &Ctx) -> Vec<SkinEntry> {
    util::read_json_opt(&library_file(ctx)).ok().flatten().unwrap_or_default()
}

fn save_lib(ctx: &Ctx, list: &[SkinEntry]) -> Result<()> {
    util::write_json(&library_file(ctx), &list)
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 40 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(Error::Invalid("invalid skin id".into()));
    }
    Ok(())
}

fn validate_model(m: &str) -> Result<()> {
    if m == "classic" || m == "slim" {
        Ok(())
    } else {
        Err(Error::Invalid("Skin model must be classic or slim".into()))
    }
}

/// Check a PNG is a Minecraft skin: valid signature, IHDR 64×64 or 64×32, small file.
pub fn validate_png(bytes: &[u8]) -> Result<(u32, u32)> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 33 || bytes[..8] != SIG || &bytes[12..16] != b"IHDR" {
        return Err(Error::Invalid("That file is not a PNG image".into()));
    }
    if bytes.len() > 256 * 1024 {
        return Err(Error::Invalid("Skin files must be smaller than 256 KB".into()));
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if w != 64 || (h != 64 && h != 32) {
        return Err(Error::Invalid(format!("Minecraft skins are 64×64 (or legacy 64×32) pixels; this image is {w}×{h}")));
    }
    Ok((w, h))
}

fn data_url(bytes: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

pub fn decode_data_url(s: &str) -> Result<Vec<u8>> {
    let b64 = s.strip_prefix("data:image/png;base64,").unwrap_or(s);
    base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|_| Error::Invalid("Invalid image data".into()))
}

pub fn list(ctx: &Ctx) -> Vec<SkinWithData> {
    load(ctx)
        .into_iter()
        .filter_map(|e| {
            let bytes = std::fs::read(dir(ctx).join(format!("{}.png", e.id))).ok()?;
            Some(SkinWithData { data_url: data_url(&bytes), entry: e })
        })
        .collect()
}

pub fn png_path(ctx: &Ctx, id: &str) -> Result<PathBuf> {
    validate_id(id)?;
    let p = dir(ctx).join(format!("{id}.png"));
    if !p.is_file() {
        return Err(Error::NotFound("skin".into()));
    }
    Ok(p)
}

/// Create or update a skin. `png` must be a 64×64 (or 64×32) PNG.
pub fn save(ctx: &Ctx, id: Option<&str>, name: &str, model: &str, png: &[u8], source: &str) -> Result<SkinEntry> {
    validate_png(png)?;
    validate_model(model)?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return Err(Error::Invalid("Skin names must be 1–40 characters".into()));
    }
    let mut lib = load(ctx);
    let now = util::now_iso();
    let entry = match id {
        Some(id) => {
            validate_id(id)?;
            let e = lib.iter_mut().find(|e| e.id == id).ok_or_else(|| Error::NotFound("skin".into()))?;
            e.name = name.into();
            e.model = model.into();
            e.updated_at = now;
            e.clone()
        }
        None => {
            let e = SkinEntry {
                id: uuid::Uuid::new_v4().simple().to_string()[..12].to_string(),
                name: name.into(),
                model: model.into(),
                created_at: now.clone(),
                updated_at: now,
                source: source.into(),
            };
            lib.push(e.clone());
            e
        }
    };
    util::atomic_write(&dir(ctx).join(format!("{}.png", entry.id)), png)?;
    save_lib(ctx, &lib)?;
    Ok(entry)
}

/// Delete a skin: the PNG moves to backups/skins (recoverable).
pub fn delete(ctx: &Ctx, id: &str) -> Result<()> {
    validate_id(id)?;
    let mut lib = load(ctx);
    let before = lib.len();
    lib.retain(|e| e.id != id);
    if lib.len() == before {
        return Err(Error::NotFound("skin".into()));
    }
    let p = dir(ctx).join(format!("{id}.png"));
    if p.exists() {
        util::move_path(&p, &ctx.paths.backups_dir().join("skins").join(format!("{id}-{}.png", util::timestamp_slug())))?;
    }
    save_lib(ctx, &lib)
}

pub fn export(ctx: &Ctx, id: &str, dest: &Path) -> Result<()> {
    std::fs::copy(png_path(ctx, id)?, dest)?;
    Ok(())
}

/// Read a PNG from disk for import (the UI converts legacy 64×32 skins and saves).
pub fn read_png_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    validate_png(&bytes)?;
    Ok(data_url(&bytes))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSkin {
    pub name: String,
    pub model: String,
    pub data_url: String,
}

/// Fetch a player's public skin by name (Mojang profile + session server).
pub async fn fetch_player_skin(ctx: &Ctx, name: &str) -> Result<PlayerSkin> {
    let name = name.trim();
    if !(3..=16).contains(&name.len()) || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Error::Invalid("Player names are 3–16 letters, digits or underscores".into()));
    }
    let profile: serde_json::Value = ctx
        .http
        .get_json(&format!("{}/users/profiles/minecraft/{name}", ctx.api.mojang_api), Some(std::time::Duration::from_secs(600)), Default::default())
        .await
        .map_err(|e| match e {
            Error::NotFound(_) => Error::NotFound(format!("No Minecraft player named {name}")),
            other => other,
        })?;
    let uuid = profile["id"].as_str().filter(|s| s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit())).ok_or_else(|| Error::NotFound(format!("No Minecraft player named {name}")))?;
    let real_name = profile["name"].as_str().unwrap_or(name).to_string();
    let session: serde_json::Value = ctx
        .http
        .get_json(&format!("{}/session/minecraft/profile/{uuid}", ctx.api.session_server), Some(std::time::Duration::from_secs(600)), Default::default())
        .await?;
    let textures_b64 = session["properties"]
        .as_array()
        .and_then(|a| a.iter().find(|p| p["name"] == "textures"))
        .and_then(|p| p["value"].as_str())
        .ok_or_else(|| Error::NotFound(format!("{real_name} has no custom skin")))?;
    let textures: serde_json::Value = serde_json::from_slice(&base64::engine::general_purpose::STANDARD.decode(textures_b64).map_err(|_| Error::Invalid("bad textures data".into()))?)?;
    let skin = &textures["textures"]["SKIN"];
    let mut url = skin["url"].as_str().ok_or_else(|| Error::NotFound(format!("{real_name} uses a default skin")))?.to_string();
    if url.starts_with("http://textures.minecraft.net/") {
        url = url.replacen("http://", "https://", 1);
    }
    let parsed = url::Url::parse(&url).map_err(|_| Error::Invalid("bad skin URL".into()))?;
    let host_ok = parsed.host_str() == Some("textures.minecraft.net") || (cfg!(test) && parsed.host_str() == Some("127.0.0.1"));
    if !host_ok {
        return Err(Error::Invalid("Skin URL is not on textures.minecraft.net".into()));
    }
    let resp = ctx.http.client.get(url.as_str()).header(reqwest::header::USER_AGENT, ctx.http.user_agent()).send().await?;
    if !resp.status().is_success() {
        return Err(Error::Http(format!("skin download failed (HTTP {})", resp.status())));
    }
    let bytes = resp.bytes().await?;
    validate_png(&bytes)?;
    let model = if skin["metadata"]["model"] == "slim" { "slim" } else { "classic" };
    Ok(PlayerSkin { name: real_name, model: model.into(), data_url: data_url(&bytes) })
}

/// Upload a skin to a Microsoft account (official Minecraft services API).
pub async fn upload_to_account(ctx: &Ctx, account_id: &str, skin_id: &str) -> Result<()> {
    let auth = crate::auth::launch_auth(ctx, account_id).await?;
    if auth.offline {
        return Err(Error::Invalid("Offline profiles have no online skin. Use “Use in a profile” to see this skin locally, or sign in with Microsoft.".into()));
    }
    let lib = load(ctx);
    let entry = lib.iter().find(|e| e.id == skin_id).ok_or_else(|| Error::NotFound("skin".into()))?;
    let bytes = std::fs::read(png_path(ctx, skin_id)?)?;
    let part = reqwest::multipart::Part::bytes(bytes).file_name("skin.png").mime_str("image/png").map_err(|e| Error::msg(e.to_string()))?;
    let form = reqwest::multipart::Form::new().text("variant", entry.model.clone()).part("file", part);
    let url = format!("{}/minecraft/profile/skins", ctx.api.mc_services);
    crate::http::Http::ensure_url_allowed(&url)?;
    let resp = ctx
        .http
        .client
        .post(&url)
        .header(reqwest::header::USER_AGENT, ctx.http.user_agent())
        .bearer_auth(&auth.access_token)
        .multipart(form)
        .send()
        .await?;
    match resp.status().as_u16() {
        200..=299 => Ok(()),
        401 => Err(Error::Auth("Your Microsoft sign-in expired. Sign in again.".into())),
        429 => Err(Error::Http("Mojang limits how often skins can be changed. Try again in a minute.".into())),
        s => Err(Error::Http(format!("Minecraft services refused the skin (HTTP {s})"))),
    }
}

/// Default-skin texture names (1.19.3+) and legacy paths, all replaced by the local pack.
const DEFAULT_SKINS: &[&str] = &["alex", "ari", "efe", "kai", "makena", "noor", "steve", "sunny", "zuri"];
pub const LOCAL_PACK_NAME: &str = "Shadow Client Skin.zip";
/// The pack's name before the client was renamed; switched off when the new one is applied.
const LEGACY_PACK_NAME: &str = "Sunny Client Skin.zip";

/// Resource pack format for older versions (newer ones read the supported range).
fn pack_format(mc: &str) -> u32 {
    let table: &[(&str, u32)] = &[
        ("1.20", 15),
        ("1.19.4", 13),
        ("1.19.3", 12),
        ("1.19", 9),
        ("1.18", 8),
        ("1.17", 7),
        ("1.16.2", 6),
        ("1.15", 5),
        ("1.13", 4),
        ("1.11", 3),
        ("1.9", 2),
    ];
    for (min, f) in table {
        if compare_versions(mc, min).is_none_or(|o| o != std::cmp::Ordering::Less) {
            return *f;
        }
    }
    1
}

/// Build the local-skin resource pack bytes.
pub fn build_local_pack(png: &[u8], mc: &str) -> Result<Vec<u8>> {
    validate_png(png)?;
    let f = pack_format(mc);
    let mcmeta = serde_json::json!({
        "pack": {
            "description": "Your skin from Shadow Client (only visible on this PC)",
            "pack_format": f,
            "supported_formats": [f, 999],
            "min_format": f,
            "max_format": 999
        }
    });
    let mut cur = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut cur);
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file("pack.mcmeta", opts)?;
        z.write_all(serde_json::to_vec_pretty(&mcmeta)?.as_slice())?;
        for name in DEFAULT_SKINS {
            for variant in ["wide", "slim"] {
                z.start_file(format!("assets/minecraft/textures/entity/player/{variant}/{name}.png"), opts)?;
                z.write_all(png)?;
            }
        }
        for legacy in ["steve", "alex"] {
            z.start_file(format!("assets/minecraft/textures/entity/{legacy}.png"), opts)?;
            z.write_all(png)?;
        }
        z.finish()?;
    }
    Ok(cur.into_inner())
}

/// Install the skin as a local resource pack in a profile and switch it on.
pub fn apply_locally(ctx: &Ctx, skin_id: &str, game_dir: &Path, profile_data: &Path, mc: &str) -> Result<String> {
    let png = std::fs::read(png_path(ctx, skin_id)?)?;
    let pack = build_local_pack(&png, mc)?;
    let packs = game_dir.join("resourcepacks");
    std::fs::create_dir_all(&packs)?;
    util::atomic_write(&packs.join(LOCAL_PACK_NAME), &pack)?;
    let opt_path = game_dir.join("options.txt");
    match OptionsFile::load(&opt_path)? {
        Some(mut file) => {
            options::backup(game_dir, profile_data)?;
            let id = format!("file/{LOCAL_PACK_NAME}");
            let mut list: Vec<String> = file.get("resourcePacks").and_then(|v| serde_json::from_str(v).ok()).unwrap_or_else(|| vec!["vanilla".into()]);
            let legacy = format!("file/{LEGACY_PACK_NAME}");
            list.retain(|x| x != &id && x != &legacy);
            list.push(id.clone());
            file.set("resourcePacks", &serde_json::to_string(&list)?);
            // Accept the pack even if its declared format is older/newer than this version.
            let mut inc: Vec<String> = file.get("incompatibleResourcePacks").and_then(|v| serde_json::from_str(v).ok()).unwrap_or_default();
            if !inc.contains(&id) {
                inc.push(id);
            }
            file.set("incompatibleResourcePacks", &serde_json::to_string(&inc)?);
            util::atomic_write(&opt_path, file.render().as_bytes())?;
            Ok("Skin pack installed and switched on. It shows the next time this profile starts.".into())
        }
        None => Ok("Skin pack installed. Start this profile once, then turn on “Shadow Client Skin” under Mods → Resource packs (or apply again).".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny valid 64×64 PNG header + IHDR (enough for validation).
    pub fn fake_png(w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        v
    }

    #[test]
    fn png_validation() {
        assert!(validate_png(&fake_png(64, 64)).is_ok());
        assert!(validate_png(&fake_png(64, 32)).is_ok());
        assert!(validate_png(&fake_png(128, 128)).is_err());
        assert!(validate_png(b"not a png at all, definitely not one").is_err());
    }

    #[test]
    fn library_crud_and_local_pack() {
        let d = tempfile::tempdir().unwrap();
        let (ctx, _) = crate::testutil::ctx(d.path(), "http://127.0.0.1:9");
        let e = save(&ctx, None, "Shadow", "slim", &fake_png(64, 64), "created").unwrap();
        assert_eq!(list(&ctx).len(), 1);
        assert!(list(&ctx)[0].data_url.starts_with("data:image/png;base64,"));
        let e2 = save(&ctx, Some(&e.id), "Shadow 2", "classic", &fake_png(64, 64), "created").unwrap();
        assert_eq!(e2.id, e.id);
        assert_eq!(list(&ctx)[0].entry.name, "Shadow 2");
        assert!(save(&ctx, None, "", "classic", &fake_png(64, 64), "created").is_err());
        assert!(save(&ctx, None, "x", "huge", &fake_png(64, 64), "created").is_err());
        assert!(png_path(&ctx, "../evil").is_err());

        // local pack
        let game = d.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("options.txt"), "version:4671\nresourcePacks:[\"vanilla\"]\nkey_key.jump:key.keyboard.space\n").unwrap();
        let msg = apply_locally(&ctx, &e.id, &game, &d.path().join("pdata"), "1.21.11").unwrap();
        assert!(msg.contains("switched on"));
        let opts = std::fs::read_to_string(game.join("options.txt")).unwrap();
        assert!(opts.contains("file/Shadow Client Skin.zip") && opts.contains("key_key.jump"));
        let mut z = zip::ZipArchive::new(std::fs::File::open(game.join("resourcepacks").join(LOCAL_PACK_NAME)).unwrap()).unwrap();
        assert!(z.by_name("assets/minecraft/textures/entity/player/wide/steve.png").is_ok());
        assert!(z.by_name("assets/minecraft/textures/entity/player/slim/alex.png").is_ok());
        assert!(z.by_name("pack.mcmeta").is_ok());

        delete(&ctx, &e.id).unwrap();
        assert!(list(&ctx).is_empty());
        assert!(delete(&ctx, &e.id).is_err());
    }

    #[test]
    fn pack_formats() {
        assert_eq!(pack_format("1.8.9"), 1);
        assert_eq!(pack_format("1.12.2"), 3);
        assert_eq!(pack_format("1.16.5"), 6);
        assert_eq!(pack_format("1.21.11"), 15);
    }

    #[tokio::test]
    async fn player_skin_and_upload_paths() {
        let mut s = mockito::Server::new_async().await;
        let base = s.url();
        let d = tempfile::tempdir().unwrap();
        let (ctx, _) = crate::testutil::ctx(d.path(), &base);
        let png = fake_png(64, 64);
        let textures = serde_json::json!({"textures": {"SKIN": {"url": format!("{base}/tex/abc"), "metadata": {"model": "slim"}}}});
        let b64 = base64::engine::general_purpose::STANDARD.encode(textures.to_string());
        let _a = s.mock("GET", "/mojang/users/profiles/minecraft/Notch").with_body(r#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"}"#).create_async().await;
        let _b = s
            .mock("GET", "/session/session/minecraft/profile/069a79f444e94726a5befca90e38aaf5")
            .with_body(serde_json::json!({"id": "x", "name": "Notch", "properties": [{"name": "textures", "value": b64}]}).to_string())
            .create_async()
            .await;
        let _c = s.mock("GET", "/tex/abc").with_body(png.clone()).create_async().await;
        let skin = fetch_player_skin(&ctx, "Notch").await.unwrap();
        assert_eq!((skin.name.as_str(), skin.model.as_str()), ("Notch", "slim"));
        let _nf = s.mock("GET", "/mojang/users/profiles/minecraft/Nobody_123").with_status(404).create_async().await;
        assert!(fetch_player_skin(&ctx, "Nobody_123").await.unwrap_err().to_string().contains("No Minecraft player"));
        assert!(fetch_player_skin(&ctx, "bad name!").await.is_err());

        // Offline accounts cannot upload.
        let off = crate::auth::add_offline(&ctx, "Tester").unwrap();
        let e = save(&ctx, None, "S", "classic", &png, "created").unwrap();
        assert!(upload_to_account(&ctx, &off.id, &e.id).await.unwrap_err().to_string().contains("Offline"));
    }
}
