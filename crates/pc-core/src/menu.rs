//! Lunar-style title screen ("main menu") with the user's own background.
//!
//! The menu is drawn in game by FancyMenu (installed through the normal resolver). Shadow
//! Client only writes FancyMenu's own layout format (checked against FancyMenu's source) plus
//! the image assets into the profile's `config/fancymenu` folder:
//!
//! * `customization/sunny_title_screen.txt` – the layout (one file; other layouts untouched)
//! * `assets/sunny/background.png`, `logo.png`, `button.png`, `button_hover.png`
//!
//! Turning it off moves exactly those files into the profile's backups.

use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::{util, Error, Result};

pub const FANCYMENU_PROJECT_ID: &str = "Wq5SjeWM";
const LAYOUT_FILE: &str = "sunny_title_screen.txt";
const ASSET_DIR: &str = "sunny";
const ASSETS: &[&str] = &["background.png", "logo.png", "button.png", "button_hover.png"];

fn fm_dir(game_dir: &Path) -> PathBuf {
    game_dir.join("config").join("fancymenu")
}

fn layout_path(game_dir: &Path) -> PathBuf {
    fm_dir(game_dir).join("customization").join(LAYOUT_FILE)
}

fn asset_dir(game_dir: &Path) -> PathBuf {
    fm_dir(game_dir).join("assets").join(ASSET_DIR)
}

fn is_png(b: &[u8]) -> bool {
    b.len() > 24 && b[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
}

fn is_jpeg(b: &[u8]) -> bool {
    b.len() > 4 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF
}

/// Read a user-chosen image (PNG/JPEG, ≤ 25 MB) as a data URL for the UI to crop/blur.
pub fn read_image(path: &Path) -> Result<String> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > 25 * 1024 * 1024 {
        return Err(Error::Invalid("Images must be smaller than 25 MB".into()));
    }
    let bytes = std::fs::read(path)?;
    let mime = if is_png(&bytes) {
        "image/png"
    } else if is_jpeg(&bytes) {
        "image/jpeg"
    } else {
        return Err(Error::Invalid("Choose a PNG or JPEG image".into()));
    };
    Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

fn decode_png(data_url: &str, what: &str, max: usize) -> Result<Vec<u8>> {
    let b64 = data_url.strip_prefix("data:image/png;base64,").ok_or_else(|| Error::Invalid(format!("{what} must be a PNG")))?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|_| Error::Invalid(format!("{what} is not valid image data")))?;
    if !is_png(&bytes) {
        return Err(Error::Invalid(format!("{what} must be a PNG")));
    }
    if bytes.len() > max {
        return Err(Error::Invalid(format!("{what} is too large")));
    }
    Ok(bytes)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuImages {
    pub background: String,
    pub logo: String,
    pub button: String,
    pub button_hover: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuState {
    pub installed: bool,
    pub background: Option<String>,
}

fn src(name: &str) -> String {
    format!("[source:local]/config/fancymenu/assets/{ASSET_DIR}/{name}")
}

/// One FancyMenu vanilla-widget block (move, resize, restyle or hide a title-screen widget).
fn widget(id: &str, anchor: &str, x: i32, y: i32, w: i32, h: i32, hidden: bool, styled: bool) -> String {
    let mut s = format!(
        "vanilla_button {{\n  element_type = vanilla_button\n  instance_identifier = {id}\n  anchor_point = {anchor}\n  x = {x}\n  y = {y}\n  width = {w}\n  height = {h}\n  is_hidden = {hidden}\n"
    );
    if styled {
        s += &format!(
            "  backgroundnormal = {}\n  backgroundhovered = {}\n  nine_slice_custom_background = true\n  nine_slice_border_x = 3\n  nine_slice_border_y = 3\n  label_base_color = #FFFFFF\n  label_hover_color = #FFD36B\n",
            src("button.png"),
            src("button_hover.png")
        );
    }
    s += "}\n\n";
    s
}

/// Build the FancyMenu layout text (Lunar-like: logo, two big buttons, small row below).
pub fn build_layout(now_ms: u128) -> String {
    let mut s = String::from("type = fancymenu_layout\n\n");
    s += &format!(
        "layout-meta {{\n  identifier = title_screen\n  render_custom_elements_behind_vanilla = false\n  last_edited_time = {now_ms}\n  is_enabled = true\n  randommode = false\n  randomgroup = 1\n  randomonlyfirsttime = false\n  layout_index = 0\n}}\n\n"
    );
    s += &format!(
        "menu_background {{\n  instance_identifier = sunny_background\n  background_type = image\n  image_path = {}\n  slide = false\n  repeat_texture = false\n  parallax = false\n}}\n\n",
        src("background.png")
    );
    s += "customization {\n  action = backgroundoptions\n  keepaspectratio = true\n}\n\n";
    s += &format!(
        "element {{\n  element_type = image\n  instance_identifier = sunny_logo\n  source = {}\n  anchor_point = mid-centered\n  x = -60\n  y = -112\n  width = 120\n  height = 75\n  stay_on_screen = true\n}}\n\n",
        src("logo.png")
    );
    // The two Lunar-style buttons in the middle.
    s += &widget("mc_titlescreen_singleplayer_button", "mid-centered", -100, -24, 200, 20, false, true);
    s += &widget("mc_titlescreen_multiplayer_button", "mid-centered", -100, 2, 200, 20, false, true);
    // Small row at the bottom: Mods (Mod Menu), Options, Quit.
    s += &widget("modmenu_titlescreen_mods_button", "bottom-centered", -152, -34, 98, 20, false, true);
    s += &widget("mc_titlescreen_options_button", "bottom-centered", -49, -34, 98, 20, false, true);
    s += &widget("mc_titlescreen_quit_button", "bottom-centered", 54, -34, 98, 20, false, true);
    // Hidden: vanilla logo, splash, Realms and its notification icons, language/accessibility icons.
    for id in [
        "minecraft_logo_widget",
        "minecraft_splash_widget",
        "minecraft_realms_notification_icons_widget",
        "mc_titlescreen_realms_button",
        "mc_titlescreen_language_button",
        "mc_titlescreen_accessibility_button",
    ] {
        s += &widget(id, "vanilla", 0, 0, 0, 0, true, false);
    }
    s
}

/// Write the menu assets + layout into a profile. Existing files of ours are backed up first.
pub fn apply(game_dir: &Path, profile_data: &Path, images: &MenuImages) -> Result<()> {
    let bg = decode_png(&images.background, "Background", 30 * 1024 * 1024)?;
    let logo = decode_png(&images.logo, "Logo", 4 * 1024 * 1024)?;
    let btn = decode_png(&images.button, "Button texture", 512 * 1024)?;
    let btn_h = decode_png(&images.button_hover, "Button texture", 512 * 1024)?;
    if layout_path(game_dir).exists() {
        backup(game_dir, profile_data)?;
    }
    let dir = asset_dir(game_dir);
    std::fs::create_dir_all(&dir)?;
    for (name, bytes) in [("background.png", &bg), ("logo.png", &logo), ("button.png", &btn), ("button_hover.png", &btn_h)] {
        util::atomic_write(&dir.join(name), bytes)?;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    util::atomic_write(&layout_path(game_dir), build_layout(now).as_bytes())?;
    hide_editor_bar(game_dir)?;
    Ok(())
}

const OVERLAY_KEY: &str = "B:show_customization_overlay = ";

/// FancyMenu shows its editor menu bar (Customization / Tools / Help) on every menu by default.
/// A client's own menu shouldn't, so profiles with our layout get it switched off in
/// config/fancymenu/options.txt (Ctrl+Alt+C in game still toggles it). Other options are kept;
/// FancyMenu fills in missing ones itself.
pub fn hide_editor_bar(game_dir: &Path) -> Result<bool> {
    if !layout_path(game_dir).exists() {
        return Ok(false);
    }
    let path = game_dir.join("config").join("fancymenu").join("options.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let wanted = format!("{OVERLAY_KEY}'false';");
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|l| {
            if l.trim_start().starts_with(OVERLAY_KEY) {
                found = true;
                wanted.clone()
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(wanted);
    }
    let out = lines.join("\n") + "\n";
    if out == text {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    util::atomic_write(&path, out.as_bytes())?;
    Ok(true)
}

fn backup(game_dir: &Path, profile_data: &Path) -> Result<PathBuf> {
    let dest = profile_data.join("backups").join("menu").join(util::timestamp_slug());
    std::fs::create_dir_all(&dest)?;
    if layout_path(game_dir).exists() {
        std::fs::copy(layout_path(game_dir), dest.join(LAYOUT_FILE))?;
    }
    for a in ASSETS {
        let p = asset_dir(game_dir).join(a);
        if p.exists() {
            std::fs::copy(&p, dest.join(a))?;
        }
    }
    Ok(dest)
}

/// Turn the custom menu off: our layout and assets move to backups; nothing else is touched.
pub fn remove(game_dir: &Path, profile_data: &Path) -> Result<()> {
    if !layout_path(game_dir).exists() {
        return Ok(());
    }
    backup(game_dir, profile_data)?;
    std::fs::remove_file(layout_path(game_dir))?;
    for a in ASSETS {
        let p = asset_dir(game_dir).join(a);
        if p.exists() {
            std::fs::remove_file(p)?;
        }
    }
    Ok(())
}

pub fn state(game_dir: &Path) -> MenuState {
    let installed = layout_path(game_dir).is_file();
    let background = if installed {
        std::fs::read(asset_dir(game_dir).join("background.png"))
            .ok()
            .filter(|b| b.len() < 30 * 1024 * 1024)
            .map(|b| format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(b)))
    } else {
        None
    };
    MenuState { installed, background }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_bar_hidden_only_for_our_menu_and_other_options_kept() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path();
        assert!(!hide_editor_bar(g).unwrap(), "no custom menu: untouched");
        std::fs::create_dir_all(layout_path(g).parent().unwrap()).unwrap();
        std::fs::write(layout_path(g), "type = fancymenu_layout").unwrap();
        let opts = g.join("config/fancymenu/options.txt");
        std::fs::write(&opts, "B:modpack_mode = 'false';\nB:show_customization_overlay = 'true';\nD:x = '2.0';\n").unwrap();
        assert!(hide_editor_bar(g).unwrap());
        let t = std::fs::read_to_string(&opts).unwrap();
        assert_eq!(t, "B:modpack_mode = 'false';\nB:show_customization_overlay = 'false';\nD:x = '2.0';\n");
        assert!(!hide_editor_bar(g).unwrap(), "idempotent");
        std::fs::remove_file(&opts).unwrap();
        assert!(hide_editor_bar(g).unwrap(), "created before FancyMenu's first run");
        assert_eq!(std::fs::read_to_string(&opts).unwrap(), "B:show_customization_overlay = 'false';\n");
    }

    fn png_url() -> String {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
        format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(v))
    }

    #[test]
    fn layout_uses_fancymenu_format() {
        let l = build_layout(1);
        assert!(l.starts_with("type = fancymenu_layout\n"));
        assert!(l.contains("identifier = title_screen"));
        assert!(l.contains("background_type = image") && l.contains("image_path = [source:local]/config/fancymenu/assets/sunny/background.png"));
        assert!(l.contains("instance_identifier = mc_titlescreen_singleplayer_button\n  anchor_point = mid-centered"));
        assert!(l.contains("instance_identifier = minecraft_logo_widget\n  anchor_point = vanilla\n  x = 0\n  y = 0\n  width = 0\n  height = 0\n  is_hidden = true"));
        // Every block is closed.
        assert_eq!(l.matches('{').count(), l.matches('}').count());
    }

    #[test]
    fn apply_backup_and_remove_only_touch_our_files() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("game");
        let other = g.join("config/fancymenu/customization/someone_else.txt");
        std::fs::create_dir_all(other.parent().unwrap()).unwrap();
        std::fs::write(&other, "type = fancymenu_layout").unwrap();
        let imgs = MenuImages { background: png_url(), logo: png_url(), button: png_url(), button_hover: png_url() };
        apply(&g, &d.path().join("p"), &imgs).unwrap();
        assert!(state(&g).installed && state(&g).background.is_some());
        apply(&g, &d.path().join("p"), &imgs).unwrap(); // re-apply backs up the previous files
        assert!(d.path().join("p/backups/menu").read_dir().unwrap().next().is_some());
        remove(&g, &d.path().join("p")).unwrap();
        assert!(!state(&g).installed);
        assert!(other.exists(), "other layouts are never touched");
        let bad = MenuImages { background: "data:image/png;base64,AAAA".into(), ..imgs };
        assert!(apply(&g, &d.path().join("p"), &bad).is_err());
    }

    #[test]
    fn read_image_checks_type() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x.txt");
        std::fs::write(&p, b"hello world, not an image at all").unwrap();
        assert!(read_image(&p).is_err());
        let j = d.path().join("x.jpg");
        std::fs::write(&j, [0xFF, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F']).unwrap();
        assert!(read_image(&j).unwrap().starts_with("data:image/jpeg;base64,"));
    }
}
