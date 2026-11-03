//! The PLAY flow and its pre-launch checks.
//!
//! validate profile → resolve & verify game files → check mods → choose Java → confirm account
//! → apply first-run config → launch → supervise. Each stage emits `launch:progress`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::auth::{self, Account};
use crate::context::Ctx;
use crate::events::emit;
use crate::java::{self, JavaChoice, JavaInstall};
use crate::minecraft::install::{self, ResolvedVersion};
use crate::minecraft::launch::{self, ArgInputs, GameManager, GameState};
use crate::minecraft::loaders::LoaderKind;
use crate::minecraft::options;
use crate::mods::manage::{self, ProfileScope};
use crate::mods::Target;
use crate::performance::{hardware, presets};
use crate::profiles::{Profile, Profiles};
use crate::settings::Settings;
use crate::{jvmargs, memory, util, Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    /// "error" | "warning" | "info"
    pub severity: String,
    pub title: String,
    pub detail: String,
    /// Machine-readable fix the UI can offer: "download-java:<component>", "open-mods",
    /// "choose-account", "open-settings"
    pub action: Option<String>,
}

fn problem(severity: &str, title: impl Into<String>, detail: impl Into<String>, action: Option<String>) -> Problem {
    Problem { severity: severity.into(), title: title.into(), detail: detail.into(), action }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preflight {
    pub ready: bool,
    pub problems: Vec<Problem>,
    pub version_id: Option<String>,
    pub required_java: Option<u32>,
    pub java: Option<JavaInstall>,
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    pub memory_reasons: Vec<String>,
    pub mod_count: usize,
    pub enabled_mod_count: usize,
    pub account: Option<Account>,
    pub files_missing: usize,
    pub game_dir: PathBuf,
}

pub fn scope_for(ctx: &Ctx, p: &Profile) -> ProfileScope {
    let profiles = Profiles::new(&ctx.paths);
    ProfileScope {
        profile_id: p.id.clone(),
        game_dir: profiles.game_dir(p),
        data_dir: ctx.paths.profile_data_dir(&p.id),
        target: Target::new(&p.mc_version, p.loader),
    }
}

async fn resolve(ctx: &Ctx, p: &Profile) -> Result<ResolvedVersion> {
    match (&p.external_version_id, &p.external_root) {
        (Some(id), Some(root)) => install::resolve_external(ctx, root, id).await,
        _ => install::resolve_version(ctx, &p.mc_version, p.loader, p.loader_version.as_deref()).await,
    }
}

fn extra_lib_dirs(p: &Profile) -> Vec<PathBuf> {
    p.external_root.iter().map(|r| r.join("libraries")).filter(|d| d.is_dir()).collect()
}

/// Mod warnings that make a launch pointless.
fn is_blocking(w: &str) -> bool {
    w.starts_with("Conflicts with") || w.starts_with("Duplicate") || w.contains(" mod but the profile uses")
}

pub async fn preflight(ctx: &Ctx, profile_id: &str, settings: &Settings, account_id: Option<&str>) -> Result<Preflight> {
    let profiles = Profiles::new(&ctx.paths);
    let p = profiles.get(profile_id)?;
    let scope = scope_for(ctx, &p);
    let mut problems = vec![];

    if !p.loader.installable() && p.external_version_id.is_none() {
        problems.push(problem("error", format!("{} is not installable here", p.loader.as_str()), "Install it with the official installer into a .minecraft folder and manage that installation instead.", None));
    }
    if (p.loader == LoaderKind::Forge || p.loader == LoaderKind::Neoforge) && !settings.experimental_forge_launch {
        problems.push(problem("error", "Forge/NeoForge launching is experimental", "Enable it under Settings → Advanced → Experimental features, or open the official launcher for this profile.", Some("open-settings".into())));
    }

    // Version metadata and files
    let mut version_id = None;
    let mut required_java = None;
    let mut component = None;
    let mut files_missing = 0;
    match resolve(ctx, &p).await {
        Ok(rv) => {
            required_java = Some(crate::minecraft::manifest::required_java(&rv.json));
            component = rv.json.java_version.as_ref().and_then(|j| j.component.clone());
            version_id = Some(rv.id.clone());
            match install::plan(ctx, &rv, &crate::minecraft::manifest::RuleEnv::current(), &extra_lib_dirs(&p)) {
                Ok(plan) => {
                    files_missing = install::missing_items(&plan.items).len();
                    if !plan.missing.is_empty() {
                        problems.push(problem("error", "Game files missing", format!("These cannot be downloaded automatically: {}", plan.missing.join(", ")), None));
                    }
                    if files_missing > 0 {
                        let assets_known = rv.json.asset_index.as_ref().is_some_and(|a| ctx.paths.assets_dir().join("indexes").join(format!("{}.json", a.id)).is_file());
                        problems.push(problem(
                            "info",
                            "Game files will be downloaded",
                            format!(
                                "{files_missing} game files{} are missing or incomplete and will be downloaded and verified before launch.",
                                if assets_known { "" } else { " (plus sounds/textures assets)" }
                            ),
                            None,
                        ));
                    }
                }
                Err(e) => problems.push(problem("error", "Version data is invalid", e.to_string(), None)),
            }
        }
        Err(e) => problems.push(problem("error", "Could not load version information", format!("{e}. Check your internet connection; versions that were installed before work offline."), None)),
    }

    // Mods
    let mods = if scope.folder(crate::mods::ContentKind::Mod).is_dir() { manage::scan_mods(&scope)? } else { vec![] };
    let enabled = mods.iter().filter(|m| m.enabled).count();
    for m in mods.iter().filter(|m| m.enabled) {
        for w in &m.warnings {
            let sev = if is_blocking(w) { "error" } else { "warning" };
            problems.push(problem(sev, m.display_name(), w.clone(), Some("open-mods".into())));
        }
    }

    // Java
    let java_sel = match required_java {
        Some(req) => {
            let installs = java::discover(&ctx.paths.runtimes_dir(), &[]);
            let explicit = p.java_path.clone().or(settings.java_path.clone()).map(PathBuf::from);
            match java::select(&installs, req, component.as_deref(), explicit.as_deref()) {
                Ok(JavaChoice::Found { java, note }) => {
                    if let Some(n) = note {
                        problems.push(problem("warning", format!("Using Java {}", java.major), n, None));
                    }
                    Some(java)
                }
                Ok(JavaChoice::NeedsDownload { required_major, component, message }) => {
                    problems.push(problem(
                        "error",
                        format!("Java {required_major} needed"),
                        format!("{message} Shadow Client can download Mojang's official runtime (verified by hash)."),
                        component.map(|c| format!("download-java:{c}")),
                    ));
                    None
                }
                Err(e) => {
                    problems.push(problem("error", "Java problem", e.to_string(), Some("open-settings".into())));
                    None
                }
            }
        }
        None => None,
    };

    // Account
    let accounts = auth::list_accounts(ctx).unwrap_or_default();
    let account = account_id.and_then(|id| accounts.iter().find(|a| a.id == id).cloned());
    match &account {
        None => problems.push(problem("error", "No account selected", "Sign in with Microsoft, or add a clearly-labelled offline profile for local testing.", Some("choose-account".into()))),
        Some(a) if a.kind == "offline" => problems.push(problem(
            "warning",
            "Offline mode",
            "Offline profiles are for local testing: singleplayer and LAN only. Online servers, Realms and skins need a Microsoft account that owns Minecraft.",
            None,
        )),
        _ => {}
    }

    // Memory
    let total = memory::total_ram_mb();
    let modern = required_java.unwrap_or(17) >= 17;
    let rec = memory::recommend(total, enabled, modern, false);
    let (mn, mx) = memory::effective(p.memory_min_mb.or(settings.memory_min_mb), p.memory_max_mb.or(settings.memory_max_mb), &rec);
    if let Err(e) = memory::validate(Some(mn), Some(mx), total) {
        problems.push(problem("error", "Memory setting", e.to_string(), Some("open-settings".into())));
    }
    if let Err(e) = jvmargs::validate_user_args(p.jvm_args.as_deref().unwrap_or(&settings.jvm_args)) {
        problems.push(problem("error", "JVM arguments", e.to_string(), Some("open-settings".into())));
    }
    if let Some(free) = util::free_space(&ctx.paths.root) {
        if free < 1024 * 1024 * 1024 {
            problems.push(problem("warning", "Low disk space", format!("Only {} MB free on the data drive.", free / 1_048_576), None));
        }
    }

    let ready = !problems.iter().any(|p| p.severity == "error");
    Ok(Preflight {
        ready,
        problems,
        version_id,
        required_java,
        java: java_sel,
        memory_min_mb: mn,
        memory_max_mb: mx,
        memory_reasons: rec.reasons,
        mod_count: mods.len(),
        enabled_mod_count: enabled,
        account,
        files_missing,
        game_dir: scope.game_dir,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    profile_id: &'a str,
    stage: &'a str,
    message: String,
}

fn progress(ctx: &Ctx, profile_id: &str, stage: &str, message: impl Into<String>) {
    emit(&ctx.sink, "launch:progress", &Progress { profile_id, stage, message: message.into() });
}

/// Run the whole PLAY flow. `force` launches despite non-blocking mod warnings.
pub async fn play(ctx: &Ctx, games: &Arc<GameManager>, profile_id: &str, settings: &Settings, account_id: Option<&str>, force: bool) -> Result<GameState> {
    play_and_join(ctx, games, profile_id, settings, account_id, force, None).await
}

/// Start a profile and, with `join_server`, connect to that server once the game is up.
pub async fn play_and_join(
    ctx: &Ctx,
    games: &Arc<GameManager>,
    profile_id: &str,
    settings: &Settings,
    account_id: Option<&str>,
    force: bool,
    join_server: Option<&str>,
) -> Result<GameState> {
    let profiles = Profiles::new(&ctx.paths);
    let mut p = profiles.get(profile_id)?;
    if games.is_running(profile_id) {
        return Err(Error::Invalid("This profile is already running.".into()));
    }
    progress(ctx, profile_id, "validate", "Checking profile");
    let pre = preflight(ctx, profile_id, settings, account_id).await?;
    let blocking: Vec<&Problem> = pre.problems.iter().filter(|x| x.severity == "error").filter(|x| !(force && x.action.as_deref() == Some("open-mods"))).collect();
    if let Some(first) = blocking.first() {
        return Err(Error::Invalid(format!("{}: {}", first.title, first.detail)));
    }

    progress(ctx, profile_id, "files", "Verifying game files");
    let rv = resolve(ctx, &p).await?;
    if p.loader_version.is_none() && rv.loader_version.is_some() && p.external_version_id.is_none() {
        // Pin the loader version so future updates never change it silently.
        p.loader_version = rv.loader_version.clone();
        profiles.save(&p)?;
    }
    let installed = install::install(ctx, rv, &extra_lib_dirs(&p), false).await?;

    if p.builtin_mods && !p.loader.accepted_mod_loaders().is_empty() {
        progress(ctx, profile_id, "mods", "Checking built-in mods");
        let scope = scope_for(ctx, &p);
        if let Err(e) = ensure_builtins(ctx, &scope).await {
            // Never block a launch on this (e.g. offline and the dependency is missing).
            tracing::warn!("could not restore built-in mods: {e}");
        }
        if let Err(e) = crate::mods::builtin::hide_in_mod_menu(&scope.game_dir) {
            tracing::warn!("could not update Mod Menu's hidden list: {e}");
        }
        if let Err(e) = crate::menu::hide_editor_bar(&scope.game_dir) {
            tracing::warn!("could not hide FancyMenu's editor bar: {e}");
        }
    }

    progress(ctx, profile_id, "java", "Selecting Java");
    let java = pre.java.clone().ok_or_else(|| Error::msg("No suitable Java runtime"))?;

    progress(ctx, profile_id, "auth", "Checking account");
    let account_id = account_id.ok_or_else(|| Error::Auth("No account selected".into()))?;
    let auth = auth::launch_auth(ctx, account_id).await?;

    let scope = scope_for(ctx, &p);
    progress(ctx, profile_id, "config", "Applying settings");
    if let Some(preset) = &p.preset {
        if !scope.game_dir.join("options.txt").exists() {
            let hw = hardware::detect(&ctx.paths.root);
            let g = if preset == "custom" { settings.custom_graphics.clone() } else { presets::preset_graphics(preset, Some(&hw)) };
            let dv = options::data_version_from_jar(&installed.plan.client_jar);
            if dv.is_some() {
                options::apply_graphics(&scope.game_dir, &scope.data_dir, &g, &p.mc_version, dv)?;
            }
        }
    }

    let extra = jvmargs::validate_user_args(p.jvm_args.as_deref().unwrap_or(&settings.jvm_args))?;
    let resolution = match (p.resolution_width.or(settings.resolution_width), p.resolution_height.or(settings.resolution_height)) {
        (Some(w), Some(h)) => Some((w, h)),
        _ => None,
    };
    let args = launch::build_args(&ArgInputs {
        installed: &installed,
        java: &java,
        game_dir: &scope.game_dir,
        auth: &auth,
        memory_min_mb: pre.memory_min_mb,
        memory_max_mb: pre.memory_max_mb,
        jvm_preset: &settings.jvm_preset,
        extra_jvm_args: extra,
        resolution,
        fullscreen: p.fullscreen.unwrap_or(settings.fullscreen),
        libraries_dir: &ctx.paths.libraries_dir(),
        assets_dir: &ctx.paths.assets_dir(),
        join_server,
    });
    progress(ctx, profile_id, "launch", format!("Starting Minecraft {} with Java {}", installed.resolved.id, java.major));
    games.spawn(ctx, profile_id, &java, args, &scope.game_dir, Duration::from_secs(25)).await
}

/// Built-in mods are always on: re-enable any that were disabled outside the launcher and
/// reinstall missing ones (only where a compatible build exists).
pub async fn ensure_builtins(ctx: &Ctx, scope: &ProfileScope) -> Result<usize> {
    use crate::mods::index::ModIndex;
    let folder = scope.folder(crate::mods::ContentKind::Mod);
    let index = ModIndex::load(&scope.data_dir);
    let mut fixed = 0;
    for e in index.entries.iter().filter(|e| e.builtin) {
        let base = e.file_name.strip_suffix(".disabled").unwrap_or(&e.file_name).to_string();
        let disabled = folder.join(format!("{base}.disabled"));
        if disabled.is_file() && !folder.join(&base).exists() {
            manage::set_enabled(scope, crate::mods::ContentKind::Mod, &format!("{base}.disabled"), true)?;
            fixed += 1;
        }
    }
    let mut mods = manage::scan_mods(scope)?;
    let stale = crate::mods::builtin::stale_files(&mods, &scope.target);
    if !stale.is_empty() {
        for f in &stale {
            manage::remove_builtin(scope, f)?;
            tracing::info!("replacing outdated built-in {f}");
        }
        fixed += stale.len();
        mods = manage::scan_mods(scope)?;
    }
    let installed = manage::installed_set(&mods);
    let (plan, _notes) = crate::mods::builtin::plan(ctx, &scope.target, &installed).await?;
    if !plan.blocked && !plan.files.is_empty() {
        let r = manage::execute(ctx, scope, &plan, crate::mods::builtin::embedded).await?;
        fixed += r.installed.len();
    }
    Ok(fixed)
}
