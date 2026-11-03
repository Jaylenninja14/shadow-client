//! Lunar-style in-game features.
//!
//! Shadow Client's built-in mod (Shadow Menu, client-mod/) does the HUD, zoom, freelook, motion
//! blur and the other client modules itself. Extras come from existing, well-known Fabric mods
//! (see data/ingame-features.json); `coveredBySuite` marks the ones Shadow Menu already does.
//! Availability is checked live for the profile's Minecraft version; installs go through the
//! normal resolver (dependencies, conflicts, hash + metadata verification). Vanilla PvP options (toggle sprint, auto-jump…)
//! are edited directly in options.txt with a backup.

use std::path::Path;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::minecraft::options::{self, OptionsFile};
use crate::mods::manage::{self, ProfileScope};
use crate::mods::modrinth as mr;
use crate::mods::resolver::{self, InstallPlan, InstalledSet};
use crate::mods::{ContentKind, Target};
use crate::{util, Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDef {
    pub key: String,
    pub name: String,
    #[serde(rename = "mod")]
    pub mod_name: String,
    pub category: String,
    pub project_id: String,
    pub slug: String,
    pub in_pack: bool,
    pub suite: bool,
    pub covered_by_suite: bool,
    pub summary: String,
    pub note: Option<String>,
    /// An old mod Shadow Menu replaced (AxolotlClient): never offered, only reported when installed
    #[serde(default)]
    pub legacy: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    suite_provides: Vec<String>,
    features: Vec<FeatureDef>,
}

static CATALOG: LazyLock<Catalog> =
    LazyLock::new(|| serde_json::from_str(include_str!("../data/ingame-features.json")).expect("bundled ingame-features.json is valid"));

pub fn catalog() -> &'static [FeatureDef] {
    &CATALOG.features
}

pub fn suite_provides() -> &'static [String] {
    &CATALOG.suite_provides
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureStatus {
    #[serde(flatten)]
    pub def: FeatureDef,
    /// None = could not check (offline)
    pub available: Option<bool>,
    pub available_version: Option<String>,
    pub installed_file: Option<String>,
    pub enabled: bool,
    /// Shadow Menu (the built-in mod) is installed and already offers this feature
    pub overlaps_suite: bool,
    pub check_note: Option<String>,
}

/// Live status of every catalog feature for a profile.
pub async fn status(ctx: &Ctx, scope: &ProfileScope) -> Result<Vec<FeatureStatus>> {
    let mut mods = manage::scan_mods(scope)?;
    // Identify manually added jars so features installed outside the launcher are recognised.
    if let Err(e) = manage::identify(ctx, scope, &mut mods).await {
        tracing::debug!("feature status: identification skipped ({e})");
    }
    let accepted = scope.target.loader.accepted_mod_loaders();
    let menu_installed = mods.iter().any(|m| m.enabled && m.meta.as_ref().is_some_and(|x| x.mod_id == "shadowmenu"));
    let mut out = vec![];
    for def in catalog() {
        let installed = mods.iter().find(|m| m.project_id().as_deref() == Some(def.project_id.as_str()));
        let (available, version, note) = if def.legacy {
            (Some(false), None, None)
        } else if accepted.is_empty() {
            (Some(false), None, Some("Needs a Fabric or Quilt profile".to_string()))
        } else {
            match mr::project_versions(ctx, &def.project_id, &accepted, &[&scope.target.mc_version]).await {
                Ok(vs) => match mr::pick_best(&vs, &scope.target.mc_version, &accepted) {
                    Some(v) => (Some(true), Some(v.version_number), None),
                    None => (Some(false), None, Some(format!("No build for Minecraft {} yet", scope.target.mc_version))),
                },
                Err(e) => (None, None, Some(format!("Could not check: {e}"))),
            }
        };
        out.push(FeatureStatus {
            overlaps_suite: menu_installed && def.covered_by_suite,
            installed_file: installed.map(|m| m.file_name.clone()),
            enabled: installed.is_some_and(|m| m.enabled),
            available,
            available_version: version,
            check_note: note,
            def: def.clone(),
        });
    }
    Ok(out)
}

/// Plan installing a set of features. Unavailable ones are skipped with a warning; the rest
/// (plus dependencies) go into one plan for the user to confirm.
pub async fn plan(ctx: &Ctx, target: &Target, installed: &InstalledSet, keys: &[String]) -> Result<InstallPlan> {
    let mut plan = InstallPlan { target: Some(target.clone()), ..Default::default() };
    if target.loader.accepted_mod_loaders().is_empty() {
        plan.blocked = true;
        plan.blocked_reason = Some("In-game features need a Fabric or Quilt profile. Create one (Profiles → New) for this Minecraft version.".into());
        return Ok(plan);
    }
    for key in keys {
        let def = catalog().iter().find(|d| &d.key == key && !d.legacy).ok_or_else(|| Error::Invalid(format!("unknown feature {key}")))?;
        if installed.project_ids.contains(&def.project_id) {
            plan.already_installed.push(def.mod_name.clone());
            continue;
        }
        let p = resolver::resolve_modrinth(ctx, target, ContentKind::Mod, &def.project_id, None, installed, false).await?;
        if p.blocked {
            plan.warnings.push(format!("{} ({}) skipped: {}", def.name, def.mod_name, p.blocked_reason.unwrap_or_default()));
            continue;
        }
        resolver::merge_plans(&mut plan, p);
    }
    for def in catalog().iter().filter(|d| d.covered_by_suite) {
        if plan.files.iter().any(|f| f.project_id == def.project_id) {
            plan.warnings.push(format!(
                "Shadow Client's built-in menu already includes {}. Having both can show the feature twice or bind the same key; you can switch one off (Right Shift in game, or the Features page).",
                def.name.to_lowercase()
            ));
        }
    }
    plan.finalize(installed);
    Ok(plan)
}

/// The one-click pack (features marked `inPack`).
pub fn pack_keys() -> Vec<String> {
    catalog().iter().filter(|d| d.in_pack && !d.legacy).map(|d| d.key.clone()).collect()
}

// ------------------------------------------------------------------ vanilla game options

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameOption {
    pub key: String,
    pub label: String,
    pub description: String,
    /// "bool" | "percent" (0.0–1.0 stored, shown as %)
    pub kind: String,
    /// Current value from options.txt; None when this Minecraft version has no such option
    pub value: Option<String>,
}

const GAME_OPTIONS: &[(&str, &str, &str, &str)] = &[
    ("toggleSprint", "Toggle sprint", "Press sprint once instead of holding it (like Lunar's ToggleSprint).", "bool"),
    ("toggleCrouch", "Toggle sneak", "Press sneak once instead of holding it.", "bool"),
    ("autoJump", "Auto-jump", "Most PvP players turn this off.", "bool"),
    ("bobView", "View bobbing", "Camera sway while walking.", "bool"),
    ("damageTiltStrength", "Damage tilt", "How much the camera shakes when you take damage (Lunar 'no hurt cam' = 0%).", "percent"),
    ("fovEffectScale", "FOV effects", "How much speed and effects change your field of view.", "percent"),
    ("screenEffectScale", "Distortion effects", "Nausea and portal screen warping.", "percent"),
    ("rawMouseInput", "Raw mouse input", "Reads the mouse directly without Windows pointer acceleration, for consistent aim.", "bool"),
    ("enableVsync", "VSync", "Off gives the lowest input lag (clicks reach the game sooner); on removes screen tearing.", "bool"),
];

/// Read the supported vanilla options from a profile's options.txt.
pub fn read_game_options(game_dir: &Path) -> Result<Option<Vec<GameOption>>> {
    let Some(f) = OptionsFile::load(&game_dir.join("options.txt"))? else { return Ok(None) };
    Ok(Some(
        GAME_OPTIONS
            .iter()
            .map(|(k, label, desc, kind)| GameOption { key: (*k).into(), label: (*label).into(), description: (*desc).into(), kind: (*kind).into(), value: f.get(k).map(String::from) })
            .collect(),
    ))
}

/// Change one vanilla option. Only known keys that already exist in the file are written.
pub fn set_game_option(game_dir: &Path, profile_data: &Path, key: &str, value: &str) -> Result<()> {
    let (_, _, _, kind) = GAME_OPTIONS.iter().find(|(k, ..)| *k == key).ok_or_else(|| Error::Invalid(format!("{key} is not a supported option")))?;
    let normalized = match *kind {
        "bool" => match value {
            "true" | "false" => value.to_string(),
            _ => return Err(Error::Invalid("Expected true or false".into())),
        },
        _ => {
            let v: f64 = value.parse().map_err(|_| Error::Invalid("Expected a number between 0 and 1".into()))?;
            if !(0.0..=1.0).contains(&v) {
                return Err(Error::Invalid("Expected a number between 0 and 1".into()));
            }
            format!("{v}")
        }
    };
    let path = game_dir.join("options.txt");
    let mut f = OptionsFile::load(&path)?.ok_or_else(|| Error::Invalid("Start this profile once so Minecraft creates options.txt.".into()))?;
    if !f.has(key) {
        return Err(Error::Incompatible("This Minecraft version doesn't have that option.".into()));
    }
    options::backup(game_dir, profile_data)?;
    f.set(key, &normalized);
    util::atomic_write(&path, f.render().as_bytes())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hotkey {
    /// Key mapping name; stored in options.txt as `key_<id>:<key>`
    pub id: String,
    pub label: String,
    /// "Client" (Shadow Menu) | "Movement" | "Game"
    pub group: String,
    pub default_key: String,
    /// Current binding, e.g. "key.keyboard.right.shift" or "key.mouse.4"
    pub key: String,
}

/// Hotkeys PvP players usually rebind. Client keys are Shadow Menu's own key mappings
/// (client-mod/: ShadowMenu, feature/Features); they only do something in Fabric/Quilt profiles.
const HOTKEYS: &[(&str, &str, &str, &str)] = &[
    ("key.shadowmenu.open", "Mod menu (HUD editor inside)", "Client", "key.keyboard.right.shift"),
    ("key.shadowmenu.zoom", "Zoom", "Client", "key.keyboard.c"),
    ("key.shadowmenu.freelook", "Freelook", "Client", "key.keyboard.v"),
    ("key.shadowmenu.fullbright", "Toggle fullbright", "Client", "key.keyboard.unknown"),
    ("key.shadowmenu.waypoint", "Add waypoint", "Client", "key.keyboard.b"),
    ("key.sprint", "Sprint", "Movement", "key.keyboard.left.control"),
    ("key.sneak", "Sneak", "Movement", "key.keyboard.left.shift"),
    ("key.jump", "Jump", "Movement", "key.keyboard.space"),
    ("key.swapOffhand", "Swap item to offhand", "Game", "key.keyboard.f"),
    ("key.drop", "Drop item", "Game", "key.keyboard.q"),
    ("key.togglePerspective", "Change perspective", "Game", "key.keyboard.f5"),
    ("key.inventory", "Inventory", "Game", "key.keyboard.e"),
    ("key.chat", "Chat", "Game", "key.keyboard.t"),
    ("key.playerlist", "Player list", "Game", "key.keyboard.tab"),
    ("key.screenshot", "Screenshot", "Game", "key.keyboard.f2"),
];

fn valid_key_name(key: &str) -> bool {
    let rest = key.strip_prefix("key.keyboard.").or_else(|| key.strip_prefix("key.mouse."));
    rest.is_some_and(|r| !r.is_empty() && r.len() <= 24 && r.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.'))
}

/// Read the curated hotkeys from options.txt (None until the game created the file).
pub fn read_hotkeys(game_dir: &Path) -> Result<Option<Vec<Hotkey>>> {
    let Some(f) = OptionsFile::load(&game_dir.join("options.txt"))? else { return Ok(None) };
    Ok(Some(
        HOTKEYS
            .iter()
            .map(|(id, label, group, default)| Hotkey {
                id: (*id).into(),
                label: (*label).into(),
                group: (*group).into(),
                default_key: (*default).into(),
                key: f.get(&format!("key_{id}")).unwrap_or(default).to_string(),
            })
            .collect(),
    ))
}

/// Rebind one curated hotkey (backup first). Mod key lines are added if the mod hasn't saved
/// them yet; Minecraft reads them when the mod registers its key.
pub fn set_hotkey(game_dir: &Path, profile_data: &Path, id: &str, key: &str) -> Result<()> {
    if !HOTKEYS.iter().any(|(k, ..)| *k == id) {
        return Err(Error::Invalid(format!("{id} is not a supported hotkey")));
    }
    if !valid_key_name(key) {
        return Err(Error::Invalid(format!("{key} is not a Minecraft key name")));
    }
    let path = game_dir.join("options.txt");
    let mut f = OptionsFile::load(&path)?.ok_or_else(|| Error::Invalid("Start this profile once so Minecraft creates options.txt.".into()))?;
    options::backup(game_dir, profile_data)?;
    f.set(&format!("key_{id}"), key);
    util::atomic_write(&path, f.render().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_read_defaults_and_rebind_safely() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("g");
        std::fs::create_dir_all(&g).unwrap();
        assert!(read_hotkeys(&g).unwrap().is_none());
        assert!(set_hotkey(&g, &d.path().join("p"), "key.shadowmenu.zoom", "key.keyboard.x").is_err(), "needs options.txt");
        std::fs::write(g.join("options.txt"), "version:4671
key_key.sprint:key.keyboard.left.control
key_key.drop:key.keyboard.q
").unwrap();
        let hk = read_hotkeys(&g).unwrap().unwrap();
        assert_eq!(hk.iter().find(|h| h.id == "key.shadowmenu.open").unwrap().key, "key.keyboard.right.shift", "default when the mod hasn't saved it");
        set_hotkey(&g, &d.path().join("p"), "key.sprint", "key.keyboard.r").unwrap();
        set_hotkey(&g, &d.path().join("p"), "key.shadowmenu.zoom", "key.mouse.4").unwrap();
        let text = std::fs::read_to_string(g.join("options.txt")).unwrap();
        assert!(text.contains("key_key.sprint:key.keyboard.r
") && text.contains("key_key.shadowmenu.zoom:key.mouse.4") && text.contains("key_key.drop:key.keyboard.q"));
        assert!(set_hotkey(&g, d.path(), "key.attack", "key.keyboard.x").is_err(), "not in the curated list");
        assert!(set_hotkey(&g, d.path(), "key.shadowmenu.zoom", "key.keyboard.x\nmaxFps:1").is_err(), "no line injection");
        assert!(set_hotkey(&g, d.path(), "key.shadowmenu.zoom", "keyboard.x").is_err());
        assert!(d.path().join("p").exists(), "backup taken");
    }

    #[test]
    fn catalog_is_consistent() {
        assert_eq!(catalog().iter().filter(|d| d.suite).count(), 0, "the HUD suite is Shadow Menu, built in");
        assert!(!pack_keys().contains(&"axolotlclient".to_string()), "AxolotlClient is no longer needed");
        assert!(catalog().iter().any(|d| d.legacy && d.key == "axolotlclient"), "but still recognised when installed");
        assert!(pack_keys().is_empty(), "everything the pack had is built into Shadow Menu now");
        assert!(catalog().iter().filter(|d| !d.legacy).all(|d| !d.covered_by_suite), "covered mods aren't offered");
        let mut keys: Vec<&str> = catalog().iter().map(|d| d.key.as_str()).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), catalog().len(), "feature keys are unique");
        assert!(catalog().iter().all(|d| d.project_id.len() == 8));
    }

    #[test]
    fn game_options_only_touch_known_existing_keys() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("g");
        std::fs::create_dir_all(&g).unwrap();
        assert!(read_game_options(&g).unwrap().is_none());
        std::fs::write(g.join("options.txt"), "version:4671\ntoggleSprint:false\nautoJump:true\ndamageTiltStrength:1.0\nkey_key.attack:key.mouse.left\n").unwrap();
        let opts = read_game_options(&g).unwrap().unwrap();
        assert_eq!(opts.iter().find(|o| o.key == "toggleSprint").unwrap().value.as_deref(), Some("false"));
        assert!(opts.iter().find(|o| o.key == "toggleCrouch").unwrap().value.is_none());
        set_game_option(&g, &d.path().join("p"), "toggleSprint", "true").unwrap();
        set_game_option(&g, &d.path().join("p"), "damageTiltStrength", "0").unwrap();
        let text = std::fs::read_to_string(g.join("options.txt")).unwrap();
        assert!(text.contains("toggleSprint:true") && text.contains("damageTiltStrength:0") && text.contains("key_key.attack"));
        assert!(set_game_option(&g, &d.path().join("p"), "toggleCrouch", "true").is_err(), "missing key not added");
        assert!(set_game_option(&g, &d.path().join("p"), "gamma", "100").is_err(), "unknown key refused");
        assert!(set_game_option(&g, &d.path().join("p"), "autoJump", "maybe").is_err());
        assert!(set_game_option(&g, &d.path().join("p"), "fovEffectScale", "2").is_err());
    }
}
