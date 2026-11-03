//! Hardware detection through documented, non-admin APIs: sysinfo (CPU/RAM/OS), the display
//! adapter class key in the registry (GPU name/driver/VRAM), and EnumDisplaySettings (display).
//! Anything unavailable is reported as `None` rather than guessed.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub driver_version: Option<String>,
    pub vram_mb: Option<u64>,
    pub integrated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    pub cpu_name: String,
    pub cpu_physical_cores: Option<usize>,
    pub cpu_logical_cores: usize,
    pub cpu_mhz: Option<u64>,
    pub ram_total_mb: u64,
    pub ram_available_mb: u64,
    pub gpus: Vec<GpuInfo>,
    pub display: Option<DisplayInfo>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub free_disk_mb: Option<u64>,
}

impl HardwareInfo {
    /// The GPU Minecraft is most likely to use: a dedicated one if present.
    pub fn primary_gpu(&self) -> Option<&GpuInfo> {
        self.gpus.iter().find(|g| !g.integrated).or_else(|| self.gpus.first())
    }
    pub fn has_dedicated_gpu(&self) -> bool {
        self.gpus.iter().any(|g| !g.integrated)
    }
}

pub fn vendor_of(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if n.contains("nvidia") || n.contains("geforce") || n.contains("quadro") || n.contains("rtx") {
        "NVIDIA"
    } else if n.contains("amd") || n.contains("radeon") || n.contains("ati ") {
        "AMD"
    } else if n.contains("intel") {
        "Intel"
    } else {
        "Other"
    }
}

/// Heuristic based on the adapter name (and VRAM when Windows reports it).
pub fn is_integrated(name: &str, vram_mb: Option<u64>) -> bool {
    let n = name.to_ascii_lowercase();
    match vendor_of(name) {
        "NVIDIA" => false,
        "Intel" => {
            // Arc A/B-series desktop/laptop cards are discrete; "Arc Graphics" (Meteor/Lunar
            // Lake), Iris, UHD and HD Graphics are integrated.
            let discrete_arc = regex::Regex::new(r"arc\(tm\)\s*[ab]\d{3}|arc\s+[ab]\d{3}").unwrap().is_match(&n);
            !discrete_arc
        }
        "AMD" => {
            // APUs report "Radeon(TM) Graphics", "Radeon Vega 8 Graphics", "Radeon 780M", …
            let apu = n.contains("radeon(tm) graphics")
                || n.contains("radeon graphics")
                || n.contains("vega") && n.contains("graphics") && !n.contains("rx vega")
                || regex::Regex::new(r"radeon(\(tm\))?\s*[5-9][0-9]0m").unwrap().is_match(&n);
            apu || vram_mb.is_some_and(|v| v < 1024)
        }
        _ => vram_mb.is_some_and(|v| v < 1024),
    }
}

#[cfg(windows)]
fn gpus() -> Vec<GpuInfo> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;
    let mut out: Vec<GpuInfo> = vec![];
    let Ok(class) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}", KEY_READ) else {
        return out;
    };
    for sub in class.enum_keys().flatten() {
        if !sub.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let Ok(k) = class.open_subkey_with_flags(&sub, KEY_READ) else { continue };
        let Ok(name) = k.get_value::<String, _>("DriverDesc") else { continue };
        let lower = name.to_ascii_lowercase();
        if lower.contains("microsoft basic") || lower.contains("remote display") || lower.contains("virtual") || lower.contains("parsec") || lower.contains("indirect display") {
            continue;
        }
        let vram = k
            .get_raw_value("HardwareInformation.qwMemorySize")
            .ok()
            .and_then(|v| v.bytes.get(..8).map(|b| u64::from_le_bytes(b.try_into().unwrap())))
            .or_else(|| {
                k.get_raw_value("HardwareInformation.MemorySize").ok().and_then(|v| v.bytes.get(..4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as u64))
            })
            .filter(|b| *b > 0)
            .map(|b| b / 1_048_576);
        let driver_version = k.get_value::<String, _>("DriverVersion").ok();
        if out.iter().any(|g| g.name == name) {
            continue;
        }
        out.push(GpuInfo { integrated: is_integrated(&name, vram), vendor: vendor_of(&name).into(), name, driver_version, vram_mb: vram });
    }
    out
}

#[cfg(not(windows))]
fn gpus() -> Vec<GpuInfo> {
    vec![]
}

#[cfg(windows)]
pub fn display() -> Option<DisplayInfo> {
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
    unsafe {
        let mut dm: DEVMODEW = std::mem::zeroed();
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        if EnumDisplaySettingsW(std::ptr::null(), ENUM_CURRENT_SETTINGS, &mut dm) == 0 {
            return None;
        }
        Some(DisplayInfo {
            width: dm.dmPelsWidth,
            height: dm.dmPelsHeight,
            refresh_hz: (dm.dmDisplayFrequency > 1).then_some(dm.dmDisplayFrequency),
        })
    }
}

#[cfg(not(windows))]
pub fn display() -> Option<DisplayInfo> {
    None
}

pub fn detect(data_dir: &std::path::Path) -> HardwareInfo {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();
    let cpus = sys.cpus();
    let cpu_name = cpus.first().map(|c| c.brand().trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Unknown CPU".into());
    let cpu_mhz = cpus.first().map(|c| c.frequency()).filter(|f| *f > 0);
    HardwareInfo {
        cpu_name,
        cpu_physical_cores: sysinfo::System::physical_core_count(),
        cpu_logical_cores: cpus.len().max(1),
        cpu_mhz,
        ram_total_mb: sys.total_memory() / 1_048_576,
        ram_available_mb: sys.available_memory() / 1_048_576,
        gpus: gpus(),
        display: display(),
        os_name: sysinfo::System::long_os_version(),
        os_version: sysinfo::System::os_version(),
        free_disk_mb: crate::util::free_space(data_dir).map(|b| b / 1_048_576),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrated_heuristics() {
        assert!(is_integrated("Intel(R) UHD Graphics 620", Some(128)));
        assert!(is_integrated("Intel(R) Iris(R) Xe Graphics", None));
        assert!(is_integrated("Intel(R) Arc(TM) Graphics", None));
        assert!(!is_integrated("Intel(R) Arc(TM) A770 Graphics", Some(16384)));
        assert!(is_integrated("AMD Radeon(TM) Graphics", Some(512)));
        assert!(is_integrated("AMD Radeon 780M Graphics", None));
        assert!(!is_integrated("AMD Radeon RX 6600", Some(8192)));
        assert!(!is_integrated("NVIDIA GeForce GTX 1050", Some(2048)));
    }

    #[test]
    fn detect_runs() {
        let d = tempfile::tempdir().unwrap();
        let h = detect(d.path());
        assert!(h.ram_total_mb > 0);
        assert!(h.cpu_logical_cores >= 1);
    }
}
