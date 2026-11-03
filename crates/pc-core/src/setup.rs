//! Creating ready-to-play profiles: loader validation, automatic Sodium + compatible
//! performance mods, built-in mods, and the Performance Mods page status.

use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::minecraft::loaders::{self, LoaderKind};
use crate::mods::compat::DB;
use crate::mods::manage::{self, ProfileScope};
use crate::mods::modrinth as mr;
use crate::mods::resolver::{self, InstallPlan};
use crate::mods::{builtin, ContentKind, Target};
use crate::play::scope_for;
use crate::profiles::{NewProfile, Profile, ProfileKind, Profiles};
use crate::settings::Settings;
use crate::{Error, Result};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupReport {
    pub installed: Vec<String>,
    pub notes: Vec<String>,
    pub sodium: Option<String>,
    pub suggestion_versions: Vec<String>,
}

const SODIUM: &str = "AANobbMI";

/// Plan the performance mod set for a target, skipping anything without a compatible build.
pub async fn plan_performance_mods(ctx: &Ctx, target: &Target, slugs: &[String], installed: &resolver::InstalledSet, report: &mut SetupReport) -> Result<InstallPlan> {
    let mut plan = InstallPlan { target: Some(target.clone()), ..Default::default() };
    for slug in slugs {
        let Some(entry) = DB.performance_mods.iter().find(|m| &m.slug == slug) else { continue };
        if !entry.loaders.iter().any(|l| target.loader.accepted_mod_loaders().contains(&l.as_str())) {
            report.notes.push(format!("{} does not support {}; skipped.", entry.name, target.loader.as_str()));
            continue;
        }
        let p = resolver::resolve_modrinth(ctx, target, ContentKind::Mod, &entry.project_id, None, installed, false).await?;
        if p.blocked {
            if entry.project_id == SODIUM {
                let s = resolver::modrinth_suggestion(ctx, SODIUM, target, ContentKind::Mod).await;
                let versions = s.map(|s| s.mc_versions).unwrap_or_default();
                report.notes.push(format!(
                    "Sodium has no build for Minecraft {} on {} yet, so no renderer optimisation was installed.{}",
                    target.mc_version,
                    target.loader.as_str(),
                    if versions.is_empty() { String::new() } else { format!(" Sodium is available for: {}.", versions.join(", ")) }
                ));
                report.suggestion_versions = versions;
            } else {
                report.notes.push(format!("{}: {}", entry.name, p.blocked_reason.unwrap_or_else(|| "not available".into())));
            }
            continue;
        }
        if entry.project_id == SODIUM {
            report.sodium = p.files.iter().find(|f| f.project_id == SODIUM).map(|f| f.version_number.clone());
        }
        resolver::merge_plans(&mut plan, p);
    }
    let ids = installed.clone();
    plan.finalize(&ids);
    Ok(plan)
}

/// Create a profile and set it up (loader check, performance mods, built-ins).
pub async fn create_profile(ctx: &Ctx, req: NewProfile, settings: &Settings) -> Result<(Profile, SetupReport)> {
    let mut report = SetupReport::default();
    // Validate the loader supports the version before creating anything.
    let mut req = req;
    if matches!(req.loader, LoaderKind::Fabric | LoaderKind::Quilt) && req.loader_version.is_none() {
        req.loader_version = Some(loaders::recommended_loader(&ctx.http, &ctx.api.minecraft, req.loader, &req.mc_version).await?);
    }
    if !req.loader.installable() && req.game_dir.is_none() {
        return Err(Error::Incompatible(format!(
            "{} cannot be installed by Shadow Client. Install it with its official installer, then use Profiles → Add existing installation.",
            req.loader.as_str()
        )));
    }
    let wants_perf = matches!(req.kind, ProfileKind::Performance | ProfileKind::Pvp) && settings.auto_install_sodium;
    let wants_features = req.ingame_features.unwrap_or(false);
    // Built-in mods are always part of Shadow Client profiles that can load mods.
    req.builtin_mods = Some(!req.loader.accepted_mod_loaders().is_empty());
    let profiles = Profiles::new(&ctx.paths);
    let profile = profiles.create(req)?;
    if profile.loader.accepted_mod_loaders().is_empty() {
        return Ok((profile, report));
    }
    let scope = scope_for(ctx, &profile);
    let target = scope.target.clone();
    let mut plan = InstallPlan { target: Some(target.clone()), ..Default::default() };
    let installed = manage::installed_set(&manage::scan_mods(&scope)?);
    if wants_perf {
        let mut slugs = settings.performance_mods.clone();
        if !slugs.iter().any(|s| s == "sodium") {
            slugs.insert(0, "sodium".into());
        }
        let p = plan_performance_mods(ctx, &target, &slugs, &installed, &mut report).await?;
        resolver::merge_plans(&mut plan, p);
    }
    if wants_features {
        let p = crate::features::plan(ctx, &target, &installed, &crate::features::pack_keys()).await?;
        report.notes.extend(p.warnings.iter().cloned());
        resolver::merge_plans(&mut plan, p);
    }
    if profile.builtin_mods {
        // Built-in mods are part of the client and not listed to the player.
        let (p, notes) = builtin::plan(ctx, &target, &installed).await?;
        for n in notes {
            tracing::info!("built-in mods: {n}");
        }
        resolver::merge_plans(&mut plan, p);
    }
    plan.finalize(&installed);
    if plan.blocked {
        report.notes.push(format!("Some mods were not installed: {}", plan.blocked_reason.clone().unwrap_or_default()));
        plan.blocked = false;
        plan.files.retain(|f| f.downloadable);
    }
    if !plan.files.is_empty() {
        let r = manage::execute(ctx, &scope, &plan, builtin::embedded).await?;
        let hidden: Vec<&str> = plan.files.iter().filter(|f| f.builtin).map(|f| f.title.as_str()).collect();
        report.installed = r.installed.into_iter().filter(|t| !hidden.contains(&t.as_str())).collect();
        report.notes.extend(r.warnings);
    }
    if profile.builtin_mods {
        if let Err(e) = builtin::hide_in_mod_menu(&scope.game_dir) {
            tracing::warn!("could not update Mod Menu's hidden list: {e}");
        }
    }
    if wants_features {
        match crate::hud::apply_default_if_new(&scope.game_dir, &scope.data_dir) {
            Ok(true) => report.notes.push("HUD set up like Lunar: FPS, CPS, keystrokes, ping, armor and potions. Change it on the Features page or in game with Right Shift.".into()),
            Ok(false) => {}
            Err(e) => tracing::warn!("could not set the default HUD: {e}"),
        }
    }
    if let Some(v) = &report.sodium {
        report.notes.insert(0, format!("Sodium {v} installed and verified for Minecraft {}.", target.mc_version));
    }
    Ok((profile, report))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfModStatus {
    pub slug: String,
    pub project_id: String,
    pub name: String,
    pub role: String,
    pub summary: String,
    pub recommended: bool,
    /// None = not checked / no network
    pub available_version: Option<String>,
    pub available: Option<bool>,
    pub installed_file: Option<String>,
    pub enabled: bool,
    pub note: Option<String>,
}

/// Status of every curated performance mod for a profile (live availability from Modrinth).
pub async fn performance_status(ctx: &Ctx, scope: &ProfileScope, preset: &str) -> Result<Vec<PerfModStatus>> {
    let mods = manage::scan_mods(scope)?;
    let accepted = scope.target.loader.accepted_mod_loaders();
    let mut out = vec![];
    for m in &DB.performance_mods {
        let installed = mods.iter().find(|x| x.project_id().as_deref() == Some(m.project_id.as_str()) || x.meta.as_ref().is_some_and(|meta| crate::mods::compat::mod_id_for_project(&m.project_id) == Some(meta.mod_id.as_str())));
        let loader_ok = m.loaders.iter().any(|l| accepted.contains(&l.as_str()));
        let (available, version, note) = if !loader_ok {
            (Some(false), None, Some(format!("Not available for {}", scope.target.loader.as_str())))
        } else {
            match mr::project_versions(ctx, &m.project_id, &accepted, &[&scope.target.mc_version]).await {
                Ok(vs) => match mr::pick_best(&vs, &scope.target.mc_version, &accepted) {
                    Some(v) => (Some(true), Some(v.version_number), None),
                    None => (Some(false), None, Some(format!("No build for Minecraft {}", scope.target.mc_version))),
                },
                Err(e) => (None, None, Some(format!("Could not check: {e}"))),
            }
        };
        out.push(PerfModStatus {
            slug: m.slug.clone(),
            project_id: m.project_id.clone(),
            name: m.name.clone(),
            role: m.role.clone(),
            summary: m.summary.clone(),
            recommended: m.presets.iter().any(|p| p == preset),
            available_version: version,
            available,
            installed_file: installed.map(|i| i.file_name.clone()),
            enabled: installed.is_some_and(|i| i.enabled),
            note,
        });
    }
    Ok(out)
}
