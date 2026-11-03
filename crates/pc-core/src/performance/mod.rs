pub mod benchmark;
pub mod hardware;
pub mod presets;

use serde::{Deserialize, Serialize};

/// A real sample of the game process (not an estimate).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSample {
    pub pid: u32,
    pub memory_mb: u64,
    /// Percent of one core × cores (sysinfo convention), divided by logical cores to give 0–100
    pub cpu_percent: f32,
    pub at: String,
}

/// Samples one process. Keeps a `System` so CPU usage can be computed between calls.
pub struct ProcessSampler {
    sys: sysinfo::System,
    pid: sysinfo::Pid,
    cores: usize,
}

impl ProcessSampler {
    pub fn new(pid: u32) -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_usage();
        let cores = sys.cpus().len().max(1);
        Self { sys, pid: sysinfo::Pid::from_u32(pid), cores }
    }

    pub fn sample(&mut self) -> Option<ProcessSample> {
        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[self.pid]),
            true,
            sysinfo::ProcessRefreshKind::nothing().with_memory().with_cpu(),
        );
        let p = self.sys.process(self.pid)?;
        Some(ProcessSample {
            pid: self.pid.as_u32(),
            memory_mb: p.memory() / 1_048_576,
            cpu_percent: p.cpu_usage() / self.cores as f32,
            at: crate::util::now_iso(),
        })
    }
}
