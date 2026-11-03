//! Built-in mods shipped with the launcher (see data/builtin-mods.json).
//!
//! For a pinned Minecraft version the embedded jar is used (verified by SHA-512, works
//! offline). For other versions the same Modrinth project is resolved through the normal
//! compatibility checks; if no compatible build exists the mod is skipped with a note.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::resolver::{self, InstallPlan, InstalledSet, PlannedFile};
use super::{ContentKind, Target};
use crate::context::Ctx;
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pinned {
    pub game_version: String,
    pub loaders: Vec<String>,
    pub modrinth_version_id: String,
    pub file_name: String,
    pub embedded: String,
    pub sha1: String,
    pub sha512: String,
    pub size: u64,
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinMod {
    pub key: String,
    pub name: String,
    pub mod_id: String,
    pub modrinth_project_id: String,
    pub modrinth_slug: String,
    pub license: String,
    pub authors: Vec<String>,
    pub category: String,
    pub summary: String,
    pub note: Option<String>,
    /// Shadow Client's own mod: not on Modrinth, so only its pinned versions exist
    #[serde(default)]
    pub self_made: bool,
    pub pinned: Vec<Pinned>,
}

#[derive(Deserialize)]
struct File {
    mods: Vec<BuiltinMod>,
}

pub static BUILTINS: LazyLock<Vec<BuiltinMod>> =
    LazyLock::new(|| serde_json::from_str::<File>(include_str!("../../data/builtin-mods.json")).expect("bundled builtin-mods.json is valid").mods);

/// Embedded jar bytes by file name.
pub fn embedded(name: &str) -> Option<&'static [u8]> {
    match name {
        "marlows-crystal-optimizer-2.0.0+mc1.21.11.jar" => Some(include_bytes!("../../data/builtin/marlows-crystal-optimizer-2.0.0+mc1.21.11.jar")),
        "AlwaysShield-1.2.0+mc1.21.11-Fabric.jar" => Some(include_bytes!("../../data/builtin/AlwaysShield-1.2.0+mc1.21.11-Fabric.jar")),
        "shadow-menu-2.0.1.jar" => Some(include_bytes!("../../data/builtin/shadow-menu-2.0.1.jar")),
        _ => None,
    }
}

/// Mod ids our own mods had under the client's earlier names. A profile that still has one gets
/// it swapped for the current mod (two menus would fight over the same key).
pub const RETIRED_MOD_IDS: &[&str] = &["sunnymenu"];

/// Built-in files to take out before planning: our own mods under an old id, and Shadow Client's
/// own (self-made) mods whose file isn't the one pinned for this version (an older build). Their
/// mod id is then missing, so the plan installs the current file.
pub fn stale_files(mods: &[super::manage::InstalledMod], target: &Target) -> Vec<String> {
    mods.iter()
        .filter_map(|m| {
            let id = m.meta.as_ref()?.mod_id.as_str();
            if RETIRED_MOD_IDS.contains(&id) {
                return Some(m.file_name.clone());
            }
            let b = list().iter().find(|b| b.self_made && b.mod_id == id)?;
            let pinned = pinned_for(b, target)?;
            let base = m.file_name.strip_suffix(".disabled").unwrap_or(&m.file_name);
            (base != pinned.file_name).then(|| m.file_name.clone())
        })
        .collect()
}

pub fn list() -> &'static [BuiltinMod] {
    &BUILTINS
}

fn pinned_for<'a>(b: &'a BuiltinMod, target: &Target) -> Option<&'a Pinned> {
    let accepted = target.loader.accepted_mod_loaders();
    b.pinned.iter().find(|p| p.game_version == target.mc_version && p.loaders.iter().any(|l| accepted.contains(&l.as_str())))
}

/// Plan the built-in mods (and their dependencies) for a profile. Mods without a compatible
/// build are reported in `notes`, never forced.
pub async fn plan(ctx: &Ctx, target: &Target, installed: &InstalledSet) -> Result<(InstallPlan, Vec<String>)> {
    let mut plan = InstallPlan { target: Some(target.clone()), ..Default::default() };
    let mut notes = vec![];
    if target.loader.accepted_mod_loaders().is_empty() {
        notes.push("Built-in mods need a Fabric or Quilt profile.".into());
        return Ok((plan, notes));
    }
    for b in list() {
        if installed.project_ids.contains(&b.modrinth_project_id) || installed.mod_ids.contains(&b.mod_id) {
            continue;
        }
        if let Some(p) = pinned_for(b, target) {
            plan.files.push(PlannedFile {
                platform: "modrinth".into(),
                project_id: b.modrinth_project_id.clone(),
                version_id: p.modrinth_version_id.clone(),
                title: b.name.clone(),
                slug: Some(b.modrinth_slug.clone()),
                version_number: p.file_name.clone(),
                file_name: p.file_name.clone(),
                url: None,
                sha1: Some(p.sha1.clone()),
                sha512: Some(p.sha512.clone()),
                size: Some(p.size),
                reason: "builtin".into(),
                required_by: None,
                replaces: None,
                icon_url: None,
                website_url: Some(super::modrinth::project_url("mod", &b.modrinth_slug)),
                downloadable: true,
                embedded: Some(p.embedded.clone()),
                builtin: true,
                game_versions: vec![p.game_version.clone()],
                loaders: p.loaders.clone(),
                kind: ContentKind::Mod,
            });
            for dep in &p.requires {
                if installed.has_project(dep) || plan.files.iter().any(|f| &f.project_id == dep) {
                    continue;
                }
                let dep_plan = resolver::resolve_modrinth(ctx, target, ContentKind::Mod, dep, None, installed, false).await?;
                for mut f in dep_plan.files {
                    f.reason = "dependency".into();
                    f.required_by = Some(b.name.clone());
                    if !plan.files.iter().any(|x| x.project_id == f.project_id) {
                        plan.files.push(f);
                    }
                }
                if dep_plan.blocked {
                    notes.push(format!("{} skipped: {}", b.name, dep_plan.blocked_reason.unwrap_or_default()));
                    plan.files.retain(|f| f.project_id != b.modrinth_project_id);
                }
            }
            continue;
        }
        if b.self_made {
            notes.push(format!("{} is only built for Minecraft {} so far.", b.name, b.pinned.iter().map(|p| p.game_version.as_str()).collect::<Vec<_>>().join(", ")));
            continue;
        }
        // Not pinned for this version: try Modrinth.
        match resolver::resolve_modrinth(ctx, target, ContentKind::Mod, &b.modrinth_project_id, None, installed, false).await {
            Ok(p) if !p.blocked => {
                for mut f in p.files {
                    if f.project_id == b.modrinth_project_id {
                        f.builtin = true;
                        f.reason = "builtin".into();
                    }
                    if !plan.files.iter().any(|x| x.project_id == f.project_id) {
                        plan.files.push(f);
                    }
                }
            }
            Ok(p) => notes.push(format!("{} is not available for Minecraft {} ({}); skipped.", b.name, target.mc_version, p.blocked_reason.unwrap_or_default())),
            Err(e) => notes.push(format!("{} could not be checked ({e}); skipped.", b.name)),
        }
    }
    if let Some(b) = list().iter().find(|b| b.note.is_some() && plan.files.iter().any(|f| f.project_id == b.modrinth_project_id)) {
        notes.push(b.note.clone().unwrap());
    }
    plan.finalize(installed);
    Ok((plan, notes))
}

/// Keep built-in mods out of Mod Menu's in-game list, like a client's own internals.
/// Merges their mod ids into config/modmenu.json "hidden_mods" (Mod Menu reads only the keys
/// present; everything else in the file is kept). A file that isn't a JSON object is left alone.
pub fn hide_in_mod_menu(game_dir: &std::path::Path) -> Result<bool> {
    use serde_json::Value;
    let path = game_dir.join("config").join("modmenu.json");
    let mut cfg = match crate::util::read_json_opt::<Value>(&path) {
        Ok(Some(v)) if v.is_object() => v,
        Ok(None) => Value::Object(Default::default()),
        Ok(Some(_)) | Err(_) => return Ok(false),
    };
    let obj = cfg.as_object_mut().expect("checked above");
    let hidden = obj.entry("hidden_mods").or_insert_with(|| Value::Array(vec![]));
    if !hidden.is_array() {
        *hidden = Value::Array(vec![]);
    }
    let hidden = hidden.as_array_mut().expect("just set");
    let mut changed = false;
    for b in list() {
        if !hidden.iter().any(|v| v.as_str() == Some(b.mod_id.as_str())) {
            hidden.push(Value::String(b.mod_id.clone()));
            changed = true;
        }
    }
    // hidden mods also don't count toward the "Mods (N)" number
    if obj.get("count_hidden_mods") != Some(&Value::Bool(false)) {
        obj.insert("count_hidden_mods".into(), Value::Bool(false));
        changed = true;
    }
    if changed {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        crate::util::atomic_write(&path, serde_json::to_string_pretty(&cfg).map_err(|e| crate::Error::msg(e.to_string()))?.as_bytes())?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_builtins_in_mod_menu_and_keeps_other_settings() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path();
        assert!(hide_in_mod_menu(g).unwrap());
        let v: serde_json::Value = crate::util::read_json(&g.join("config/modmenu.json")).unwrap();
        assert_eq!(v["hidden_mods"], serde_json::json!(["marlowcrystal", "alwaysshield", "shadowmenu"]));
        assert_eq!(v["count_hidden_mods"], false);
        assert!(!hide_in_mod_menu(g).unwrap(), "idempotent");
        std::fs::write(g.join("config/modmenu.json"), r#"{"sorting":"ascending","hidden_mods":["foo","alwaysshield"],"count_hidden_mods":true}"#).unwrap();
        assert!(hide_in_mod_menu(g).unwrap());
        let v: serde_json::Value = crate::util::read_json(&g.join("config/modmenu.json")).unwrap();
        assert_eq!(v["sorting"], "ascending");
        assert_eq!(v["hidden_mods"], serde_json::json!(["foo", "alwaysshield", "marlowcrystal", "shadowmenu"]));
        std::fs::write(g.join("config/modmenu.json"), "not json").unwrap();
        assert!(!hide_in_mod_menu(g).unwrap());
        assert_eq!(std::fs::read_to_string(g.join("config/modmenu.json")).unwrap(), "not json", "malformed file untouched");
    }

    #[test]
    fn embedded_files_match_pins() {
        for b in list() {
            for p in &b.pinned {
                let bytes = embedded(&p.embedded).expect("embedded jar present");
                assert_eq!(bytes.len() as u64, p.size);
                assert_eq!(crate::util::sha1_hex(bytes), p.sha1, "{} sha1", p.file_name);
            }
        }
        assert_eq!(list().len(), 3);
        assert!(list().iter().any(|b| b.self_made && b.mod_id == "shadowmenu"));
    }
}
