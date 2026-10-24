use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

use crate::error::Result;
use crate::meminfo::MemInfo;
use crate::psi::PsiReport;
use crate::zram::{ZramReport, ZramStat};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub timestamp_epoch_secs: u64,
    pub meminfo: MemInfo,
    pub psi: Option<PsiReport>,
    pub zram: Option<ZramReport>,
}

impl MemorySnapshot {
    pub fn capture() -> Result<Self> {
        let meminfo = MemInfo::parse_file(MemInfo::DEFAULT_PATH)?;
        let psi = PsiReport::parse_file(PsiReport::DEFAULT_PATH).ok();
        let zram = ZramReport::discover_default();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(MemorySnapshot {
            timestamp_epoch_secs: now,
            meminfo,
            psi,
            zram,
        })
    }

    pub fn capture_from_paths<P1, P2, P3>(
        meminfo_path: P1,
        psi_path: Option<P2>,
        zram_path: Option<P3>,
        now_secs: u64,
    ) -> Result<Self>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
        P3: AsRef<Path>,
    {
        let meminfo = MemInfo::parse_file(meminfo_path)?;
        let psi = match psi_path {
            Some(p) => PsiReport::parse_file(p).ok(),
            None => None,
        };
        let zram = match zram_path {
            Some(p) => {
                let stat = ZramStat::parse_file(p, "zram0").ok();
                stat.map(|s| ZramReport { devices: vec![s] })
            }
            None => None,
        };

        Ok(MemorySnapshot {
            timestamp_epoch_secs: now_secs,
            meminfo,
            psi,
            zram,
        })
    }

    pub fn total_mb(&self) -> u64 {
        self.meminfo.total_kb / 1024
    }

    pub fn used_mb(&self) -> u64 {
        self.meminfo.used_kb() / 1024
    }

    pub fn available_mb(&self) -> u64 {
        self.meminfo.available_kb / 1024
    }

    pub fn free_mb(&self) -> u64 {
        self.meminfo.free_kb / 1024
    }

    pub fn cache_mb(&self) -> u64 {
        self.meminfo.cache_and_buffers_kb() / 1024
    }

    pub fn swap_total_mb(&self) -> u64 {
        self.meminfo.swap_total_kb / 1024
    }

    pub fn swap_used_mb(&self) -> u64 {
        self.meminfo.swap_used_kb() / 1024
    }

    pub fn used_percent(&self) -> f64 {
        self.meminfo.used_percent()
    }

    pub fn available_percent(&self) -> f64 {
        self.meminfo.available_percent()
    }

    pub fn tray_tooltip(&self) -> String {
        let psi_text = match &self.psi {
            Some(p) => format!("{:?}", p.level()),
            None => "N/A".to_string(),
        };

        let zram_text = match &self.zram {
            Some(z) => format!(" | zram: {:.1}x", z.average_compression_ratio()),
            None => String::new(),
        };

        format!(
            "RAM: {:.1}% ({:.1} GB avail)\nSwap: {:.1}% | PSI: {}{}",
            self.used_percent(),
            self.available_mb() as f64 / 1024.0,
            self.meminfo.swap_used_percent(),
            psi_text,
            zram_text
        )
    }
}
