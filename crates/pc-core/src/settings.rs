//! Persistent user settings. Every field here is read by real code; nothing is decorative.

use serde::{Deserialize, Serialize};

use crate::paths::AppPaths;
use crate::util;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GraphicsSettings {
    pub render_distance: u32,
    pub simulation_distance: u32,
    /// 0 = all, 1 = decreased, 2 = minimal (Minecraft's own encoding)
    pub particles: u32,
    /// 10..=260, 260 means unlimited
    pub max_fps: u32,
    pub vsync: bool,
    /// "fast" | "fancy"
    pub graphics: String,
    /// "off" | "fast" | "fancy"
    pub clouds: String,
    pub entity_shadows: bool,
    pub mipmap_levels: u32,
    pub biome_blend: u32,
    /// percent, 50..=500
    pub entity_distance: u32,
    pub smooth_lighting: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            render_distance: 10,
            simulation_distance: 8,
            particles: 1,
            max_fps: 120,
            vsync: false,
            graphics: "fancy".into(),
            clouds: "fast".into(),
            entity_shadows: true,
            mipmap_levels: 4,
            biome_blend: 2,
            entity_distance: 100,
            smooth_lighting: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    // ---- General
    pub theme: String,
    pub accent: String,
    pub language: String,
    /// "home" | "last"
    pub startup_page: String,
    pub check_mod_updates_on_startup: bool,
    pub notifications: bool,
    /// Where exported profiles and saved downloads go. None = the Windows Downloads folder.
    pub download_dir: Option<String>,
    /// Official launcher directory to scan. None = %APPDATA%\.minecraft.
    pub minecraft_dir: Option<String>,
    /// Opening the official launcher with a profile leaves that profile as its only installation
    /// (the others wait in launcher_profiles.shadowclient-hidden.json; turning this off puts them back).
    pub official_only_this: bool,
    pub extra_scan_dirs: Vec<String>,

    // ---- Minecraft
    pub default_version: Option<String>,
    pub default_loader: String,
    pub default_profile: Option<String>,
    /// None = choose automatically per Minecraft version
    pub java_path: Option<String>,
    /// None = recommended automatically
    pub memory_min_mb: Option<u32>,
    pub memory_max_mb: Option<u32>,
    pub jvm_args: String,
    /// "mojang" (Mojang's G1 defaults) | "minimal" | "zgc"
    pub jvm_preset: String,
    pub resolution_width: Option<u32>,
    pub resolution_height: Option<u32>,
    pub fullscreen: bool,
    /// "keep" | "minimize" | "hide"
    pub on_game_launch: String,
    pub show_snapshots: bool,
    pub show_old_versions: bool,

    // ---- Performance
    /// "auto" | "low" | "balanced" | "high" | "custom"
    pub default_preset: String,
    pub custom_graphics: GraphicsSettings,
    pub auto_install_sodium: bool,
    /// Modrinth slugs of optional performance mods to include in new performance profiles.
    pub performance_mods: Vec<String>,

    // ---- Downloads
    pub download_concurrency: u32,
    pub download_retries: u32,
    pub cache_ttl_minutes: u32,

    // ---- Advanced
    pub debug_logging: bool,
    pub experimental_forge_launch: bool,
    pub prompt_benchmark_on_first_run: bool,

    // ---- Services (not secret: client ids are public identifiers)
    pub msa_client_id: Option<String>,
    pub modrinth_contact: Option<String>,
}

/// Default accent colour comes from branding.json so rebranding changes it too.
fn branding_accent() -> String {
    serde_json::from_str::<serde_json::Value>(include_str!("../../../branding.json"))
        .ok()
        .and_then(|v| v["accent"].as_str().map(String::from))
        .filter(|a| a.len() == 7 && a.starts_with('#'))
        .unwrap_or_else(|| "#9b7bff".into())
}

/// Accent colours that were the default under the client's earlier names. Settings that still
/// have one never chose it, so they follow the current default.
const OLD_DEFAULT_ACCENTS: &[&str] = &["#ffb627"];

/// Optional Microsoft (Azure) client id baked into a build via branding.json, so a shared
/// build can offer Microsoft sign-in without each user configuring it. Client ids are public.
fn branding_client_id() -> Option<String> {
    serde_json::from_str::<serde_json::Value>(include_str!("../../../branding.json"))
        .ok()
        .and_then(|v| v["msaClientId"].as_str().map(|s| s.trim().to_string()))
        .filter(|s| uuid::Uuid::parse_str(s).is_ok())
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            accent: branding_accent(),
            language: "en".into(),
            startup_page: "home".into(),
            check_mod_updates_on_startup: false,
            notifications: true,
            download_dir: None,
            minecraft_dir: None,
            official_only_this: true,
            extra_scan_dirs: vec![],
            default_version: None,
            default_loader: "fabric".into(),
            default_profile: None,
            java_path: None,
            memory_min_mb: None,
            memory_max_mb: None,
            jvm_args: String::new(),
            jvm_preset: "mojang".into(),
            resolution_width: None,
            resolution_height: None,
            fullscreen: false,
            on_game_launch: "minimize".into(),
            show_snapshots: false,
            show_old_versions: false,
            default_preset: "auto".into(),
            custom_graphics: GraphicsSettings::default(),
            auto_install_sodium: true,
            performance_mods: vec![
                "sodium".into(),
                "lithium".into(),
                "ferrite-core".into(),
                "immediatelyfast".into(),
                "entityculling".into(),
                "dynamic-fps".into(),
            ],
            download_concurrency: 6,
            download_retries: 3,
            cache_ttl_minutes: 30,
            debug_logging: false,
            experimental_forge_launch: false,
            prompt_benchmark_on_first_run: true,
            msa_client_id: branding_client_id(),
            modrinth_contact: None,
        }
    }
}

const THEMES: &[&str] = &["dark", "light"];
const STARTUP: &[&str] = &["home", "last"];
const LOADERS: &[&str] = &["vanilla", "fabric", "quilt"];
const JVM_PRESETS: &[&str] = &["mojang", "minimal", "zgc"];
const ON_LAUNCH: &[&str] = &["keep", "minimize", "hide"];
const PRESETS: &[&str] = &["auto", "low", "balanced", "high", "custom"];

impl Settings {
    /// Validate and normalise. Returns a user-facing error describing the first problem.
    pub fn validate(&mut self) -> Result<()> {
        let one_of = |field: &str, v: &str, allowed: &[&str]| -> Result<()> {
            if allowed.contains(&v) {
                Ok(())
            } else {
                Err(Error::Invalid(format!("{field} must be one of {allowed:?}")))
            }
        };
        one_of("theme", &self.theme, THEMES)?;
        one_of("startupPage", &self.startup_page, STARTUP)?;
        one_of("defaultLoader", &self.default_loader, LOADERS)?;
        one_of("jvmPreset", &self.jvm_preset, JVM_PRESETS)?;
        one_of("onGameLaunch", &self.on_game_launch, ON_LAUNCH)?;
        one_of("defaultPreset", &self.default_preset, PRESETS)?;
        if self.language != "en" {
            return Err(Error::Invalid("Only English is available in this version".into()));
        }
        let accent_ok = self.accent.len() == 7
            && self.accent.starts_with('#')
            && self.accent[1..].chars().all(|c| c.is_ascii_hexdigit());
        if !accent_ok {
            return Err(Error::Invalid("Accent colour must look like #12ab34".into()));
        }
        if !(1..=16).contains(&self.download_concurrency) {
            return Err(Error::Invalid("Download concurrency must be between 1 and 16".into()));
        }
        if self.download_retries > 10 {
            return Err(Error::Invalid("Retries must be between 0 and 10".into()));
        }
        if !(1..=1440).contains(&self.cache_ttl_minutes) {
            return Err(Error::Invalid("Cache lifetime must be between 1 and 1440 minutes".into()));
        }
        if let (Some(w), Some(h)) = (self.resolution_width, self.resolution_height) {
            if !(320..=7680).contains(&w) || !(240..=4320).contains(&h) {
                return Err(Error::Invalid("Resolution must be between 320×240 and 7680×4320".into()));
            }
        }
        crate::jvmargs::validate_user_args(&self.jvm_args)?;
        let total = crate::memory::total_ram_mb();
        crate::memory::validate(self.memory_min_mb, self.memory_max_mb, total)?;
        crate::performance::presets::validate_graphics(&self.custom_graphics)?;
        if let Some(id) = &self.msa_client_id {
            let id = id.trim();
            if id.is_empty() {
                self.msa_client_id = None;
            } else if uuid::Uuid::parse_str(id).is_err() {
                return Err(Error::Invalid("The Azure application (client) ID must be a GUID".into()));
            }
        }
        for p in [&self.java_path, &self.download_dir, &self.minecraft_dir].into_iter().flatten() {
            if p.trim().is_empty() {
                return Err(Error::Invalid("Paths cannot be blank; clear the field instead".into()));
            }
        }
        Ok(())
    }
}

pub fn load(paths: &AppPaths) -> Settings {
    match util::read_json_opt::<Settings>(&paths.settings_file()) {
        Ok(Some(mut s)) => {
            if OLD_DEFAULT_ACCENTS.iter().any(|a| s.accent.eq_ignore_ascii_case(a)) {
                s.accent = branding_accent();
            }
            s
        }
        Ok(None) => Settings::default(),
        Err(e) => {
            tracing::warn!("settings.json unreadable ({e}); keeping a copy and using defaults");
            let _ = std::fs::copy(paths.settings_file(), paths.root.join(format!("settings.broken-{}.json", util::timestamp_slug())));
            Settings::default()
        }
    }
}

pub fn save(paths: &AppPaths, settings: &Settings) -> Result<()> {
    util::write_json(&paths.settings_file(), settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate() {
        let mut s = Settings::default();
        s.validate().unwrap();
    }

    #[test]
    fn rejects_bad_values() {
        let mut s = Settings { download_concurrency: 0, ..Default::default() };
        assert!(s.validate().is_err());
        let mut s = Settings { accent: "red".into(), ..Default::default() };
        assert!(s.validate().is_err());
        let mut s = Settings { msa_client_id: Some("not-a-guid".into()), ..Default::default() };
        assert!(s.validate().is_err());
        let mut s = Settings { jvm_args: "-jar evil.jar".into(), ..Default::default() };
        assert!(s.validate().is_err());
    }

    #[test]
    fn corrupt_file_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path());
        std::fs::write(paths.settings_file(), b"{ not json").unwrap();
        assert_eq!(load(&paths), Settings::default());
        save(&paths, &Settings { theme: "light".into(), ..Default::default() }).unwrap();
        assert_eq!(load(&paths).theme, "light");
    }
}
