//! Graphics presets mapped to real options.txt values.

use serde::Serialize;

use super::hardware::HardwareInfo;
use crate::settings::GraphicsSettings;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

pub const PRESETS: &[PresetInfo] = &[
    PresetInfo { id: "low", name: "Low-end", description: "Short render distance, minimal particles, fast graphics, no clouds or entity shadows, FPS capped at the display refresh rate. For integrated graphics, older CPUs and 4–8 GB RAM." },
    PresetInfo { id: "balanced", name: "Balanced", description: "Moderate distances with fancy graphics and fast clouds. A good default for most PCs with Sodium." },
    PresetInfo { id: "high", name: "High FPS (PvP)", description: "Lower visuals for the highest, most consistent frame rate: fast graphics, no clouds, no entity shadows, uncapped FPS, VSync off." },
    PresetInfo { id: "custom", name: "Custom", description: "Your own values from Settings → Performance." },
];

/// Preset values. The display refresh rate (when known) sets sensible FPS caps.
pub fn preset_graphics(id: &str, hw: Option<&HardwareInfo>) -> GraphicsSettings {
    let refresh = hw.and_then(|h| h.display.as_ref()).and_then(|d| d.refresh_hz).unwrap_or(60);
    match id {
        "low" => GraphicsSettings {
            render_distance: 6,
            simulation_distance: 5,
            particles: 2,
            max_fps: refresh.clamp(30, 144),
            vsync: false,
            graphics: "fast".into(),
            clouds: "off".into(),
            entity_shadows: false,
            mipmap_levels: 2,
            biome_blend: 0,
            entity_distance: 75,
            smooth_lighting: false,
        },
        "high" => GraphicsSettings {
            render_distance: 8,
            simulation_distance: 6,
            particles: 1,
            max_fps: 260,
            vsync: false,
            graphics: "fast".into(),
            clouds: "off".into(),
            entity_shadows: false,
            mipmap_levels: 2,
            biome_blend: 0,
            entity_distance: 100,
            smooth_lighting: true,
        },
        _ => GraphicsSettings {
            render_distance: 10,
            simulation_distance: 8,
            particles: 1,
            max_fps: (refresh * 2).clamp(120, 260),
            vsync: false,
            graphics: "fancy".into(),
            clouds: "fast".into(),
            entity_shadows: true,
            mipmap_levels: 4,
            biome_blend: 2,
            entity_distance: 100,
            smooth_lighting: true,
        },
    }
}

pub fn validate_graphics(g: &GraphicsSettings) -> Result<()> {
    let range = |name: &str, v: u32, lo: u32, hi: u32| -> Result<()> {
        if v < lo || v > hi {
            Err(Error::Invalid(format!("{name} must be between {lo} and {hi}")))
        } else {
            Ok(())
        }
    };
    range("Render distance", g.render_distance, 2, 32)?;
    range("Simulation distance", g.simulation_distance, 5, 32)?;
    range("Particles", g.particles, 0, 2)?;
    range("FPS limit", g.max_fps, 10, 260)?;
    range("Mipmap levels", g.mipmap_levels, 0, 4)?;
    range("Biome blend", g.biome_blend, 0, 7)?;
    range("Entity distance", g.entity_distance, 50, 500)?;
    if !matches!(g.graphics.as_str(), "fast" | "fancy") {
        return Err(Error::Invalid("Graphics must be fast or fancy".into()));
    }
    if !matches!(g.clouds.as_str(), "off" | "fast" | "fancy") {
        return Err(Error::Invalid("Clouds must be off, fast or fancy".into()));
    }
    Ok(())
}

/// Warnings for risky custom values given the hardware.
pub fn graphics_warnings(g: &GraphicsSettings, hw: Option<&HardwareInfo>) -> Vec<String> {
    let mut w = vec![];
    let igpu = hw.is_some_and(|h| !h.has_dedicated_gpu());
    let low_ram = hw.is_some_and(|h| h.ram_total_mb < 8192);
    if g.render_distance > 16 && igpu {
        w.push("Render distance above 16 is very demanding on integrated graphics.".into());
    } else if g.render_distance > 24 {
        w.push("Render distance above 24 needs a strong GPU and lots of RAM.".into());
    }
    if g.simulation_distance > 12 {
        w.push("Simulation distance above 12 mostly costs CPU in singleplayer with little visual benefit.".into());
    }
    if g.simulation_distance > g.render_distance {
        w.push("Simulation distance above render distance has no visible benefit.".into());
    }
    if g.vsync && g.max_fps < 260 {
        w.push("VSync and an FPS limit together can add input lag; usually pick one.".into());
    }
    if low_ram && g.render_distance > 12 {
        w.push("With less than 8 GB RAM, high render distances can cause stutter from memory pressure.".into());
    }
    if g.graphics == "fancy" && igpu && g.render_distance > 12 {
        w.push("Fancy graphics plus long distances can halve FPS on integrated graphics.".into());
    }
    w
}
