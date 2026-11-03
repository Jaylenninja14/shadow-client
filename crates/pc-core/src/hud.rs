//! Lunar-style HUD chooser: pick which Shadow Menu HUD modules (FPS, CPS, keystrokes…) are on
//! before starting the game. In game, Right Shift opens the mod menu and its HUD editor to drag,
//! resize and restyle them.
//!
//! Shadow Menu (the built-in mod, 2.0+) keeps its settings in config/shadowmenu.json:
//!   {"favorites": [...], "modules": {"hud.fps": {"enabled": true, "x": 0.004, "y": 0.008, "scale": 1.0, …}, …}}
//! A module missing from the file uses the mod's default. We only ever touch `enabled` (and the
//! CPS module's `right`), so positions, sizes and colours set in game are kept.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{util, Error, Result};

/// (module id, name, description, on by default in the mod)
const MODULES: &[(&str, &str, &str, bool)] = &[
    ("fps", "FPS", "Frames per second", true),
    ("cps", "CPS", "Clicks per second", true),
    ("keystrokes", "Keystrokes", "WASD, jump and mouse buttons lighting up as you press them", true),
    ("ping", "Ping", "Your latency to the server", true),
    ("armor-status", "Armor status", "Armor and held item with durability", true),
    ("potion-effects", "Potion effects", "Active effects with time left", true),
    ("coordinates", "Coordinates", "X / Y / Z and facing", false),
    ("sprint-status", "Sprint status", "Shows when toggle sprint / sneak is active", false),
    ("reach", "Reach", "Distance of your last hit", false),
    ("combo", "Combo", "Hits in a row", false),
    ("speed", "Speed", "Movement speed in blocks per second", false),
    ("direction", "Compass", "Direction you are facing", false),
    ("clock", "Clock", "Real-world time", false),
    ("pack-display", "Resource pack", "Which resource packs are active", false),
    ("memory", "Memory", "How much of the game's memory is used", false),
    ("tps", "TPS", "The server's ticks per second", false),
    ("arrows", "Arrow counter", "Arrows you carry", false),
    ("scoreboard", "Scoreboard", "Movable sidebar scoreboard without red numbers", false),
];

/// Lunar's usual starting HUD (CPS counts left and right clicks); also the mod's own defaults.
pub const PVP_DEFAULT: &[&str] = &["fps", "cps", "keystrokes", "ping", "armor-status", "potion-effects"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudModule {
    pub key: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudState {
    /// Shadow Menu has HUD settings in this profile (false before it ever ran or was set up)
    pub configured: bool,
    pub modules: Vec<HudModule>,
    /// CPS module also counts right clicks ("right", on by default)
    pub right_click_cps: bool,
}

fn config_path(game_dir: &Path) -> PathBuf {
    game_dir.join("config").join("shadowmenu.json")
}

fn read_config(path: &Path) -> Result<Option<Value>> {
    let Some(v) = util::read_json_opt::<Value>(path)? else { return Ok(None) };
    if !v.is_object() || v.get("modules").is_some_and(|m| !m.is_object()) {
        return Err(Error::Invalid("Shadow Menu's settings file has an unexpected format; change the HUD in game with Right Shift instead.".into()));
    }
    Ok(Some(v))
}

fn module<'a>(cfg: &'a Value, id: &str) -> Option<&'a serde_json::Map<String, Value>> {
    cfg.get("modules")?.get(format!("hud.{id}"))?.as_object()
}

pub fn state(game_dir: &Path) -> Result<HudState> {
    let cfg = read_config(&config_path(game_dir))?;
    let modules = MODULES
        .iter()
        .map(|(key, name, desc, def)| HudModule {
            key: (*key).into(),
            name: (*name).into(),
            description: (*desc).into(),
            enabled: cfg.as_ref().and_then(|c| module(c, key)).and_then(|e| e.get("enabled")).and_then(Value::as_bool).unwrap_or(*def),
        })
        .collect();
    let right_click_cps = cfg.as_ref().and_then(|c| module(c, "cps")).and_then(|e| e.get("right")).and_then(Value::as_bool).unwrap_or(true);
    let configured = cfg.as_ref().and_then(|c| c.get("modules")).and_then(Value::as_object).is_some_and(|m| m.keys().any(|k| k.starts_with("hud.")));
    Ok(HudState { configured, modules, right_click_cps })
}

/// Turn exactly the given modules on (all other known modules off), keeping everything else in
/// the file. The game must be closed: Shadow Menu writes its settings back while it runs.
pub fn apply(game_dir: &Path, profile_data: &Path, enabled: &[String], right_click_cps: Option<bool>) -> Result<()> {
    if let Some(bad) = enabled.iter().find(|k| !MODULES.iter().any(|(m, ..)| m == k)) {
        return Err(Error::Invalid(format!("Unknown HUD module: {bad}")));
    }
    let path = config_path(game_dir);
    let mut cfg = match read_config(&path)? {
        Some(v) => {
            backup(&path, profile_data)?;
            v
        }
        None => json!({}),
    };
    let root = cfg.as_object_mut().expect("checked above");
    let modules = root.entry("modules").or_insert_with(|| json!({}));
    let modules = modules.as_object_mut().expect("checked above");
    for (key, ..) in MODULES {
        let on = enabled.iter().any(|k| k == key);
        let entry = modules.entry(format!("hud.{key}")).or_insert_with(|| json!({}));
        if !entry.is_object() {
            *entry = json!({});
        }
        entry.as_object_mut().expect("just set").insert("enabled".into(), Value::Bool(on));
    }
    if let Some(v) = right_click_cps {
        if let Some(cps) = modules.get_mut("hud.cps").and_then(Value::as_object_mut) {
            cps.insert("right".into(), Value::Bool(v));
        }
    }
    let text = serde_json::to_string_pretty(&cfg).map_err(|e| Error::msg(e.to_string()))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    util::atomic_write(&path, text.as_bytes())
}

/// Set the default PvP HUD only if Shadow Menu's HUD has never been configured in this profile.
pub fn apply_default_if_new(game_dir: &Path, profile_data: &Path) -> Result<bool> {
    if state(game_dir)?.configured {
        return Ok(false);
    }
    let keys: Vec<String> = PVP_DEFAULT.iter().map(|s| s.to_string()).collect();
    apply(game_dir, profile_data, &keys, Some(true))?;
    Ok(true)
}

fn backup(path: &Path, profile_data: &Path) -> Result<()> {
    let dest = profile_data.join("backups").join("hud").join(util::timestamp_slug());
    std::fs::create_dir_all(&dest)?;
    std::fs::copy(path, dest.join("shadowmenu.json"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(k: &[&str]) -> Vec<String> {
        k.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn defaults_match_the_mod_until_configured() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("g");
        let s = state(&g).unwrap();
        assert!(!s.configured);
        assert!(s.right_click_cps);
        let on: Vec<&str> = s.modules.iter().filter(|m| m.enabled).map(|m| m.key.as_str()).collect();
        assert_eq!(on, PVP_DEFAULT);
        assert!(apply_default_if_new(&g, &d.path().join("p")).unwrap());
        let cfg: Value = util::read_json(&config_path(&g)).unwrap();
        assert_eq!(cfg["modules"]["hud.fps"]["enabled"], true);
        assert_eq!(cfg["modules"]["hud.coordinates"]["enabled"], false);
        assert_eq!(cfg["modules"]["hud.cps"]["right"], true);
        assert!(state(&g).unwrap().configured);
        assert!(!apply_default_if_new(&g, &d.path().join("p")).unwrap(), "second time: left alone");
    }

    #[test]
    fn apply_keeps_other_settings_and_player_positions() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("g");
        std::fs::create_dir_all(g.join("config")).unwrap();
        std::fs::write(
            config_path(&g),
            r#"{"favorites":["zoom"],"tweaks":{"long-chat":true},"modules":{"zoom":{"factor":6},"hud.fps":{"enabled":false,"x":0.7,"y":0.2,"scale":1.5}}}"#,
        )
        .unwrap();
        apply(&g, &d.path().join("p"), &keys(&["fps", "cps"]), Some(false)).unwrap();
        let cfg: Value = util::read_json(&config_path(&g)).unwrap();
        assert_eq!(cfg["favorites"][0], "zoom");
        assert_eq!(cfg["tweaks"]["long-chat"], true);
        assert_eq!(cfg["modules"]["zoom"]["factor"], 6);
        assert_eq!(cfg["modules"]["hud.fps"]["enabled"], true);
        assert_eq!(cfg["modules"]["hud.fps"]["x"], 0.7, "player's position kept");
        assert_eq!(cfg["modules"]["hud.fps"]["scale"], 1.5);
        assert_eq!(cfg["modules"]["hud.cps"]["right"], false);
        assert_eq!(cfg["modules"]["hud.keystrokes"]["enabled"], false);
        assert!(!state(&g).unwrap().right_click_cps);
        assert!(d.path().join("p/backups/hud").read_dir().unwrap().next().is_some(), "backup taken");
    }

    #[test]
    fn refuses_unknown_modules_and_formats() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("g");
        assert!(apply(&g, d.path(), &keys(&["notahud"]), None).is_err());
        std::fs::create_dir_all(g.join("config")).unwrap();
        std::fs::write(config_path(&g), r#"{"modules":[1,2]}"#).unwrap();
        assert!(apply(&g, d.path(), &keys(&["fps"]), None).is_err(), "odd file left alone");
        assert_eq!(std::fs::read_to_string(config_path(&g)).unwrap(), r#"{"modules":[1,2]}"#);
    }
}
