//! Memory (heap) recommendation and validation.
//!
//! More heap does not mean more FPS: an oversized heap only makes garbage collection pauses
//! longer and starves the OS/driver. The recommendation is sized to the workload (mod count)
//! and capped relative to installed RAM.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub fn total_ram_mb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory() / 1_048_576
}

pub fn available_ram_mb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.available_memory() / 1_048_576
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecommendation {
    pub min_mb: u32,
    pub max_mb: u32,
    /// Largest heap we allow on this machine
    pub ceiling_mb: u32,
    pub reasons: Vec<String>,
}

/// Highest heap allowed: leave room for Windows, the GPU driver and the JVM's off-heap memory.
pub fn ceiling_mb(total_mb: u64) -> u32 {
    let reserve = if total_mb <= 4096 { 1536 } else if total_mb <= 8192 { 2560 } else { 4096 };
    let by_reserve = total_mb.saturating_sub(reserve);
    let by_ratio = total_mb * 3 / 4;
    (by_reserve.min(by_ratio).max(1024)) as u32
}

pub fn recommend(total_mb: u64, mod_count: usize, modern: bool, shaders: bool) -> MemoryRecommendation {
    let mut reasons = vec![];
    let mut want: u32 = if !modern {
        reasons.push("Older Minecraft versions need little heap".into());
        1536
    } else if mod_count == 0 {
        reasons.push("Vanilla Minecraft runs well with 2 GB".into());
        2048
    } else if mod_count <= 40 {
        reasons.push(format!("{mod_count} mods: a light modded setup"));
        3072
    } else if mod_count <= 120 {
        reasons.push(format!("{mod_count} mods: a medium modpack"));
        4096
    } else {
        reasons.push(format!("{mod_count} mods: a large modpack"));
        6144
    };
    if shaders {
        want += 512;
        reasons.push("Shaders add some heap usage".into());
    }
    let ceiling = ceiling_mb(total_mb);
    if want > ceiling {
        reasons.push(format!("Capped to {ceiling} MB so Windows and the graphics driver keep enough memory ({total_mb} MB installed)"));
        want = ceiling;
    }
    // Round to 256 MB steps.
    want = (want / 256).max(4) * 256;
    let min = want.min(1024).max(512);
    MemoryRecommendation { min_mb: min, max_mb: want, ceiling_mb: ceiling, reasons }
}

/// Validate user-provided values. `None` means "automatic".
pub fn validate(min: Option<u32>, max: Option<u32>, total_mb: u64) -> Result<()> {
    let ceiling = ceiling_mb(total_mb);
    if let Some(max) = max {
        if max < 512 {
            return Err(Error::Invalid("Maximum memory must be at least 512 MB".into()));
        }
        if max > ceiling {
            return Err(Error::Invalid(format!(
                "Maximum memory {max} MB is too high for this PC ({total_mb} MB installed). Use at most {ceiling} MB so Windows and the GPU driver keep enough memory."
            )));
        }
    }
    if let Some(min) = min {
        if min < 256 {
            return Err(Error::Invalid("Minimum memory must be at least 256 MB".into()));
        }
        if let Some(max) = max {
            if min > max {
                return Err(Error::Invalid("Minimum memory cannot be larger than maximum memory".into()));
            }
        } else if min > ceiling {
            return Err(Error::Invalid(format!("Minimum memory cannot exceed {ceiling} MB on this PC")));
        }
    }
    Ok(())
}

/// Effective (min, max) given optional overrides and the recommendation.
pub fn effective(min: Option<u32>, max: Option<u32>, rec: &MemoryRecommendation) -> (u32, u32) {
    let max = max.unwrap_or(rec.max_mb);
    let min = min.unwrap_or(rec.min_mb).min(max);
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_allocates_everything() {
        for total in [2048u64, 4096, 8192, 16384, 32768, 65536] {
            let c = ceiling_mb(total) as u64;
            assert!(c < total, "ceiling {c} must be below total {total}");
            let r = recommend(total, 500, true, true);
            assert!(r.max_mb as u64 <= c.max(1024));
            assert!(r.min_mb <= r.max_mb);
        }
    }

    #[test]
    fn sizes_by_workload() {
        assert_eq!(recommend(16384, 0, true, false).max_mb, 2048);
        assert_eq!(recommend(16384, 30, true, false).max_mb, 3072);
        assert_eq!(recommend(16384, 200, true, false).max_mb, 6144);
        // 4 GB laptop: heavily capped
        assert!(recommend(4096, 200, true, false).max_mb <= 2560);
    }

    #[test]
    fn validation() {
        assert!(validate(None, None, 8192).is_ok());
        assert!(validate(Some(1024), Some(4096), 16384).is_ok());
        assert!(validate(Some(4096), Some(1024), 16384).is_err());
        assert!(validate(None, Some(16000), 16384).is_err());
        assert!(validate(None, Some(256), 16384).is_err());
    }
}
