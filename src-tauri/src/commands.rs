//! IPC commands. Each one is a thin wrapper around pc-core; validation lives in the engine.

use std::path::{Path, PathBuf};

use pc_core::auth::{self, Account};
use pc_core::download::JobInfo;
use pc_core::integration::{self, OfficialLauncher};
use pc_core::java::{self, JavaInstall};
use pc_core::minecraft::detect::{self, DetectedInstall};
use pc_core::minecraft::install::{self, VersionSummary};
use pc_core::minecraft::launch::{GameState, RunningInfo};
use pc_core::minecraft::loaders::{self, LoaderKind};
use pc_core::minecraft::options;
use pc_core::modpacks::{self, PackInstallReport, PackSummary};
use pc_core::mods::curseforge as cf;
use pc_core::mods::manage::{self, InstalledMod, ProfileScope};
use pc_core::mods::modrinth as mr;
use pc_core::mods::resolver::{self, InstallPlan};
use pc_core::mods::{builtin, ContentKind};
use pc_core::performance::benchmark::{self, BenchmarkResult};
use pc_core::performance::hardware::{self, HardwareInfo};
use pc_core::performance::presets;
use pc_core::play::{self, Preflight};
use pc_core::profiles::{DeleteReport, NewProfile, Profile, ProfileKind, Profiles};
use pc_core::secrets::CURSEFORGE_KEY;
use pc_core::settings::{GraphicsSettings, Settings};
use pc_core::setup::{self, PerfModStatus, SetupReport};
use pc_core::{redact, resources, skins, util, Error};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use tauri_plugin_opener::OpenerExt;
use tokio_util::sync::CancellationToken;

use crate::state::{AppState, PendingPlan};

type R<T> = Result<T, Error>;

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        app_info, app_ready, get_settings, save_settings, reset_settings, get_app_state, set_app_state,
        detect_installs, official_launcher, open_official_launcher, open_official_with_profile, import_install, register_with_official,
        list_versions, loader_game_versions, loader_versions, recommended_performance_version, repair_profile,
        list_profiles, get_profile, create_profile, update_profile, rename_profile, duplicate_profile, delete_profile,
        export_profile, check_version_change, apply_version_change, open_profile_folder,
        list_accounts, add_offline_account, remove_account, set_active_account, msa_start, msa_cancel,
        curseforge_status, set_curseforge_key, clear_curseforge_key,
        preflight, play, running_games, kill_game, game_log_tail, list_java, download_java, memory_recommendation,
        mods_list, mods_check_updates, modrinth_search, modrinth_project, modrinth_versions, curseforge_search, curseforge_mod,
        plan_install, plan_update, execute_plan, discard_plan, mod_set_enabled, mod_remove, mod_backups, mod_restore,
        record_recent, builtin_mods, install_builtins, performance_mods_status, install_performance_mods,
        resourcepacks_list, resourcepack_set_active, shader_state, shader_set, content_remove,
        hardware_info, run_benchmark, preset_list, preset_preview, apply_preset, graphics_warnings, current_graphics,
        options_backups, options_restore,
        modpack_inspect, modpack_install_file, modpack_install_remote, modpack_update_file,
        downloads_list, download_cancel, download_retry, downloads_clear,
        logs_list, log_read, diagnostics_report, cache_info, clear_cache, open_path, open_url, open_data_dir,
        features_status, features_plan, game_options_get, game_options_set,
        menu_read_image, menu_state, menu_apply, menu_remove, hud_state, hud_apply, hotkeys_get, hotkeys_set, server_plugin_install,
        bedrock_info, bedrock_launch, bedrock_packs, bedrock_import, bedrock_remove, bedrock_open_folder, bedrock_cf_classes, bedrock_cf_search, bedrock_cf_install,
        bedrock_menu, bedrock_menu_set, bedrock_menu_restore,
        servers_list, servers_save, server_ping_java, server_ping_bedrock, play_join, update_check, update_install, share_pending, share_export, share_read, share_import, screenshots_list, screenshot_remove,
        bedrock_worlds, bedrock_world_backup, bedrock_world_backups, bedrock_world_restore, bedrock_servers, bedrock_server_add, bedrock_server_remove, bedrock_join,
        skins_list, skin_save, skin_delete, skin_export, skin_read_file, skin_fetch_player, skin_upload, skin_apply_local,
    ]
}

fn profiles(s: &AppState) -> Profiles<'_> {
    Profiles::new(&s.ctx.paths)
}

fn scope(s: &AppState, profile_id: &str) -> R<(Profile, ProfileScope)> {
    let p = profiles(s).get(profile_id)?;
    let sc = play::scope_for(&s.ctx, &p);
    Ok((p, sc))
}

// ------------------------------------------------------------------ app & settings

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    data_dir: PathBuf,
    portable: bool,
    logs_dir: PathBuf,
    minecraft_dir: PathBuf,
    startup_ms: u128,
}

#[tauri::command]
fn app_info(state: State<'_, AppState>) -> AppInfo {
    let settings = state.settings();
    AppInfo {
        version: pc_core::VERSION.into(),
        data_dir: state.ctx.paths.root.clone(),
        portable: state.ctx.paths.portable,
        logs_dir: state.ctx.paths.logs_dir(),
        minecraft_dir: settings.minecraft_dir.map(PathBuf::from).unwrap_or_else(pc_core::paths::default_minecraft_dir),
        startup_ms: state.started.elapsed().as_millis(),
    }
}

/// Called by the UI after its first render; records the real startup time.
#[tauri::command]
fn app_ready(state: State<'_, AppState>) -> u128 {
    let ms = state.started.elapsed().as_millis();
    tracing::info!("UI ready {ms} ms after process start");
    ms
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

fn apply_runtime_settings(state: &AppState, s: &Settings) {
    state.ctx.downloads.set_limits(s.download_concurrency, s.download_retries);
    state.ctx.http.set_default_ttl(std::time::Duration::from_secs(s.cache_ttl_minutes as u64 * 60));
    state.ctx.http.set_user_agent(pc_core::http::build_user_agent(s.modrinth_contact.as_deref()));
    let level = if s.debug_logging { tracing_subscriber::filter::LevelFilter::DEBUG } else { tracing_subscriber::filter::LevelFilter::INFO };
    let _ = state.log_handle.modify(|f| *f = level);
}

#[tauri::command]
fn save_settings(state: State<'_, AppState>, settings: Settings) -> R<Settings> {
    let mut s = settings;
    s.validate()?;
    // "Only show the profile you open" switched off: the launcher's other installations come back.
    if state.settings().official_only_this && !s.official_only_this {
        // the folder they were put aside in (before any folder change in this same save)
        match integration::show_all(&mc_dir(&state)) {
            Ok(n) => tracing::info!("official launcher: {n} installation(s) put back"),
            Err(e) => return Err(Error::msg(format!("Your Minecraft Launcher installations couldn't be put back: {e}"))),
        }
    }
    pc_core::settings::save(&state.ctx.paths, &s)?;
    apply_runtime_settings(&state, &s);
    *state.settings.write().unwrap() = s.clone();
    Ok(s)
}

#[tauri::command]
fn reset_settings(state: State<'_, AppState>) -> R<Settings> {
    let s = Settings::default();
    pc_core::settings::save(&state.ctx.paths, &s)?;
    apply_runtime_settings(&state, &s);
    *state.settings.write().unwrap() = s.clone();
    Ok(s)
}

#[tauri::command]
fn get_app_state(state: State<'_, AppState>) -> pc_core::state::AppState {
    state.app.lock().unwrap().clone()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePatch {
    onboarding_done: Option<bool>,
    onboarding_choice: Option<String>,
    selected_profile: Option<String>,
    last_page: Option<String>,
}

#[tauri::command]
fn set_app_state(state: State<'_, AppState>, patch: AppStatePatch) -> R<pc_core::state::AppState> {
    {
        let mut a = state.app.lock().unwrap();
        if let Some(v) = patch.onboarding_done {
            a.onboarding_done = v;
        }
        if let Some(v) = patch.onboarding_choice {
            a.onboarding_choice = Some(v);
        }
        if let Some(v) = patch.selected_profile {
            a.selected_profile = Some(v);
        }
        if let Some(v) = patch.last_page {
            a.last_page = Some(v);
        }
    }
    state.save_app_state()?;
    Ok(state.app.lock().unwrap().clone())
}

// ------------------------------------------------------------------ detection & official launcher

fn mc_dir(s: &AppState) -> PathBuf {
    s.settings().minecraft_dir.map(PathBuf::from).unwrap_or_else(pc_core::paths::default_minecraft_dir)
}

#[tauri::command]
async fn detect_installs(state: State<'_, AppState>) -> R<Vec<DetectedInstall>> {
    let root = mc_dir(&state);
    let extra: Vec<PathBuf> = state.settings().extra_scan_dirs.iter().map(PathBuf::from).collect();
    tokio::task::spawn_blocking(move || detect::detect_all(&root, &extra)).await.map_err(|e| Error::msg(e.to_string()))
}

#[tauri::command]
fn official_launcher(state: State<'_, AppState>) -> OfficialLauncher {
    integration::detect(&mc_dir(&state))
}

#[tauri::command]
fn open_official_launcher(state: State<'_, AppState>) -> R<String> {
    integration::open(&integration::detect(&mc_dir(&state)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReq {
    root: PathBuf,
    game_dir: PathBuf,
    version_id: Option<String>,
    mc_version: Option<String>,
    loader: Option<String>,
    name: String,
    /// "manage" = use the folder in place; "copy" = new isolated instance with a copy of
    /// mods/config/packs (worlds only if `include_worlds`)
    mode: String,
    include_worlds: Option<bool>,
}

#[tauri::command]
async fn import_install(state: State<'_, AppState>, req: ImportReq) -> R<Profile> {
    let ctx = &state.ctx;
    // Work out Minecraft version + loader.
    let mut mc = req.mc_version.clone();
    let mut loader = req.loader.as_deref().and_then(LoaderKind::parse).unwrap_or(LoaderKind::Vanilla);
    let mut loader_version = None;
    let mut external_version_id = None;
    if let Some(id) = req.version_id.as_ref().filter(|v| !v.starts_with("latest-")) {
        let json = req.root.join("versions").join(id).join(format!("{id}.json"));
        if let Ok(v) = util::read_json::<pc_core::minecraft::manifest::VersionJson>(&json) {
            let (l, lv) = pc_core::minecraft::manifest::detect_loader(&v);
            loader = LoaderKind::parse(l).unwrap_or(LoaderKind::Vanilla);
            loader_version = lv;
            mc = Some(v.inherits_from.clone().unwrap_or_else(|| id.clone()));
            if !loader.installable() {
                external_version_id = Some(id.clone());
            }
        } else {
            mc = Some(id.clone());
        }
    }
    let mc = match mc {
        Some(m) => m,
        None => install::manifest(ctx, false).await?.latest.release,
    };
    let p = profiles(&state);
    let manage = req.mode == "manage";
    let mut prof = p.create(NewProfile {
        name: req.name.clone(),
        kind: if manage { ProfileKind::External } else { ProfileKind::Modded },
        mc_version: mc,
        loader,
        loader_version,
        game_dir: if manage { Some(req.game_dir.clone()) } else { None },
        preset: None,
        // a copy is a normal Shadow Client profile: it gets the built-in mods (the mod menu) like any other
        builtin_mods: Some(!manage && matches!(loader, LoaderKind::Fabric | LoaderKind::Quilt)),
        ingame_features: None,
    })?;
    if manage {
        prof.external = true;
        prof.external_root = Some(req.root.clone());
        prof.external_version_id = external_version_id;
    } else {
        let to = p.game_dir(&prof);
        let worlds = req.include_worlds.unwrap_or(false);
        let from = req.game_dir.clone();
        let skip = move |path: &Path| {
            let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            let top = path.parent() == Some(from.as_path());
            top && (matches!(name.as_str(), "logs" | "crash-reports" | "versions" | "libraries" | "assets" | "runtime" | "webcache2" | "screenshots" | "quickplay" | "staging" | "systemconfig")
                || name.starts_with("launcher_")
                || name.contains("credentials")
                || name.ends_with(".log")
                || (!worlds && name == "saves"))
        };
        let src = req.game_dir.clone();
        tokio::task::spawn_blocking(move || util::copy_dir_recursive(&src, &to, &skip)).await.map_err(|e| Error::msg(e.to_string()))??;
    }
    p.save(&prof)?;
    Ok(prof)
}

#[tauri::command]
async fn register_with_official(state: State<'_, AppState>, profile_id: String) -> R<integration::RegisterReport> {
    register_official(&state, &profile_id).await
}

/// Add (or refresh) a profile in the official launcher, with its mods folder as the game directory.
async fn register_official(state: &AppState, profile_id: &str) -> R<integration::RegisterReport> {
    let p = profiles(state).get(profile_id)?;
    if !matches!(p.loader, LoaderKind::Fabric | LoaderKind::Quilt | LoaderKind::Vanilla) {
        return Err(Error::Incompatible("Only Vanilla, Fabric and Quilt profiles can be registered".into()));
    }
    let (version_id, json) = match p.loader {
        // Vanilla needs no version file: the official launcher downloads it itself.
        LoaderKind::Vanilla => (p.mc_version.clone(), None),
        kind => {
            let lv = match p.loader_version.clone() {
                Some(v) => v,
                None => loaders::recommended_loader(&state.ctx.http, &state.ctx.api.minecraft, kind, &p.mc_version).await?,
            };
            let v = loaders::profile_json(&state.ctx.http, &state.ctx.api.minecraft, kind, &p.mc_version, &lv).await?;
            // The built-in mods (Shadow Menu) are normally put in place when Shadow Client launches.
            let sc = play::scope_for(&state.ctx, &p);
            if let Err(e) = play::ensure_builtins(&state.ctx, &sc).await {
                tracing::warn!("built-in mods for the official launcher: {e}");
            }
            (v.id.clone(), Some(serde_json::to_value(&v)?))
        }
    };
    let game_dir = profiles(state).game_dir(&p);
    let only = state.settings().official_only_this;
    integration::register_profile(&mc_dir(state), &format!("{} (Shadow Client)", p.name), &version_id, json.as_ref(), &game_dir, None, only)
}

/// Open the official launcher with the given profile (and its mods) added as an installation.
/// Profiles it can't take (Forge, NeoForge) and a launcher that was never started still open it.
#[tauri::command]
async fn open_official_with_profile(state: State<'_, AppState>, profile_id: Option<String>) -> R<String> {
    let launcher = integration::detect(&mc_dir(&state));
    if !launcher.installed {
        return integration::open(&launcher);
    }
    let note = match profile_id {
        None => String::new(),
        Some(id) => match register_official(&state, &id).await {
            Ok(r) => {
                let name = profiles(&state).get(&id).map(|p| p.name).unwrap_or_default();
                if state.settings().official_only_this {
                    let aside = if r.hidden > 0 { format!(" Your other {} installation(s) are put aside; turn off \"Only show the profile you open\" in Settings to get them back.", r.hidden) } else { String::new() };
                    format!(" \"{name} (Shadow Client)\" is the only installation next to PLAY.{aside}")
                } else {
                    format!(" Pick \"{name} (Shadow Client)\" next to PLAY to play with your mods.")
                }
            }
            Err(Error::NotFound(_)) => " Start it once and close it, then click again to add your mods profile.".into(),
            Err(e) => format!(" Your profile wasn't added: {e}"),
        },
    };
    integration::open(&launcher).map(|m| format!("{m}{note}"))
}

// ------------------------------------------------------------------ versions

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    latest_release: String,
    latest_snapshot: String,
    versions: Vec<VersionSummary>,
}

#[tauri::command]
async fn list_versions(state: State<'_, AppState>, snapshots: bool, old: bool, refresh: Option<bool>) -> R<VersionList> {
    let m = install::manifest(&state.ctx, refresh.unwrap_or(false)).await?;
    Ok(VersionList { latest_release: m.latest.release.clone(), latest_snapshot: m.latest.snapshot.clone(), versions: install::summarize(&state.ctx, &m, snapshots, old) })
}

#[tauri::command]
async fn loader_game_versions(state: State<'_, AppState>, loader: String) -> R<Vec<loaders::GameVersionSupport>> {
    let k = LoaderKind::parse(&loader).ok_or_else(|| Error::Invalid("unknown loader".into()))?;
    loaders::supported_game_versions(&state.ctx.http, &state.ctx.api.minecraft, k).await
}

#[tauri::command]
async fn loader_versions(state: State<'_, AppState>, loader: String, mc: String) -> R<Vec<loaders::LoaderVersion>> {
    let k = LoaderKind::parse(&loader).ok_or_else(|| Error::Invalid("unknown loader".into()))?;
    loaders::loader_versions(&state.ctx.http, &state.ctx.api.minecraft, k, &mc).await
}

/// Newest release that has both a Fabric loader and a Sodium build.
#[tauri::command]
async fn recommended_performance_version(state: State<'_, AppState>) -> R<String> {
    let m = install::manifest(&state.ctx, false).await?;
    let fabric = loaders::supported_game_versions(&state.ctx.http, &state.ctx.api.minecraft, LoaderKind::Fabric).await?;
    for v in m.versions.iter().filter(|v| v.kind == "release").take(12) {
        if !fabric.iter().any(|f| f.version == v.id) {
            continue;
        }
        let vs = mr::project_versions(&state.ctx, "AANobbMI", &["fabric"], &[&v.id]).await?;
        if mr::pick_best(&vs, &v.id, &["fabric"]).is_some() {
            return Ok(v.id.clone());
        }
    }
    Ok(m.latest.release)
}

#[tauri::command]
async fn repair_profile(state: State<'_, AppState>, profile_id: String) -> R<String> {
    let p = profiles(&state).get(&profile_id)?;
    let rv = match (&p.external_version_id, &p.external_root) {
        (Some(id), Some(root)) => install::resolve_external(&state.ctx, root, id).await?,
        _ => install::resolve_version(&state.ctx, &p.mc_version, p.loader, p.loader_version.as_deref()).await?,
    };
    let extra: Vec<PathBuf> = p.external_root.iter().map(|r| r.join("libraries")).filter(|d| d.is_dir()).collect();
    let inst = install::install(&state.ctx, rv, &extra, true).await?;
    Ok(format!("All files for {} are present and verified.", inst.resolved.id))
}

// ------------------------------------------------------------------ profiles

#[tauri::command]
fn list_profiles(state: State<'_, AppState>) -> Vec<Profile> {
    profiles(&state).list()
}

#[tauri::command]
fn get_profile(state: State<'_, AppState>, id: String) -> R<Profile> {
    profiles(&state).get(&id)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResult {
    profile: Profile,
    report: SetupReport,
}

#[tauri::command]
async fn create_profile(state: State<'_, AppState>, req: NewProfile) -> R<CreateResult> {
    let settings = state.settings();
    let (profile, report) = setup::create_profile(&state.ctx, req, &settings).await?;
    {
        let mut a = state.app.lock().unwrap();
        a.selected_profile = Some(profile.id.clone());
    }
    state.save_app_state()?;
    Ok(CreateResult { profile, report })
}

/// Update editable profile fields (not the Minecraft version; see check/apply_version_change).
#[tauri::command]
fn update_profile(state: State<'_, AppState>, profile: Profile) -> R<Profile> {
    let p = profiles(&state);
    let mut cur = p.get(&profile.id)?;
    let total = pc_core::memory::total_ram_mb();
    pc_core::memory::validate(profile.memory_min_mb, profile.memory_max_mb, total)?;
    if let Some(a) = &profile.jvm_args {
        pc_core::jvmargs::validate_user_args(a)?;
    }
    if let Some(pr) = &profile.preset {
        if !["low", "balanced", "high", "custom"].contains(&pr.as_str()) {
            return Err(Error::Invalid("unknown preset".into()));
        }
    }
    cur.name = profile.name;
    cur.java_path = profile.java_path.filter(|s| !s.trim().is_empty());
    cur.memory_min_mb = profile.memory_min_mb;
    cur.memory_max_mb = profile.memory_max_mb;
    cur.jvm_args = profile.jvm_args.filter(|s| !s.trim().is_empty());
    cur.resolution_width = profile.resolution_width;
    cur.resolution_height = profile.resolution_height;
    cur.fullscreen = profile.fullscreen;
    cur.preset = profile.preset;
    // builtin_mods is not user-editable: built-in mods are always on.
    cur.icon = profile.icon;
    cur.notes = profile.notes;
    p.save(&cur)?;
    Ok(cur)
}

#[tauri::command]
fn rename_profile(state: State<'_, AppState>, id: String, name: String) -> R<Profile> {
    profiles(&state).rename(&id, &name)
}

#[tauri::command]
async fn duplicate_profile(state: State<'_, AppState>, id: String, name: String, include_worlds: bool) -> R<Profile> {
    let paths = state.ctx.paths.clone();
    let src_id = id.clone();
    let p = tokio::task::spawn_blocking(move || Profiles::new(&paths).duplicate(&id, &name, include_worlds)).await.map_err(|e| Error::msg(e.to_string()))??;
    // Carry over the mod index so sources and update checks keep working in the copy.
    let src_index = pc_core::mods::index::index_path(&state.ctx.paths.profile_data_dir(&src_id));
    if src_index.is_file() {
        let dest = pc_core::mods::index::index_path(&state.ctx.paths.profile_data_dir(&p.id));
        std::fs::create_dir_all(dest.parent().unwrap())?;
        std::fs::copy(src_index, dest)?;
    }
    Ok(p)
}

#[tauri::command]
async fn delete_profile(state: State<'_, AppState>, id: String) -> R<DeleteReport> {
    if state.games.is_running(&id) {
        return Err(Error::Invalid("Close the game before deleting this profile.".into()));
    }
    let paths = state.ctx.paths.clone();
    let id2 = id.clone();
    let report = tokio::task::spawn_blocking(move || Profiles::new(&paths).delete(&id2)).await.map_err(|e| Error::msg(e.to_string()))??;
    // Launcher bookkeeping (mod index, mod backups) is kept, parked under backups/.
    let data = state.ctx.paths.profile_data_dir(&id);
    if data.exists() {
        let dest = state.ctx.paths.backups_dir().join("deleted-profile-data").join(format!("{id}-{}", util::timestamp_slug()));
        let _ = tokio::task::spawn_blocking(move || pc_core::util::move_path(&data, &dest)).await;
    }
    {
        let mut a = state.app.lock().unwrap();
        if a.selected_profile.as_deref() == Some(id.as_str()) {
            a.selected_profile = None;
        }
    }
    state.save_app_state()?;
    Ok(report)
}

#[tauri::command]
async fn export_profile(state: State<'_, AppState>, id: String, dest: PathBuf, include_options: bool) -> R<PathBuf> {
    let ctx = state.ctx.clone();
    tokio::task::spawn_blocking(move || modpacks::export_mrpack(&ctx, &id, &dest, include_options)).await.map_err(|e| Error::msg(e.to_string()))?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionChangeCheck {
    ok: bool,
    unavailable: Vec<String>,
    unknown: Vec<String>,
    message: String,
}

/// Check whether every mod has a build for `mc`. Never changes anything.
#[tauri::command]
async fn check_version_change(state: State<'_, AppState>, id: String, mc: String) -> R<VersionChangeCheck> {
    let (p, sc) = scope(&state, &id)?;
    let mut mods = manage::scan_mods(&sc)?;
    let _ = manage::identify(&state.ctx, &sc, &mut mods).await;
    let loaders_ok = p.loader.accepted_mod_loaders();
    let mut unavailable = vec![];
    let mut unknown = vec![];
    for m in &mods {
        match m.project_id() {
            Some(pid) => {
                let vs = mr::project_versions(&state.ctx, &pid, &loaders_ok, &[&mc]).await.unwrap_or_default();
                if mr::pick_best(&vs, &mc, &loaders_ok).is_none() {
                    unavailable.push(m.display_name());
                }
            }
            None => unknown.push(m.display_name()),
        }
    }
    let ok = unavailable.is_empty() && unknown.is_empty();
    let message = if ok {
        format!("All {} mods have builds for Minecraft {mc}.", mods.len())
    } else {
        "Changing this profile's version would break mods. Create a new profile for the new version instead; this one stays as it is.".into()
    };
    Ok(VersionChangeCheck { ok, unavailable, unknown, message })
}

/// Change a profile's Minecraft version. Only allowed for profiles without mods (vanilla) or
/// when `check_version_change` passed; worlds are backed up first.
#[tauri::command]
async fn apply_version_change(state: State<'_, AppState>, id: String, mc: String) -> R<Profile> {
    let (mut p, sc) = scope(&state, &id)?;
    let mods = manage::scan_mods(&sc)?;
    if !mods.is_empty() {
        let check = check_version_change(state.clone(), id.clone(), mc.clone()).await?;
        if !check.ok {
            return Err(Error::Incompatible(check.message));
        }
        return Err(Error::Incompatible("Profiles with mods are updated by creating a new profile, so mods can be re-resolved safely. Use 'Create profile for new version'.".into()));
    }
    let saves = sc.game_dir.join("saves");
    if saves.is_dir() && std::fs::read_dir(&saves).map(|mut r| r.next().is_some()).unwrap_or(false) {
        let dest = sc.data_dir.join("backups").join(format!("worlds-before-{mc}-{}", util::timestamp_slug()));
        let s2 = saves.clone();
        tokio::task::spawn_blocking(move || util::copy_dir_recursive(&s2, &dest, &|_| false)).await.map_err(|e| Error::msg(e.to_string()))??;
    }
    p.mc_version = mc;
    if matches!(p.loader, LoaderKind::Fabric | LoaderKind::Quilt) {
        p.loader_version = None;
    }
    profiles(&state).save(&p)?;
    Ok(p)
}

#[tauri::command]
fn open_profile_folder(app: tauri::AppHandle, state: State<'_, AppState>, id: String, sub: Option<String>) -> R<()> {
    let p = profiles(&state).get(&id)?;
    let mut dir = profiles(&state).game_dir(&p);
    if let Some(s) = sub {
        if !["mods", "resourcepacks", "shaderpacks", "saves", "config", "screenshots", "logs", "crash-reports"].contains(&s.as_str()) {
            return Err(Error::Invalid("unknown folder".into()));
        }
        dir = dir.join(s);
    }
    std::fs::create_dir_all(&dir)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| Error::msg(e.to_string()))
}

// ------------------------------------------------------------------ accounts

#[tauri::command]
fn list_accounts(state: State<'_, AppState>) -> R<Vec<Account>> {
    auth::list_accounts(&state.ctx)
}

#[tauri::command]
fn add_offline_account(state: State<'_, AppState>, name: String) -> R<Account> {
    let a = auth::add_offline(&state.ctx, &name)?;
    state.app.lock().unwrap().active_account = Some(a.id.clone());
    state.save_app_state()?;
    Ok(a)
}

#[tauri::command]
fn remove_account(state: State<'_, AppState>, id: String) -> R<()> {
    auth::remove_account(&state.ctx, &id)?;
    let mut a = state.app.lock().unwrap();
    if a.active_account.as_deref() == Some(id.as_str()) {
        a.active_account = None;
    }
    drop(a);
    state.save_app_state()
}

#[tauri::command]
fn set_active_account(state: State<'_, AppState>, id: String) -> R<()> {
    if !auth::list_accounts(&state.ctx)?.iter().any(|a| a.id == id) {
        return Err(Error::NotFound("account".into()));
    }
    state.app.lock().unwrap().active_account = Some(id);
    state.save_app_state()
}

/// Start the Microsoft device-code sign-in. The result arrives as an `auth:result` event.
#[tauri::command]
async fn msa_start(app: tauri::AppHandle, state: State<'_, AppState>) -> R<auth::DeviceCode> {
    let client_id = auth::require_client_id(state.settings().msa_client_id.as_deref())?;
    let code = auth::start_device_login(&state.ctx, &client_id).await?;
    let cancel = CancellationToken::new();
    if let Some((_, _, old)) = state.login.lock().unwrap().replace((code.clone(), client_id.clone(), cancel.clone())) {
        old.cancel();
    }
    let ctx = state.ctx.clone();
    let code2 = code.clone();
    tauri::async_runtime::spawn(async move {
        let result = auth::finish_device_login(&ctx, &client_id, &code2, cancel).await;
        let payload = match result {
            Ok(acc) => {
                if let Some(st) = tauri::Manager::try_state::<AppState>(&app) {
                    st.app.lock().unwrap().active_account = Some(acc.id.clone());
                    let _ = st.save_app_state();
                    *st.login.lock().unwrap() = None;
                }
                serde_json::json!({"ok": true, "account": acc})
            }
            Err(Error::Cancelled) => serde_json::json!({"ok": false, "cancelled": true}),
            Err(e) => serde_json::json!({"ok": false, "error": e}),
        };
        let _ = app.emit("auth:result", payload);
    });
    Ok(code)
}

#[tauri::command]
fn msa_cancel(state: State<'_, AppState>) {
    if let Some((_, _, c)) = state.login.lock().unwrap().take() {
        c.cancel();
    }
}

#[tauri::command]
fn curseforge_status(state: State<'_, AppState>) -> bool {
    cf::has_key(&state.ctx)
}

#[tauri::command]
fn set_curseforge_key(state: State<'_, AppState>, key: String) -> R<()> {
    let k = key.trim();
    if k.len() < 20 || k.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(Error::Invalid("That does not look like a CurseForge API key".into()));
    }
    state.ctx.secrets.set(CURSEFORGE_KEY, k)
}

#[tauri::command]
fn clear_curseforge_key(state: State<'_, AppState>) -> R<()> {
    state.ctx.secrets.remove(CURSEFORGE_KEY)
}

// ------------------------------------------------------------------ launch

fn active_account(state: &AppState) -> Option<String> {
    state.app.lock().unwrap().active_account.clone()
}

#[tauri::command]
async fn preflight(state: State<'_, AppState>, profile_id: String) -> R<Preflight> {
    let settings = state.settings();
    let acc = active_account(&state);
    play::preflight(&state.ctx, &profile_id, &settings, acc.as_deref()).await
}

#[tauri::command]
async fn play(state: State<'_, AppState>, profile_id: String, force: bool) -> R<GameState> {
    let settings = state.settings();
    let acc = active_account(&state);
    {
        let mut a = state.app.lock().unwrap();
        a.selected_profile = Some(profile_id.clone());
    }
    let _ = state.save_app_state();
    play::play(&state.ctx, &state.games, &profile_id, &settings, acc.as_deref(), force).await
}

#[tauri::command]
fn running_games(state: State<'_, AppState>) -> Vec<RunningInfo> {
    state.games.list()
}

#[tauri::command]
fn kill_game(state: State<'_, AppState>, profile_id: String) -> bool {
    state.games.kill(&profile_id)
}

#[tauri::command]
fn game_log_tail(state: State<'_, AppState>, profile_id: String) -> Vec<String> {
    state.games.tail(&profile_id)
}

#[tauri::command]
async fn list_java(state: State<'_, AppState>) -> R<Vec<JavaInstall>> {
    let rt = state.ctx.paths.runtimes_dir();
    let extra = vec![mc_dir(&state)];
    tokio::task::spawn_blocking(move || java::discover(&rt, &extra)).await.map_err(|e| Error::msg(e.to_string()))
}

#[tauri::command]
async fn download_java(state: State<'_, AppState>, component: String) -> R<JavaInstall> {
    java::download_runtime(&state.ctx, &component).await
}

#[tauri::command]
fn memory_recommendation(state: State<'_, AppState>, profile_id: Option<String>) -> R<pc_core::memory::MemoryRecommendation> {
    let mods = match profile_id {
        Some(id) => {
            let (_, sc) = scope(&state, &id)?;
            manage::scan_mods(&sc).map(|m| m.iter().filter(|x| x.enabled).count()).unwrap_or(0)
        }
        None => 0,
    };
    Ok(pc_core::memory::recommend(pc_core::memory::total_ram_mb(), mods, true, false))
}

// ------------------------------------------------------------------ mods

#[tauri::command]
async fn mods_list(state: State<'_, AppState>, profile_id: String, identify: bool) -> R<Vec<InstalledMod>> {
    let (_, sc) = scope(&state, &profile_id)?;
    let sc2 = sc.clone();
    let mut mods = tokio::task::spawn_blocking(move || manage::scan_mods(&sc2)).await.map_err(|e| Error::msg(e.to_string()))??;
    if identify {
        if let Err(e) = manage::identify(&state.ctx, &sc, &mut mods).await {
            tracing::warn!("mod identification failed: {e}");
        }
    }
    Ok(mods)
}

#[tauri::command]
async fn mods_check_updates(state: State<'_, AppState>, profile_id: String) -> R<Vec<InstalledMod>> {
    let (_, sc) = scope(&state, &profile_id)?;
    let mut mods = manage::scan_mods(&sc)?;
    manage::identify(&state.ctx, &sc, &mut mods).await?;
    manage::check_updates(&state.ctx, &sc, &mut mods).await?;
    Ok(mods)
}

#[tauri::command]
async fn modrinth_search(state: State<'_, AppState>, query: mr::SearchQuery) -> R<mr::SearchResult> {
    mr::search(&state.ctx, &query).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetails {
    project: mr::Project,
    authors: Vec<String>,
    url: String,
}

#[tauri::command]
async fn modrinth_project(state: State<'_, AppState>, id: String) -> R<ProjectDetails> {
    let project = mr::project(&state.ctx, &id).await?;
    let authors = mr::members(&state.ctx, &project.id).await.map(|m| m.into_iter().map(|t| t.user.username).collect()).unwrap_or_default();
    let url = mr::project_url(&project.project_type, project.slug.as_deref().unwrap_or(&project.id));
    let mut a = state.app.lock().unwrap();
    a.push_recent(pc_core::state::RecentProject {
        platform: "modrinth".into(),
        id: project.id.clone(),
        slug: project.slug.clone(),
        title: project.title.clone(),
        icon_url: project.icon_url.clone(),
        project_type: Some(project.project_type.clone()),
        viewed_at: String::new(),
    });
    drop(a);
    let _ = state.save_app_state();
    Ok(ProjectDetails { project, authors, url })
}

#[tauri::command]
async fn modrinth_versions(state: State<'_, AppState>, id: String, profile_id: Option<String>) -> R<Vec<mr::Version>> {
    match profile_id {
        Some(pid) => {
            let (p, _) = scope(&state, &pid)?;
            let loaders = p.loader.accepted_mod_loaders();
            mr::project_versions(&state.ctx, &id, &loaders, &[&p.mc_version]).await
        }
        None => mr::project_versions(&state.ctx, &id, &[], &[]).await,
    }
}

#[tauri::command]
async fn curseforge_search(state: State<'_, AppState>, query: cf::CfSearchQuery) -> R<cf::CfSearchResult> {
    cf::search(&state.ctx, &query).await
}

#[tauri::command]
async fn curseforge_mod(state: State<'_, AppState>, id: u64) -> R<cf::CfMod> {
    let m = cf::get_mod(&state.ctx, id).await?;
    let mut a = state.app.lock().unwrap();
    a.push_recent(pc_core::state::RecentProject {
        platform: "curseforge".into(),
        id: m.id.to_string(),
        slug: Some(m.slug.clone()),
        title: m.name.clone(),
        icon_url: m.logo.as_ref().and_then(|l| l.thumbnail_url.clone()),
        project_type: Some("mod".into()),
        viewed_at: String::new(),
    });
    drop(a);
    let _ = state.save_app_state();
    Ok(m)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanResult {
    plan_id: String,
    plan: InstallPlan,
}

fn store_plan(state: &AppState, profile_id: &str, plan: InstallPlan) -> PlanResult {
    let id = uuid::Uuid::new_v4().to_string();
    let mut plans = state.plans.lock().unwrap();
    if plans.len() > 20 {
        plans.clear();
    }
    plans.insert(id.clone(), PendingPlan { profile_id: profile_id.into(), plan: plan.clone() });
    PlanResult { plan_id: id, plan }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanReq {
    profile_id: String,
    /// "modrinth" | "curseforge"
    platform: String,
    project_id: String,
    version_id: Option<String>,
    kind: ContentKind,
}

#[tauri::command]
async fn plan_install(state: State<'_, AppState>, req: PlanReq) -> R<PlanResult> {
    let (p, sc) = scope(&state, &req.profile_id)?;
    let mods = manage::scan_mods(&sc)?;
    let installed = manage::installed_set(&mods);
    let mut plan = match req.platform.as_str() {
        "modrinth" => resolver::resolve_modrinth(&state.ctx, &sc.target, req.kind, &req.project_id, req.version_id.as_deref(), &installed, req.version_id.is_some()).await?,
        "curseforge" => {
            let mid: u64 = req.project_id.parse().map_err(|_| Error::Invalid("bad CurseForge id".into()))?;
            let fid = req.version_id.as_deref().map(|v| v.parse::<u64>()).transpose().map_err(|_| Error::Invalid("bad file id".into()))?;
            resolver::resolve_curseforge(&state.ctx, &sc.target, req.kind, mid, fid, &installed).await?
        }
        _ => return Err(Error::Invalid("unknown platform".into())),
    };
    if req.kind == ContentKind::Shader && !plan.blocked {
        // Shaders on Fabric/Quilt need Iris (which brings Sodium). Never enabled automatically.
        let iris = pc_core::mods::compat::DB.shader_support.iris_project_id.clone();
        if p.loader.accepted_mod_loaders().is_empty() {
            plan.warnings.push("Shader packs need Iris, which requires a Fabric or Quilt profile.".into());
        } else if !installed.project_ids.contains(&iris) && !installed.mod_ids.contains("iris") {
            let ip = resolver::resolve_modrinth(&state.ctx, &sc.target, ContentKind::Mod, &iris, None, &installed, false).await?;
            if ip.blocked {
                plan.warnings.push(format!("Iris is not available for this version ({}); the shader pack can be installed but will not load.", ip.blocked_reason.clone().unwrap_or_default()));
            } else {
                plan.warnings.push("Iris (shader loader) will be installed too. Shaders cost a lot of GPU time; on integrated graphics expect large FPS drops. Shaders are not turned on automatically.".into());
                resolver::merge_plans(&mut plan, ip);
            }
        }
    }
    Ok(store_plan(&state, &req.profile_id, plan))
}

/// Plan an update of one installed mod to the newest compatible version.
#[tauri::command]
async fn plan_update(state: State<'_, AppState>, profile_id: String, file_name: String) -> R<PlanResult> {
    let (_, sc) = scope(&state, &profile_id)?;
    let mut mods = manage::scan_mods(&sc)?;
    manage::identify(&state.ctx, &sc, &mut mods).await?;
    manage::check_updates(&state.ctx, &sc, &mut mods).await?;
    let m = mods.iter().find(|m| m.file_name == file_name).ok_or_else(|| Error::NotFound(file_name.clone()))?;
    let up = m.update.clone().ok_or_else(|| Error::Invalid(format!("{} is up to date", m.display_name())))?;
    let pid = m.project_id().ok_or_else(|| Error::Invalid("unknown project".into()))?;
    let installed = manage::installed_set(&mods);
    let plan = resolver::resolve_modrinth(&state.ctx, &sc.target, ContentKind::Mod, &pid, Some(&up.version_id), &installed, true).await?;
    Ok(store_plan(&state, &profile_id, plan))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecReq {
    plan_id: String,
    /// Optional dependencies the user ticked (Modrinth project ids / CurseForge mod ids)
    optional: Vec<String>,
}

#[tauri::command]
async fn execute_plan(state: State<'_, AppState>, req: ExecReq) -> R<manage::InstallReport> {
    let pending = state.plans.lock().unwrap().remove(&req.plan_id).ok_or_else(|| Error::NotFound("This install plan expired; please try again.".into()))?;
    let (_, sc) = scope(&state, &pending.profile_id)?;
    let mut plan = pending.plan;
    if !req.optional.is_empty() {
        let installed = manage::installed_set(&manage::scan_mods(&sc)?);
        // Only optional dependencies that the plan actually offered can be added.
        let chosen: Vec<(String, String)> = plan.optional.iter().filter(|x| req.optional.contains(&x.project_id)).map(|x| (x.project_id.clone(), x.platform.clone())).collect();
        for (opt, platform) in &chosen {
            let extra = if platform == "curseforge" {
                resolver::resolve_curseforge(&state.ctx, &sc.target, ContentKind::Mod, opt.parse().unwrap_or(0), None, &installed).await?
            } else {
                resolver::resolve_modrinth(&state.ctx, &sc.target, ContentKind::Mod, opt, None, &installed, false).await?
            };
            if extra.blocked {
                plan.warnings.push(format!("Optional {} skipped: {}", opt, extra.blocked_reason.clone().unwrap_or_default()));
                continue;
            }
            resolver::merge_plans(&mut plan, extra);
        }
        plan.finalize(&installed);
    }
    let r = manage::execute(&state.ctx, &sc, &plan, builtin::embedded).await?;
    pc_core::events::notify(&state.ctx.sink, "success", "Installed", if r.installed.is_empty() { "Nothing new to install".to_string() } else { r.installed.join(", ") });
    Ok(r)
}

#[tauri::command]
fn discard_plan(state: State<'_, AppState>, plan_id: String) {
    state.plans.lock().unwrap().remove(&plan_id);
}

#[tauri::command]
fn mod_set_enabled(state: State<'_, AppState>, profile_id: String, file_name: String, enabled: bool, kind: Option<ContentKind>) -> R<String> {
    let (_, sc) = scope(&state, &profile_id)?;
    manage::set_enabled(&sc, kind.unwrap_or(ContentKind::Mod), &file_name, enabled)
}

#[tauri::command]
fn mod_remove(state: State<'_, AppState>, profile_id: String, file_name: String, kind: Option<ContentKind>) -> R<manage::RemoveReport> {
    let (_, sc) = scope(&state, &profile_id)?;
    manage::remove(&sc, kind.unwrap_or(ContentKind::Mod), &file_name)
}

#[tauri::command]
fn content_remove(state: State<'_, AppState>, profile_id: String, file_name: String, kind: ContentKind) -> R<manage::RemoveReport> {
    let (_, sc) = scope(&state, &profile_id)?;
    // Folder-style packs are moved as a whole into backups.
    let path = sc.folder(kind).join(&file_name);
    if path.is_dir() {
        let segs = pc_core::safepath::validate_relative(&file_name)?;
        if segs.len() != 1 {
            return Err(Error::UnsafePath(file_name));
        }
        let backup = sc.backups(kind).join(util::timestamp_slug()).join(&file_name);
        util::move_path(&path, &backup)?;
        return Ok(manage::RemoveReport { backup, dependents: vec![] });
    }
    manage::remove(&sc, kind, &file_name)
}

#[tauri::command]
fn mod_backups(state: State<'_, AppState>, profile_id: String, kind: Option<ContentKind>) -> R<Vec<manage::BackupEntry>> {
    let (_, sc) = scope(&state, &profile_id)?;
    Ok(manage::list_backups(&sc, kind.unwrap_or(ContentKind::Mod)))
}

#[tauri::command]
fn mod_restore(state: State<'_, AppState>, profile_id: String, path: PathBuf, kind: Option<ContentKind>) -> R<String> {
    let (_, sc) = scope(&state, &profile_id)?;
    manage::restore(&sc, kind.unwrap_or(ContentKind::Mod), &path)
}

#[tauri::command]
fn record_recent(state: State<'_, AppState>, item: pc_core::state::RecentProject) -> R<()> {
    state.app.lock().unwrap().push_recent(item);
    state.save_app_state()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinStatus {
    #[serde(flatten)]
    def: builtin::BuiltinMod,
    installed_file: Option<String>,
    enabled: bool,
    pinned_for_profile: bool,
}

#[tauri::command]
fn builtin_mods(state: State<'_, AppState>, profile_id: Option<String>) -> R<Vec<BuiltinStatus>> {
    let mods = match &profile_id {
        Some(id) => {
            let (_, sc) = scope(&state, id)?;
            manage::scan_mods(&sc).unwrap_or_default()
        }
        None => vec![],
    };
    let target = match &profile_id {
        Some(id) => Some(scope(&state, id)?.1.target),
        None => None,
    };
    Ok(builtin::list()
        .iter()
        .map(|b| {
            let inst = mods.iter().find(|m| m.meta.as_ref().is_some_and(|x| x.mod_id == b.mod_id));
            BuiltinStatus {
                def: b.clone(),
                installed_file: inst.map(|m| m.file_name.clone()),
                enabled: inst.is_some_and(|m| m.enabled),
                pinned_for_profile: target.as_ref().is_some_and(|t| b.pinned.iter().any(|p| p.game_version == t.mc_version)),
            }
        })
        .collect())
}

#[tauri::command]
async fn install_builtins(state: State<'_, AppState>, profile_id: String) -> R<SetupReport> {
    let (mut p, sc) = scope(&state, &profile_id)?;
    let installed = manage::installed_set(&manage::scan_mods(&sc)?);
    let (plan, notes) = builtin::plan(&state.ctx, &sc.target, &installed).await?;
    let mut report = SetupReport { notes, ..Default::default() };
    if plan.blocked {
        return Err(Error::Incompatible(plan.blocked_reason.unwrap_or_default()));
    }
    if !plan.files.is_empty() {
        let r = manage::execute(&state.ctx, &sc, &plan, builtin::embedded).await?;
        report.installed = r.installed;
        report.notes.extend(r.warnings);
    }
    p.builtin_mods = true;
    profiles(&state).save(&p)?;
    Ok(report)
}

#[tauri::command]
async fn performance_mods_status(state: State<'_, AppState>, profile_id: String) -> R<Vec<PerfModStatus>> {
    let (p, sc) = scope(&state, &profile_id)?;
    let preset = p.preset.clone().unwrap_or_else(|| "balanced".into());
    setup::performance_status(&state.ctx, &sc, &preset).await
}

#[tauri::command]
async fn install_performance_mods(state: State<'_, AppState>, profile_id: String, slugs: Vec<String>) -> R<PlanResult> {
    let (_, sc) = scope(&state, &profile_id)?;
    let installed = manage::installed_set(&manage::scan_mods(&sc)?);
    let mut report = SetupReport::default();
    let mut plan = setup::plan_performance_mods(&state.ctx, &sc.target, &slugs, &installed, &mut report).await?;
    plan.warnings.extend(report.notes);
    Ok(store_plan(&state, &profile_id, plan))
}

// ------------------------------------------------------------------ resource packs & shaders

#[tauri::command]
fn resourcepacks_list(state: State<'_, AppState>, profile_id: String) -> R<Vec<resources::PackInfo>> {
    let (_, sc) = scope(&state, &profile_id)?;
    Ok(resources::list_resource_packs(&sc.game_dir))
}

#[tauri::command]
fn resourcepack_set_active(state: State<'_, AppState>, profile_id: String, name: String, active: bool) -> R<()> {
    if state.games.is_running(&profile_id) {
        return Err(Error::Invalid("Close the game first: Minecraft rewrites options.txt when it exits.".into()));
    }
    let (_, sc) = scope(&state, &profile_id)?;
    resources::set_resource_pack_active(&sc.game_dir, &sc.data_dir, &name, active)
}

#[tauri::command]
fn shader_state(state: State<'_, AppState>, profile_id: String) -> R<resources::ShaderState> {
    let (_, sc) = scope(&state, &profile_id)?;
    Ok(resources::shader_state(&sc.game_dir))
}

#[tauri::command]
fn shader_set(state: State<'_, AppState>, profile_id: String, name: Option<String>) -> R<()> {
    let (_, sc) = scope(&state, &profile_id)?;
    resources::set_shader(&sc.game_dir, &sc.data_dir, name.as_deref())
}

// ------------------------------------------------------------------ performance

#[tauri::command]
async fn hardware_info(state: State<'_, AppState>, refresh: bool) -> R<HardwareInfo> {
    if !refresh {
        if let Some(h) = state.hardware.lock().unwrap().clone() {
            return Ok(h);
        }
    }
    let root = state.ctx.paths.root.clone();
    let h = tokio::task::spawn_blocking(move || hardware::detect(&root)).await.map_err(|e| Error::msg(e.to_string()))?;
    *state.hardware.lock().unwrap() = Some(h.clone());
    Ok(h)
}

/// Runs only when the user asks (never automatically).
#[tauri::command]
async fn run_benchmark(state: State<'_, AppState>) -> R<BenchmarkResult> {
    let root = state.ctx.paths.root.clone();
    let tmp = state.ctx.paths.tmp_dir();
    let r = tokio::task::spawn_blocking(move || benchmark::run(hardware::detect(&root), &tmp)).await.map_err(|e| Error::msg(e.to_string()))?;
    *state.hardware.lock().unwrap() = Some(r.hardware.clone());
    state.app.lock().unwrap().benchmark = Some(r.clone());
    state.save_app_state()?;
    Ok(r)
}

#[tauri::command]
fn preset_list() -> Vec<presets::PresetInfo> {
    presets::PRESETS.to_vec()
}

fn graphics_for(state: &AppState, preset: &str) -> GraphicsSettings {
    if preset == "custom" {
        state.settings().custom_graphics
    } else {
        let hw = state.hardware.lock().unwrap().clone();
        presets::preset_graphics(preset, hw.as_ref())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetPreview {
    graphics: GraphicsSettings,
    changes: Vec<(String, String, String)>,
    options_exists: bool,
    warnings: Vec<String>,
}

#[tauri::command]
fn preset_preview(state: State<'_, AppState>, profile_id: String, preset: String) -> R<PresetPreview> {
    let (p, sc) = scope(&state, &profile_id)?;
    let g = graphics_for(&state, &preset);
    let existing = options::OptionsFile::load(&sc.game_dir.join("options.txt"))?;
    let changes = options::graphics_changes(&g, &p.mc_version, existing.as_ref())
        .into_iter()
        .map(|(k, v)| {
            let old = existing.as_ref().and_then(|e| e.get(&k).map(String::from)).unwrap_or_default();
            (k, old, v)
        })
        .filter(|(_, o, n)| o != n)
        .collect();
    let hw = state.hardware.lock().unwrap().clone();
    Ok(PresetPreview { warnings: presets::graphics_warnings(&g, hw.as_ref()), graphics: g, changes, options_exists: existing.is_some() })
}

#[tauri::command]
async fn apply_preset(state: State<'_, AppState>, profile_id: String, preset: String) -> R<options::ApplyReport> {
    if state.games.is_running(&profile_id) {
        return Err(Error::Invalid("Close the game first: Minecraft rewrites options.txt when it exits.".into()));
    }
    let (mut p, sc) = scope(&state, &profile_id)?;
    let g = graphics_for(&state, &preset);
    let jar = state.ctx.paths.version_jar(&p.mc_version);
    let dv = options::data_version_from_jar(&jar);
    let r = if !sc.game_dir.join("options.txt").exists() && dv.is_none() {
        // Game never started yet: remember the preset; PLAY writes it with the right data version.
        options::ApplyReport { changed: vec![], backup: None, created: false }
    } else {
        options::apply_graphics(&sc.game_dir, &sc.data_dir, &g, &p.mc_version, dv)?
    };
    p.preset = Some(preset);
    profiles(&state).save(&p)?;
    Ok(r)
}

#[tauri::command]
fn graphics_warnings(state: State<'_, AppState>, graphics: GraphicsSettings) -> R<Vec<String>> {
    presets::validate_graphics(&graphics)?;
    let hw = state.hardware.lock().unwrap().clone();
    Ok(presets::graphics_warnings(&graphics, hw.as_ref()))
}

#[tauri::command]
fn current_graphics(state: State<'_, AppState>, profile_id: String) -> R<Option<serde_json::Value>> {
    let (_, sc) = scope(&state, &profile_id)?;
    options::read_graphics(&sc.game_dir)
}

#[tauri::command]
fn options_backups(state: State<'_, AppState>, profile_id: String) -> R<Vec<options::OptionsBackup>> {
    let (_, sc) = scope(&state, &profile_id)?;
    Ok(options::list_backups(&sc.data_dir))
}

#[tauri::command]
fn options_restore(state: State<'_, AppState>, profile_id: String, name: String) -> R<()> {
    if state.games.is_running(&profile_id) {
        return Err(Error::Invalid("Close the game first.".into()));
    }
    let (_, sc) = scope(&state, &profile_id)?;
    options::restore(&sc.game_dir, &sc.data_dir, &name)
}

// ------------------------------------------------------------------ modpacks

#[tauri::command]
fn modpack_inspect(path: PathBuf) -> R<PackSummary> {
    modpacks::inspect(&path)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackResult {
    profile: Profile,
    report: PackInstallReport,
}

#[tauri::command]
async fn modpack_install_file(state: State<'_, AppState>, path: PathBuf, name: Option<String>) -> R<PackResult> {
    let (profile, report) = modpacks::install(&state.ctx, &path, name).await?;
    // Restore Shadow Client settings embedded by our own exports.
    if let Some(saved) = modpacks::embedded_profile_settings(&path) {
        let mut p = profile.clone();
        p.memory_min_mb = saved.memory_min_mb;
        p.memory_max_mb = saved.memory_max_mb;
        p.jvm_args = saved.jvm_args;
        p.preset = saved.preset;
        p.kind = saved.kind;
        let _ = profiles(&state).save(&p);
        return Ok(PackResult { profile: p, report });
    }
    Ok(PackResult { profile, report })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePackReq {
    platform: String,
    project_id: String,
    version_id: Option<String>,
}

#[tauri::command]
async fn modpack_install_remote(state: State<'_, AppState>, req: RemotePackReq) -> R<PackResult> {
    let ctx = &state.ctx;
    let tmp = ctx.paths.tmp_dir().join(format!("pack-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&tmp)?;
    let file = match req.platform.as_str() {
        "modrinth" => {
            let v = match &req.version_id {
                Some(v) => mr::version(ctx, v).await?,
                None => {
                    let vs = mr::project_versions(ctx, &req.project_id, &[], &[]).await?;
                    vs.iter().find(|v| v.version_type == "release").or(vs.first()).cloned().ok_or_else(|| Error::NotFound("no pack versions".into()))?
                }
            };
            let f = v.primary_file().ok_or_else(|| Error::NotFound("pack file".into()))?;
            if !f.filename.ends_with(".mrpack") {
                return Err(Error::Invalid("This version is not a .mrpack file".into()));
            }
            let dest = tmp.join("pack.mrpack");
            ctx.downloads
                .run(&format!("Modpack file {}", f.filename), vec![pc_core::download::DownloadItem::new(f.url.clone(), dest.clone()).sha1(f.hashes.sha1.clone()).sha512(f.hashes.sha512.clone()).size(f.size)])
                .await?;
            dest
        }
        "curseforge" => {
            let mid: u64 = req.project_id.parse().map_err(|_| Error::Invalid("bad id".into()))?;
            let m = cf::get_mod(ctx, mid).await?;
            let file = match &req.version_id {
                Some(f) => cf::get_files(ctx, &[f.parse().map_err(|_| Error::Invalid("bad file id".into()))?]).await?.into_iter().next(),
                None => {
                    let files = cf::mod_files(ctx, mid, None, None).await?;
                    let mut files = files;
                    files.sort_by(|a, b| a.release_type.cmp(&b.release_type).then(b.file_date.cmp(&a.file_date)));
                    files.into_iter().next()
                }
            }
            .ok_or_else(|| Error::NotFound("pack file".into()))?;
            let url = file.download_url.clone().ok_or_else(|| {
                Error::Incompatible(format!("{}'s author does not allow downloads through third-party apps. Download the pack from {} and import the zip with 'Import file'.", m.name, cf::project_url(&m)))
            })?;
            let dest = tmp.join("pack.zip");
            ctx.downloads.run(&format!("Modpack file {}", file.file_name), vec![pc_core::download::DownloadItem::new(url, dest.clone()).sha1(file.sha1()).size(file.file_length)]).await?;
            dest
        }
        _ => return Err(Error::Invalid("unknown platform".into())),
    };
    let result = modpacks::install(ctx, &file, None).await;
    let _ = std::fs::remove_dir_all(&tmp);
    let (profile, report) = result?;
    Ok(PackResult { profile, report })
}

#[tauri::command]
async fn modpack_update_file(state: State<'_, AppState>, profile_id: String, path: PathBuf) -> R<PackInstallReport> {
    if state.games.is_running(&profile_id) {
        return Err(Error::Invalid("Close the game first.".into()));
    }
    modpacks::update(&state.ctx, &profile_id, &path).await
}

// ------------------------------------------------------------------ downloads

#[tauri::command]
fn downloads_list(state: State<'_, AppState>) -> Vec<JobInfo> {
    state.ctx.downloads.list()
}

#[tauri::command]
fn download_cancel(state: State<'_, AppState>, id: String) -> bool {
    state.ctx.downloads.cancel(&id)
}

#[tauri::command]
async fn download_retry(state: State<'_, AppState>, id: String) -> R<JobInfo> {
    state.ctx.downloads.retry(&id).await
}

#[tauri::command]
fn downloads_clear(state: State<'_, AppState>) {
    state.ctx.downloads.clear_finished()
}

// ------------------------------------------------------------------ diagnostics & files

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFile {
    name: String,
    path: PathBuf,
    size: u64,
    modified: Option<String>,
    kind: String,
}

#[tauri::command]
fn logs_list(state: State<'_, AppState>) -> Vec<LogFile> {
    let mut out = vec![];
    for (dir, kind) in [(state.ctx.paths.logs_dir(), "launcher"), (state.ctx.paths.logs_dir().join("game"), "game")] {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if !e.path().is_file() {
                continue;
            }
            let m = e.metadata().ok();
            out.push(LogFile {
                name: e.file_name().to_string_lossy().into_owned(),
                path: e.path(),
                size: m.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: m.and_then(|m| m.modified().ok()).map(|t| {
                    let d: std::time::Duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    d.as_secs().to_string()
                }),
                kind: kind.into(),
            });
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

/// Read a log (last 1 MB), always redacted. Only files inside the logs folder.
#[tauri::command]
fn log_read(state: State<'_, AppState>, path: PathBuf, for_sharing: bool) -> R<String> {
    let logs = pc_core::paths::canonical_or_self(&state.ctx.paths.logs_dir());
    let real = pc_core::paths::canonical_or_self(&path);
    if !real.starts_with(&logs) {
        return Err(Error::UnsafePath("only launcher logs can be read here".into()));
    }
    let bytes = std::fs::read(&real)?;
    let start = bytes.len().saturating_sub(1024 * 1024);
    let text = String::from_utf8_lossy(&bytes[start..]).into_owned();
    Ok(if for_sharing { redact::redact_for_sharing(&text) } else { redact::redact(&text) })
}

#[tauri::command]
async fn diagnostics_report(state: State<'_, AppState>) -> R<String> {
    let hw = hardware_info(state.clone(), false).await.ok();
    let settings = state.settings();
    let mut s = format!("Shadow Client {}\nPortable: {}\n", pc_core::VERSION, state.ctx.paths.portable);
    if let Some(h) = hw {
        s += &format!(
            "OS: {} \nCPU: {} ({} threads)\nRAM: {} MB total, {} MB free\nGPU: {}\nDisplay: {}\n",
            h.os_name.unwrap_or_default(),
            h.cpu_name,
            h.cpu_logical_cores,
            h.ram_total_mb,
            h.ram_available_mb,
            h.gpus.iter().map(|g| format!("{} (driver {})", g.name, g.driver_version.clone().unwrap_or_default())).collect::<Vec<_>>().join("; "),
            h.display.map(|d| format!("{}x{} @{:?}Hz", d.width, d.height, d.refresh_hz)).unwrap_or_default()
        );
    }
    s += &format!(
        "Settings: preset={} jvmPreset={} memory={:?}/{:?} concurrency={} debug={}\n",
        settings.default_preset, settings.jvm_preset, settings.memory_min_mb, settings.memory_max_mb, settings.download_concurrency, settings.debug_logging
    );
    for p in profiles(&state).list() {
        s += &format!("Profile: {} | {} {} {:?} | last: {:?}\n", p.name, p.mc_version, p.loader.as_str(), p.loader_version, p.last_launch.as_ref().map(|l| (&l.result, &l.message)));
    }
    let javas = list_java(state.clone()).await.unwrap_or_default();
    for j in javas {
        s += &format!("Java: {} ({}) {}\n", j.version, j.source, j.path.display());
    }
    if let Some(latest) = logs_list(state.clone()).into_iter().find(|l| l.kind == "launcher") {
        let text = std::fs::read_to_string(&latest.path).unwrap_or_default();
        let tail: Vec<&str> = text.lines().rev().take(150).collect();
        s += "\n--- launcher log (last 150 lines) ---\n";
        s += &tail.into_iter().rev().collect::<Vec<_>>().join("\n");
    }
    Ok(redact::redact_for_sharing(&s))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    http_cache_bytes: u64,
    tmp_bytes: u64,
}

#[tauri::command]
fn cache_info(state: State<'_, AppState>) -> CacheInfo {
    CacheInfo { http_cache_bytes: state.ctx.http.cache_size(), tmp_bytes: util::dir_size(&state.ctx.paths.tmp_dir()) }
}

#[tauri::command]
fn clear_cache(state: State<'_, AppState>) -> R<u64> {
    let mut freed = state.ctx.http.clear_cache()?;
    // Temp files of finished operations (never game files).
    if state.games.list().is_empty() {
        freed += util::dir_size(&state.ctx.paths.tmp_dir());
        for e in std::fs::read_dir(state.ctx.paths.tmp_dir()).into_iter().flatten().flatten() {
            let _ = if e.path().is_dir() { std::fs::remove_dir_all(e.path()) } else { std::fs::remove_file(e.path()) };
        }
    }
    Ok(freed)
}

fn path_allowed(state: &AppState, p: &Path) -> bool {
    let real = pc_core::paths::canonical_or_self(p);
    let mut roots = vec![state.ctx.paths.root.clone(), mc_dir(state)];
    for prof in profiles(state).list() {
        roots.push(profiles(state).game_dir(&prof));
    }
    roots.iter().any(|r| real.starts_with(pc_core::paths::canonical_or_self(r)))
}

#[tauri::command]
fn open_path(app: tauri::AppHandle, state: State<'_, AppState>, path: PathBuf, reveal: bool) -> R<()> {
    if !path_allowed(&state, &path) {
        return Err(Error::UnsafePath("Only launcher and game folders can be opened from here".into()));
    }
    if reveal {
        app.opener().reveal_item_in_dir(&path).map_err(|e| Error::msg(e.to_string()))
    } else {
        app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| Error::msg(e.to_string()))
    }
}

#[tauri::command]
fn open_url(app: tauri::AppHandle, url: String) -> R<()> {
    let u = url::Url::parse(&url).map_err(|_| Error::Invalid("bad URL".into()))?;
    if u.scheme() != "https" {
        return Err(Error::Invalid("Only https links can be opened".into()));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| Error::msg(e.to_string()))
}

#[tauri::command]
fn open_data_dir(app: tauri::AppHandle, state: State<'_, AppState>) -> R<()> {
    app.opener().open_path(state.ctx.paths.root.to_string_lossy(), None::<&str>).map_err(|e| Error::msg(e.to_string()))
}

// ------------------------------------------------------------------ skins

#[tauri::command]
fn skins_list(state: State<'_, AppState>) -> Vec<skins::SkinWithData> {
    skins::list(&state.ctx)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinSaveReq {
    id: Option<String>,
    name: String,
    model: String,
    data_url: String,
    source: Option<String>,
}

#[tauri::command]
fn skin_save(state: State<'_, AppState>, req: SkinSaveReq) -> R<skins::SkinEntry> {
    let png = skins::decode_data_url(&req.data_url)?;
    let source = req.source.unwrap_or_else(|| "created".into());
    if !(source == "created" || source == "file" || source.starts_with("player:")) {
        return Err(Error::Invalid("unknown skin source".into()));
    }
    skins::save(&state.ctx, req.id.as_deref(), &req.name, &req.model, &png, &source)
}

#[tauri::command]
fn skin_delete(state: State<'_, AppState>, id: String) -> R<()> {
    skins::delete(&state.ctx, &id)
}

#[tauri::command]
fn skin_export(state: State<'_, AppState>, id: String, dest: PathBuf) -> R<()> {
    skins::export(&state.ctx, &id, &dest)
}

#[tauri::command]
fn skin_read_file(path: PathBuf) -> R<String> {
    skins::read_png_file(&path)
}

#[tauri::command]
async fn skin_fetch_player(state: State<'_, AppState>, name: String) -> R<skins::PlayerSkin> {
    skins::fetch_player_skin(&state.ctx, &name).await
}

/// Upload to the active Microsoft account (changes the skin everyone sees online).
#[tauri::command]
async fn skin_upload(state: State<'_, AppState>, id: String) -> R<String> {
    let acc = active_account(&state).ok_or_else(|| Error::Auth("Choose a Microsoft account first.".into()))?;
    skins::upload_to_account(&state.ctx, &acc, &id).await?;
    Ok("Skin uploaded to your Minecraft account. It can take a minute to appear in game.".into())
}

#[tauri::command]
fn skin_apply_local(state: State<'_, AppState>, id: String, profile_id: String) -> R<String> {
    if state.games.is_running(&profile_id) {
        return Err(Error::Invalid("Close the game first: Minecraft rewrites options.txt when it exits.".into()));
    }
    let (p, sc) = scope(&state, &profile_id)?;
    skins::apply_locally(&state.ctx, &id, &sc.game_dir, &sc.data_dir, &p.mc_version)
}

// ------------------------------------------------------------------ in-game features

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturesView {
    features: Vec<pc_core::features::FeatureStatus>,
    suite_provides: Vec<String>,
    pack: Vec<String>,
}

#[tauri::command]
async fn features_status(state: State<'_, AppState>, profile_id: String) -> R<FeaturesView> {
    let (_, sc) = scope(&state, &profile_id)?;
    Ok(FeaturesView {
        features: pc_core::features::status(&state.ctx, &sc).await?,
        suite_provides: pc_core::features::suite_provides().to_vec(),
        pack: pc_core::features::pack_keys(),
    })
}

/// Plan installing features (the user confirms in the plan dialog).
#[tauri::command]
async fn features_plan(state: State<'_, AppState>, profile_id: String, keys: Vec<String>) -> R<PlanResult> {
    let (_, sc) = scope(&state, &profile_id)?;
    let mut mods = manage::scan_mods(&sc)?;
    let _ = manage::identify(&state.ctx, &sc, &mut mods).await;
    let installed = manage::installed_set(&mods);
    let plan = pc_core::features::plan(&state.ctx, &sc.target, &installed, &keys).await?;
    Ok(store_plan(&state, &profile_id, plan))
}

#[tauri::command]
fn game_options_get(state: State<'_, AppState>, profile_id: String) -> R<Option<Vec<pc_core::features::GameOption>>> {
    let (_, sc) = scope(&state, &profile_id)?;
    pc_core::features::read_game_options(&sc.game_dir)
}

#[tauri::command]
fn game_options_set(state: State<'_, AppState>, profile_id: String, key: String, value: String) -> R<()> {
    if state.games.is_running(&profile_id) {
        return Err(Error::Invalid("Close the game first: Minecraft rewrites options.txt when it exits.".into()));
    }
    let (_, sc) = scope(&state, &profile_id)?;
    pc_core::features::set_game_option(&sc.game_dir, &sc.data_dir, &key, &value)
}

// ------------------------------------------------------------------ custom main menu

#[tauri::command]
fn menu_read_image(path: PathBuf) -> R<String> {
    pc_core::menu::read_image(&path)
}

#[tauri::command]
fn menu_state(state: State<'_, AppState>, profile_id: String) -> R<pc_core::menu::MenuState> {
    let (_, sc) = scope(&state, &profile_id)?;
    Ok(pc_core::menu::state(&sc.game_dir))
}

/// Write the menu layout + images. FancyMenu itself is installed through the normal plan flow.
#[tauri::command]
fn menu_apply(state: State<'_, AppState>, profile_id: String, images: pc_core::menu::MenuImages) -> R<()> {
    let (_, sc) = scope(&state, &profile_id)?;
    pc_core::menu::apply(&sc.game_dir, &sc.data_dir, &images)
}

#[tauri::command]
fn menu_remove(state: State<'_, AppState>, profile_id: String) -> R<()> {
    let (_, sc) = scope(&state, &profile_id)?;
    pc_core::menu::remove(&sc.game_dir, &sc.data_dir)
}

// ------------------------------------------------------------------ HUD chooser

#[tauri::command]
fn hud_state(state: State<'_, AppState>, profile_id: String) -> R<pc_core::hud::HudState> {
    let (_, sc) = scope(&state, &profile_id)?;
    pc_core::hud::state(&sc.game_dir)
}

#[tauri::command]
fn hud_apply(state: State<'_, AppState>, profile_id: String, enabled: Vec<String>, right_click_cps: Option<bool>) -> R<()> {
    if state.games.is_running(&profile_id) {
        return Err(Error::Invalid("Close the game first, or press Right Shift in game: AxolotlClient saves its HUD settings when the game exits.".into()));
    }
    let (_, sc) = scope(&state, &profile_id)?;
    pc_core::hud::apply(&sc.game_dir, &sc.data_dir, &enabled, right_click_cps)
}

#[tauri::command]
fn hotkeys_get(state: State<'_, AppState>, profile_id: String) -> R<Option<Vec<pc_core::features::Hotkey>>> {
    let (_, sc) = scope(&state, &profile_id)?;
    pc_core::features::read_hotkeys(&sc.game_dir)
}

#[tauri::command]
fn hotkeys_set(state: State<'_, AppState>, profile_id: String, id: String, key: String) -> R<()> {
    if state.games.is_running(&profile_id) {
        return Err(Error::Invalid("Close the game first: Minecraft rewrites options.txt when it exits.".into()));
    }
    let (_, sc) = scope(&state, &profile_id)?;
    pc_core::features::set_hotkey(&sc.game_dir, &sc.data_dir, &id, &key)
}

// ------------------------------------------------------------------ ShadowCombat (server plugin)

#[tauri::command]
fn server_plugin_install(dir: PathBuf) -> R<String> {
    Ok(pc_core::server_plugin::install(&dir)?.display().to_string())
}

// ------------------------------------------------------------------ Minecraft Bedrock

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BedrockView {
    #[serde(flatten)]
    info: pc_core::bedrock::BedrockInfo,
    curseforge_key: bool,
    store_url: &'static str,
}

fn bedrock_dir() -> R<PathBuf> {
    pc_core::bedrock::detect().data_dir.ok_or_else(|| Error::NotFound("Minecraft Bedrock's data folder wasn't found. Start Bedrock once, then try again.".into()))
}

#[tauri::command]
fn bedrock_info(state: State<'_, AppState>) -> BedrockView {
    BedrockView { info: pc_core::bedrock::detect(), curseforge_key: cf::has_key(&state.ctx), store_url: pc_core::bedrock::STORE_URL }
}

#[tauri::command]
fn bedrock_launch() -> R<()> {
    pc_core::bedrock::launch()
}

#[tauri::command]
fn bedrock_packs() -> R<Vec<pc_core::bedrock::BedrockPack>> {
    Ok(pc_core::bedrock::list_packs(&bedrock_dir()?))
}

#[tauri::command]
fn bedrock_import(state: State<'_, AppState>, path: PathBuf) -> R<pc_core::bedrock::ImportReport> {
    pc_core::bedrock::import_file(&bedrock_dir()?, &state.ctx.paths.backups_dir(), &path)
}

#[tauri::command]
fn bedrock_remove(state: State<'_, AppState>, kind: String, folder: String, development: bool) -> R<String> {
    Ok(pc_core::bedrock::remove_pack(&bedrock_dir()?, &state.ctx.paths.backups_dir(), &kind, &folder, development)?.display().to_string())
}

#[tauri::command]
fn bedrock_open_folder(app: tauri::AppHandle, kind: Option<String>) -> R<()> {
    let mut dir = bedrock_dir()?;
    if let Some(k) = kind {
        dir = dir.join(match k.as_str() {
            "resource" => "resource_packs",
            "behavior" => "behavior_packs",
            "skin" => "skin_packs",
            _ => return Err(Error::Invalid("unknown folder".into())),
        });
    }
    std::fs::create_dir_all(&dir)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| Error::msg(e.to_string()))
}

#[tauri::command]
async fn bedrock_cf_classes(state: State<'_, AppState>) -> R<Vec<cf::CfCategory>> {
    cf::classes(&state.ctx, pc_core::bedrock::CURSEFORGE_GAME_ID).await
}

#[tauri::command]
async fn bedrock_cf_search(state: State<'_, AppState>, class_id: u32, query: String, sort: Option<String>, offset: u32) -> R<cf::CfSearchResult> {
    let q = cf::CfSearchQuery { game_id: Some(pc_core::bedrock::CURSEFORGE_GAME_ID), query, class_id, sort, offset, limit: 24, ..Default::default() };
    cf::search(&state.ctx, &q).await
}

#[tauri::command]
async fn bedrock_cf_install(state: State<'_, AppState>, mod_id: u64) -> R<pc_core::bedrock::ImportReport> {
    pc_core::bedrock::install_from_curseforge(&state.ctx, &bedrock_dir()?, &state.ctx.paths.backups_dir(), mod_id).await
}

#[tauri::command]
fn bedrock_menu(state: State<'_, AppState>, user: Option<String>) -> R<pc_core::bedrock_menu::MenuView> {
    pc_core::bedrock_menu::menu(&bedrock_dir()?, &state.ctx.paths.root, &state.ctx.paths.backups_dir(), user.as_deref())
}

#[tauri::command]
fn bedrock_menu_set(state: State<'_, AppState>, user: Option<String>, id: String, value: String) -> R<pc_core::bedrock_menu::ModuleView> {
    pc_core::bedrock_menu::set(&bedrock_dir()?, &state.ctx.paths.root, &state.ctx.paths.backups_dir(), user.as_deref(), &id, &value)
}

#[tauri::command]
fn bedrock_menu_restore(state: State<'_, AppState>, user: Option<String>) -> R<()> {
    pc_core::bedrock_menu::restore(&bedrock_dir()?, &state.ctx.paths.root, &state.ctx.paths.backups_dir(), user.as_deref())
}

// ------------------------------------------------------------------ servers, updates, screenshots

#[tauri::command]
fn servers_list(state: State<'_, AppState>) -> Vec<pc_core::servers::SavedServer> {
    pc_core::servers::list(&state.ctx.paths.root)
}

#[tauri::command]
fn servers_save(state: State<'_, AppState>, servers: Vec<pc_core::servers::SavedServer>) -> R<Vec<pc_core::servers::SavedServer>> {
    pc_core::servers::save(&state.ctx.paths.root, servers)
}

#[tauri::command]
async fn server_ping_java(address: String) -> pc_core::servers::ServerStatus {
    pc_core::servers::ping_java(&address).await
}

#[tauri::command]
async fn server_ping_bedrock(address: String) -> pc_core::servers::ServerStatus {
    pc_core::servers::ping_bedrock(&address).await
}

/// Start a profile and join a server as soon as the game is up (Quick Play).
#[tauri::command]
async fn play_join(state: State<'_, AppState>, profile_id: String, address: String) -> R<GameState> {
    pc_core::servers::parse_address(&address, 25565)?;
    let settings = state.settings();
    let acc = active_account(&state);
    {
        let mut a = state.app.lock().unwrap();
        a.selected_profile = Some(profile_id.clone());
    }
    let _ = state.save_app_state();
    play::play_and_join(&state.ctx, &state.games, &profile_id, &settings, acc.as_deref(), false, Some(&address)).await
}

// ------------------------------------------------------------------ share links

/// A `shadowclient://` link Windows opened Shadow Client with, until the page picks it up.
static PENDING_SHARE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Remember a clicked share link and tell the page (it may not be listening yet: then it asks).
pub fn share_link_arrived(app: &tauri::AppHandle, link: String) {
    *PENDING_SHARE.lock().unwrap() = Some(link.clone());
    let _ = app.emit("share-link", link);
}

#[tauri::command]
fn share_pending() -> Option<String> {
    PENDING_SHARE.lock().unwrap().take()
}

/// The share link for a profile: its Modrinth mods, resource packs and shaders.
#[tauri::command]
async fn share_export(state: State<'_, AppState>, profile_id: String) -> R<pc_core::share::ShareExport> {
    let (p, sc) = scope(&state, &profile_id)?;
    let mut mods = manage::scan_mods(&sc)?;
    let _ = manage::identify(&state.ctx, &sc, &mut mods).await;
    pc_core::share::export(&sc, &p.name, &mods)
}

/// What a share link contains (shown before anything is installed).
#[tauri::command]
fn share_read(link: String) -> R<pc_core::share::SharePack> {
    pc_core::share::decode(&link)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ShareImport {
    profile: Profile,
    plan: PlanResult,
}

/// Make a new profile for a shared pack and plan installing its content (the page shows the plan to confirm).
#[tauri::command]
async fn share_import(state: State<'_, AppState>, link: String, name: String) -> R<ShareImport> {
    let pack = pc_core::share::decode(&link)?;
    let loader = LoaderKind::parse(&pack.loader).unwrap_or(LoaderKind::Vanilla);
    let name = if name.trim().is_empty() { pack.name.clone() } else { name.trim().chars().take(48).collect() };
    let req = NewProfile {
        name,
        kind: ProfileKind::Modded,
        mc_version: pack.mc_version.clone(),
        loader,
        loader_version: None,
        game_dir: None,
        preset: None,
        builtin_mods: Some(matches!(loader, LoaderKind::Fabric | LoaderKind::Quilt)),
        ingame_features: None,
    };
    let settings = state.settings();
    let (profile, _) = setup::create_profile(&state.ctx, req, &settings).await?;
    {
        let mut a = state.app.lock().unwrap();
        a.selected_profile = Some(profile.id.clone());
    }
    state.save_app_state()?;
    let (_, sc) = scope(&state, &profile.id)?;
    let mods = manage::scan_mods(&sc)?;
    let installed = manage::installed_set(&mods);
    let plan = pc_core::share::plan(&state.ctx, &sc.target, &installed, &pack).await?;
    Ok(ShareImport { plan: store_plan(&state, &profile.id, plan), profile })
}

#[tauri::command]
async fn update_check(state: State<'_, AppState>) -> R<pc_core::extras::UpdateInfo> {
    let mut info = pc_core::extras::check_update(&state.ctx).await?;
    info.can_install = match install_kind(&state) {
        Some(InstallKind::Setup) => info.installer.is_some(),
        Some(InstallKind::Msi) => info.msi.is_some(),
        None => false,
    };
    Ok(info)
}

/// How this copy was installed, for updating itself the same way.
#[derive(Clone, Copy, PartialEq)]
enum InstallKind {
    /// The setup.exe installer: its uninstall.exe sits next to the launcher.
    Setup,
    /// The MSI: Windows Installer lists it (an MSI update replaces it the same way).
    Msi,
}

/// Portable copies and dev builds can't update themselves; they get the website instead.
fn install_kind(state: &AppState) -> Option<InstallKind> {
    if !cfg!(windows) || cfg!(debug_assertions) || state.ctx.paths.portable {
        return None;
    }
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if dir.join("uninstall.exe").is_file() {
        return Some(InstallKind::Setup);
    }
    pc_core::extras::installed_by_msi(&dir).then_some(InstallKind::Msi)
}

/// Download and check the newest installer, then run it: setup.exe in passive mode (/P) reopening the new
/// version (/R); the MSI through Windows Installer (passive, asks for admin if the copy was installed for
/// all users), then Shadow Client is opened again. Profiles, worlds and settings stay either way.
#[tauri::command]
async fn update_install(app: tauri::AppHandle, state: State<'_, AppState>) -> R<()> {
    let Some(kind) = install_kind(&state) else {
        return Err(Error::Invalid("This copy of Shadow Client can't update itself. Download the new version from the website.".into()));
    };
    let installer = pc_core::extras::download_update(&state.ctx, kind == InstallKind::Msi).await?;
    if kind == InstallKind::Msi {
        run_msi_update(&installer)?;
    } else {
        std::process::Command::new(&installer)
            .args(["/P", "/R"])
            .spawn()
            .map_err(|e| Error::msg(format!("Couldn't start the installer: {e}")))?;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        app.exit(0);
    });
    Ok(())
}

/// `msiexec` waits for this launcher to close (it's replacing it), then the new version is started.
/// A hidden `cmd` runs the two one after the other, so the update still finishes after we exit.
#[cfg(windows)]
fn run_msi_update(msi: &Path) -> R<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let exe = std::env::current_exe().map_err(|e| Error::msg(e.to_string()))?;
    let (msi, exe) = (msi.display().to_string(), exe.display().to_string());
    if msi.contains('"') || exe.contains('"') {
        return Err(Error::Invalid("Unexpected characters in the update's path".into()));
    }
    std::process::Command::new("cmd")
        .raw_arg(format!("/C start \"\" /wait msiexec /i \"{msi}\" /passive /norestart & start \"\" \"{exe}\""))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| Error::msg(format!("Couldn't start the installer: {e}")))?;
    Ok(())
}

#[cfg(not(windows))]
fn run_msi_update(_msi: &Path) -> R<()> {
    Err(Error::Invalid("MSI updates only run on Windows".into()))
}

#[tauri::command]
fn screenshots_list(app: tauri::AppHandle, state: State<'_, AppState>, profile_id: String) -> R<Vec<pc_core::extras::Screenshot>> {
    let (_, sc) = scope(&state, &profile_id)?;
    let dir = sc.game_dir.join("screenshots");
    // let the page show these images (only this folder, not its subfolders)
    if dir.is_dir() {
        use tauri::Manager;
        let _ = app.asset_protocol_scope().allow_directory(&dir, false);
    }
    Ok(pc_core::extras::screenshots(&sc.game_dir))
}

#[tauri::command]
fn screenshot_remove(state: State<'_, AppState>, profile_id: String, name: String) -> R<String> {
    let (_, sc) = scope(&state, &profile_id)?;
    Ok(pc_core::extras::remove_screenshot(&sc.game_dir, &state.ctx.paths.backups_dir(), &name)?.display().to_string())
}

#[tauri::command]
fn bedrock_worlds(user: Option<String>) -> R<Vec<pc_core::extras::BedrockWorld>> {
    pc_core::extras::bedrock_worlds(&bedrock_dir()?, user.as_deref())
}

#[tauri::command]
fn bedrock_world_backup(state: State<'_, AppState>, user: Option<String>, folder: String) -> R<String> {
    Ok(pc_core::extras::backup_bedrock_world(&bedrock_dir()?, user.as_deref(), &folder, &state.ctx.paths.backups_dir())?.display().to_string())
}

#[tauri::command]
fn bedrock_world_backups(state: State<'_, AppState>) -> Vec<pc_core::extras::WorldBackup> {
    pc_core::extras::bedrock_world_backups(&state.ctx.paths.backups_dir())
}

/// Restoring hands the .mcworld to Bedrock, which imports it as a new copy of the world.
#[tauri::command]
fn bedrock_world_restore(state: State<'_, AppState>, path: PathBuf) -> R<pc_core::bedrock::ImportReport> {
    let backups = state.ctx.paths.backups_dir();
    if !path.starts_with(backups.join("bedrock").join("worlds")) {
        return Err(Error::Invalid("Only Shadow Client's own world backups can be restored here.".into()));
    }
    pc_core::bedrock::import_file(&bedrock_dir()?, &backups, &path)
}

#[tauri::command]
fn bedrock_servers(user: Option<String>) -> R<Vec<pc_core::extras::BedrockServer>> {
    pc_core::extras::bedrock_servers(&bedrock_dir()?, user.as_deref())
}

#[tauri::command]
fn bedrock_server_add(state: State<'_, AppState>, user: Option<String>, name: String, address: String, port: u16) -> R<Vec<pc_core::extras::BedrockServer>> {
    pc_core::extras::add_bedrock_server(&bedrock_dir()?, user.as_deref(), &state.ctx.paths.backups_dir(), &name, &address, port)
}

#[tauri::command]
fn bedrock_server_remove(state: State<'_, AppState>, user: Option<String>, index: u32) -> R<Vec<pc_core::extras::BedrockServer>> {
    pc_core::extras::remove_bedrock_server(&bedrock_dir()?, user.as_deref(), &state.ctx.paths.backups_dir(), index)
}

#[tauri::command]
fn bedrock_join(address: String, port: u16) -> R<()> {
    pc_core::extras::bedrock_join(&address, port)
}
