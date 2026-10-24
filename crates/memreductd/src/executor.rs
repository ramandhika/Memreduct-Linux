use memreduct_core::meminfo::MemInfo;
use memreduct_core::mode::ReduceMode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{Instant, SystemTime};
use tracing::{info, warn};

const CGROUP_RECLAIM_PATH: &str = "/sys/fs/cgroup/memory.reclaim";
const DROP_CACHES_PATH: &str = "/proc/sys/vm/drop_caches";
const COMPACT_MEMORY_PATH: &str = "/proc/sys/vm/compact_memory";

const MIN_COOLDOWN_SECS: u64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReduceStats {
    pub mode: String,
    pub bytes_freed: u64,
    pub available_before_kb: u64,
    pub available_after_kb: u64,
    pub duration_ms: u64,
    pub success: bool,
    pub message: String,
}

#[derive(Default)]
pub struct Executor {
    last_run_epoch_secs: Option<u64>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            last_run_epoch_secs: None,
        }
    }

    pub fn execute(&mut self, mode: ReduceMode) -> Result<ReduceStats, String> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Rate limiting check
        if let Some(last) = self.last_run_epoch_secs {
            let elapsed = now.saturating_sub(last);
            if elapsed < MIN_COOLDOWN_SECS {
                return Err(format!(
                    "Rate limit active: please wait {} more second(s)",
                    MIN_COOLDOWN_SECS - elapsed
                ));
            }
        }

        let start_time = Instant::now();

        // Snapshot before
        let before_info = MemInfo::parse_file(MemInfo::DEFAULT_PATH)
            .map_err(|e| format!("Failed to read /proc/meminfo before reduce: {e}"))?;

        // 1. Sync dirty pages to disk first (safe libc call)
        unsafe {
            libc::sync();
        }

        // 2. Perform requested reduction via hardcoded paths (no shell execution)
        let exec_result = match mode {
            ReduceMode::Reclaim => {
                if Path::new(CGROUP_RECLAIM_PATH).exists() {
                    // Try to reclaim 500M or 20% of used memory
                    let reclaim_target = (before_info.used_kb() / 5).max(100 * 1024);
                    let target_str = format!("{}k\n", reclaim_target);
                    fs::write(CGROUP_RECLAIM_PATH, target_str)
                        .map_err(|e| format!("Failed to write to {CGROUP_RECLAIM_PATH}: {e}"))
                } else {
                    warn!(
                        "cgroup v2 {CGROUP_RECLAIM_PATH} not found, falling back to drop_caches 1"
                    );
                    fs::write(DROP_CACHES_PATH, "1\n")
                        .map_err(|e| format!("Failed to write to {DROP_CACHES_PATH}: {e}"))
                }
            }
            ReduceMode::PageCache => fs::write(DROP_CACHES_PATH, "1\n")
                .map_err(|e| format!("Failed to write to {DROP_CACHES_PATH}: {e}")),
            ReduceMode::Slab => fs::write(DROP_CACHES_PATH, "2\n")
                .map_err(|e| format!("Failed to write to {DROP_CACHES_PATH}: {e}")),
            ReduceMode::AllCaches => fs::write(DROP_CACHES_PATH, "3\n")
                .map_err(|e| format!("Failed to write to {DROP_CACHES_PATH}: {e}")),
            ReduceMode::Compact => fs::write(COMPACT_MEMORY_PATH, "1\n")
                .map_err(|e| format!("Failed to write to {COMPACT_MEMORY_PATH}: {e}")),
        };

        exec_result?;

        // Snapshot after
        let after_info = MemInfo::parse_file(MemInfo::DEFAULT_PATH)
            .map_err(|e| format!("Failed to read /proc/meminfo after reduce: {e}"))?;

        let duration = start_time.elapsed();
        self.last_run_epoch_secs = Some(now);

        let freed_kb = after_info
            .available_kb
            .saturating_sub(before_info.available_kb);
        let bytes_freed = freed_kb * 1024;

        info!(
            "Reduce [{}] completed in {:?}: {} MB freed (avail before: {} MB, after: {} MB)",
            mode,
            duration,
            freed_kb / 1024,
            before_info.available_kb / 1024,
            after_info.available_kb / 1024
        );

        Ok(ReduceStats {
            mode: mode.as_str().to_string(),
            bytes_freed,
            available_before_kb: before_info.available_kb,
            available_after_kb: after_info.available_kb,
            duration_ms: duration.as_millis() as u64,
            success: true,
            message: format!(
                "Successfully executed {}. {:.1} MB freed.",
                mode.display_name(),
                freed_kb as f64 / 1024.0
            ),
        })
    }
}
