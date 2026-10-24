use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MemInfo {
    pub total_kb: u64,
    pub free_kb: u64,
    pub available_kb: u64,
    pub buffers_kb: u64,
    pub cached_kb: u64,
    pub swap_cached_kb: u64,
    pub active_kb: u64,
    pub inactive_kb: u64,
    pub active_anon_kb: u64,
    pub inactive_anon_kb: u64,
    pub active_file_kb: u64,
    pub inactive_file_kb: u64,
    pub dirty_kb: u64,
    pub writeback_kb: u64,
    pub anon_pages_kb: u64,
    pub mapped_kb: u64,
    pub shmem_kb: u64,
    pub slab_kb: u64,
    pub s_reclaimable_kb: u64,
    pub s_unreclaim_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

impl MemInfo {
    pub const DEFAULT_PATH: &'static str = "/proc/meminfo";

    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        let content = fs::read_to_string(p).map_err(|source| CoreError::Io {
            path: p.to_path_buf(),
            source,
        })?;
        Self::parse_str(&content, p)
    }

    pub fn parse_str<P: AsRef<Path>>(content: &str, path: P) -> Result<Self> {
        let p = path.as_ref();
        let mut info = MemInfo::default();
        let mut has_total = false;
        let mut has_free = false;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let Some((key, rest)) = line.split_once(':') else {
                continue;
            };

            let key = key.trim();
            let value_str = rest.trim_start();
            let num_str = value_str.split_whitespace().next().unwrap_or("0");
            let val = num_str.parse::<u64>().unwrap_or(0);

            match key {
                "MemTotal" => {
                    info.total_kb = val;
                    has_total = true;
                }
                "MemFree" => {
                    info.free_kb = val;
                    has_free = true;
                }
                "MemAvailable" => info.available_kb = val,
                "Buffers" => info.buffers_kb = val,
                "Cached" => info.cached_kb = val,
                "SwapCached" => info.swap_cached_kb = val,
                "Active" => info.active_kb = val,
                "Inactive" => info.inactive_kb = val,
                "Active(anon)" => info.active_anon_kb = val,
                "Inactive(anon)" => info.inactive_anon_kb = val,
                "Active(file)" => info.active_file_kb = val,
                "Inactive(file)" => info.inactive_file_kb = val,
                "Dirty" => info.dirty_kb = val,
                "Writeback" => info.writeback_kb = val,
                "AnonPages" => info.anon_pages_kb = val,
                "Mapped" => info.mapped_kb = val,
                "Shmem" => info.shmem_kb = val,
                "Slab" => info.slab_kb = val,
                "SReclaimable" => info.s_reclaimable_kb = val,
                "SUnreclaim" => info.s_unreclaim_kb = val,
                "SwapTotal" => info.swap_total_kb = val,
                "SwapFree" => info.swap_free_kb = val,
                _ => {}
            }
        }

        if !has_total {
            return Err(CoreError::MissingField {
                path: p.to_path_buf(),
                field: "MemTotal",
            });
        }
        if !has_free {
            return Err(CoreError::MissingField {
                path: p.to_path_buf(),
                field: "MemFree",
            });
        }

        // Fallback for older kernels lacking MemAvailable
        if info.available_kb == 0 {
            info.available_kb = info
                .free_kb
                .saturating_add(info.cached_kb)
                .saturating_add(info.s_reclaimable_kb);
        }

        Ok(info)
    }

    /// True used RAM: MemTotal - MemAvailable
    pub fn used_kb(&self) -> u64 {
        self.total_kb.saturating_sub(self.available_kb)
    }

    pub fn used_percent(&self) -> f64 {
        if self.total_kb == 0 {
            0.0
        } else {
            (self.used_kb() as f64 / self.total_kb as f64) * 100.0
        }
    }

    pub fn available_percent(&self) -> f64 {
        if self.total_kb == 0 {
            0.0
        } else {
            (self.available_kb as f64 / self.total_kb as f64) * 100.0
        }
    }

    pub fn cache_and_buffers_kb(&self) -> u64 {
        self.cached_kb
            .saturating_add(self.buffers_kb)
            .saturating_add(self.s_reclaimable_kb)
    }

    pub fn swap_used_kb(&self) -> u64 {
        self.swap_total_kb.saturating_sub(self.swap_free_kb)
    }

    pub fn swap_used_percent(&self) -> f64 {
        if self.swap_total_kb == 0 {
            0.0
        } else {
            (self.swap_used_kb() as f64 / self.swap_total_kb as f64) * 100.0
        }
    }
}
