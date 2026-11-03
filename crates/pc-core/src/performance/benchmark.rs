//! System benchmark: short, real measurements of CPU (single + multi thread), memory
//! bandwidth and disk speed. This is *not* an in-game FPS test, and the UI says so; it only
//! feeds the preset recommendation together with the detected hardware.

use std::hint::black_box;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::hardware::HardwareInfo;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkResult {
    pub at: String,
    /// Work units per second on one thread (higher is better). Measured reference: a Ryzen 7
    /// 5700X desktop scores ~16 000; entry-level laptop CPUs land well below 5 000.
    pub cpu_single: f64,
    pub cpu_multi: f64,
    pub threads: usize,
    pub memory_gbps: f64,
    pub disk_write_mbps: Option<f64>,
    pub disk_read_mbps: Option<f64>,
    pub duration_ms: u128,
    pub hardware: HardwareInfo,
    pub recommended_preset: String,
    pub reasons: Vec<String>,
}

/// One work unit: integer hashing + branches + a small table walk (roughly what a game's
/// main thread does). Only relative values matter.
fn work_unit(seed: u64) -> u64 {
    let mut table = [0u32; 256];
    let mut x = seed | 1;
    let mut acc = 0u64;
    for i in 0..20_000u32 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let idx = (x & 255) as usize;
        table[idx] = table[idx].wrapping_add(i);
        if x & 3 == 0 {
            acc = acc.wrapping_add(table[(x >> 8 & 255) as usize] as u64);
        } else {
            acc ^= x.rotate_left((i & 31) as u32);
        }
    }
    acc
}

fn units_per_sec(duration_ms: u64) -> f64 {
    let start = Instant::now();
    let mut n = 0u64;
    let mut sink = 0u64;
    while start.elapsed().as_millis() < duration_ms as u128 {
        sink = sink.wrapping_add(work_unit(n.wrapping_add(0x9E37_79B9_7F4A_7C15)));
        n += 1;
    }
    black_box(sink);
    n as f64 / start.elapsed().as_secs_f64()
}

fn memory_bandwidth(total_ram_mb: u64) -> f64 {
    let size = if total_ram_mb < 6000 { 64 } else { 192 } * 1024 * 1024;
    let src = vec![1u8; size];
    let mut dst = vec![0u8; size];
    let start = Instant::now();
    let passes = 4;
    for _ in 0..passes {
        dst.copy_from_slice(black_box(&src));
        black_box(&dst);
    }
    // read + write per byte copied
    (size as f64 * passes as f64 * 2.0) / start.elapsed().as_secs_f64() / 1e9
}

fn disk_speed(dir: &std::path::Path) -> (Option<f64>, Option<f64>) {
    use std::io::{Read, Write};
    let path = dir.join(format!("bench-{}.tmp", uuid::Uuid::new_v4().simple()));
    let data = vec![0x5Au8; 64 * 1024 * 1024];
    let write = (|| -> std::io::Result<f64> {
        std::fs::create_dir_all(dir)?;
        let start = Instant::now();
        let mut f = std::fs::File::create(&path)?;
        f.write_all(&data)?;
        f.sync_all()?;
        Ok(data.len() as f64 / start.elapsed().as_secs_f64() / 1_048_576.0)
    })()
    .ok();
    let read = (|| -> std::io::Result<f64> {
        let start = Instant::now();
        let mut f = std::fs::File::open(&path)?;
        let mut buf = Vec::with_capacity(data.len());
        f.read_to_end(&mut buf)?;
        Ok(buf.len() as f64 / start.elapsed().as_secs_f64() / 1_048_576.0)
    })()
    .ok();
    let _ = std::fs::remove_file(&path);
    (write, read)
}

/// Deterministic preset recommendation from measurements + hardware. Returns (preset, reasons).
pub fn recommend(hw: &HardwareInfo, cpu_single: f64, memory_gbps: f64) -> (String, Vec<String>) {
    let mut score = 0i32;
    let mut reasons = vec![];
    if hw.ram_total_mb < 6 * 1024 {
        score -= 2;
        reasons.push(format!("{} GB RAM leaves little room for Minecraft", hw.ram_total_mb / 1024));
    } else if hw.ram_total_mb >= 12 * 1024 {
        score += 1;
    }
    match hw.primary_gpu() {
        Some(g) if !g.integrated => {
            score += 1;
            reasons.push(format!("Dedicated GPU detected: {}", g.name));
            if g.vram_mb.is_some_and(|v| v >= 4096) {
                score += 1;
            }
        }
        Some(g) => {
            let n = g.name.to_ascii_lowercase();
            let modern_igpu = n.contains("iris") || n.contains("arc") || n.contains("680m") || n.contains("780m") || n.contains("880m") || n.contains("890m");
            if modern_igpu {
                score -= 1;
                reasons.push(format!("Modern integrated graphics ({}) handle Sodium well at moderate settings", g.name));
            } else {
                score -= 2;
                reasons.push(format!("Integrated graphics ({}) benefit most from lighter settings", g.name));
            }
        }
        None => {
            reasons.push("GPU could not be detected".into());
        }
    }
    if cpu_single < 5000.0 {
        score -= 1;
        reasons.push(format!("Single-thread CPU speed is modest ({cpu_single:.0} units/s)"));
    } else if cpu_single > 12000.0 {
        score += 1;
        reasons.push(format!("Fast single-thread CPU ({cpu_single:.0} units/s)"));
    }
    if memory_gbps < 4.0 {
        score -= 1;
        reasons.push(format!("Low memory bandwidth ({memory_gbps:.1} GB/s), common with single-channel RAM"));
    }
    if let Some(d) = &hw.display {
        if d.width * d.height > 2560 * 1440 && !hw.has_dedicated_gpu() {
            score -= 1;
            reasons.push(format!("High resolution ({}×{}) on integrated graphics", d.width, d.height));
        }
    }
    let preset = if score <= -2 { "low" } else { "balanced" };
    if preset == "balanced" {
        reasons.push("For competitive PvP you can pick High FPS instead: it lowers visuals to maximise frame rate.".into());
    }
    (preset.into(), reasons)
}

/// Run the benchmark (blocking; call from a background thread). Takes ~3–5 seconds.
pub fn run(hw: HardwareInfo, tmp_dir: &std::path::Path) -> BenchmarkResult {
    let start = Instant::now();
    let cpu_single = units_per_sec(1200);
    let threads = hw.cpu_logical_cores.clamp(1, 32);
    let handles: Vec<_> = (0..threads).map(|_| std::thread::spawn(|| units_per_sec(1200))).collect();
    let cpu_multi: f64 = handles.into_iter().filter_map(|h| h.join().ok()).sum();
    let memory_gbps = memory_bandwidth(hw.ram_total_mb);
    let (disk_write_mbps, disk_read_mbps) = disk_speed(tmp_dir);
    let (recommended_preset, reasons) = recommend(&hw, cpu_single, memory_gbps);
    BenchmarkResult {
        at: crate::util::now_iso(),
        cpu_single,
        cpu_multi,
        threads,
        memory_gbps,
        disk_write_mbps,
        disk_read_mbps,
        duration_ms: start.elapsed().as_millis(),
        hardware: hw,
        recommended_preset,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::hardware::GpuInfo;

    fn hw(ram: u64, gpu: &str, integrated: bool) -> HardwareInfo {
        HardwareInfo {
            cpu_name: "x".into(),
            cpu_physical_cores: Some(4),
            cpu_logical_cores: 8,
            cpu_mhz: None,
            ram_total_mb: ram,
            ram_available_mb: ram / 2,
            gpus: vec![GpuInfo { name: gpu.into(), vendor: "x".into(), driver_version: None, vram_mb: None, integrated }],
            display: None,
            os_name: None,
            os_version: None,
            free_disk_mb: None,
        }
    }

    #[test]
    fn recommendations() {
        assert_eq!(recommend(&hw(4096, "Intel(R) UHD Graphics 620", true), 3000.0, 3.0).0, "low");
        assert_eq!(recommend(&hw(16384, "NVIDIA GeForce RTX 3060", false), 16000.0, 8.0).0, "balanced");
        // a modern iGPU with a decent CPU is not forced to low-end
        assert_eq!(recommend(&hw(16384, "Intel(R) Iris(R) Xe Graphics", true), 9000.0, 8.0).0, "balanced");
    }

    #[test]
    fn work_unit_is_deterministic() {
        assert_eq!(work_unit(42), work_unit(42));
        assert!(units_per_sec(50) > 0.0);
    }
}
