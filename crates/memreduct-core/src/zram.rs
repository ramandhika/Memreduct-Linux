use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ZramStat {
    pub device_name: String,
    pub orig_data_bytes: u64,
    pub compr_data_bytes: u64,
    pub mem_used_total_bytes: u64,
    pub mem_limit_bytes: u64,
    pub mem_used_max_bytes: u64,
    pub same_pages: u64,
    pub pages_compacted: u64,
}

impl ZramStat {
    pub fn parse_file<P: AsRef<Path>>(path: P, device_name: &str) -> Result<Self> {
        let p = path.as_ref();
        let content = fs::read_to_string(p).map_err(|source| CoreError::Io {
            path: p.to_path_buf(),
            source,
        })?;
        Self::parse_str(&content, device_name, p)
    }

    pub fn parse_str<P: AsRef<Path>>(content: &str, device_name: &str, path: P) -> Result<Self> {
        let p = path.as_ref();
        let nums: Vec<u64> = content
            .split_whitespace()
            .map(|s| s.parse::<u64>().unwrap_or(0))
            .collect();

        if nums.len() < 3 {
            return Err(CoreError::Parse {
                path: p.to_path_buf(),
                field: "mm_stat".into(),
                detail: format!("Expected at least 3 values, got {}", nums.len()),
            });
        }

        Ok(ZramStat {
            device_name: device_name.to_string(),
            orig_data_bytes: nums[0],
            compr_data_bytes: nums[1],
            mem_used_total_bytes: nums[2],
            mem_limit_bytes: nums.get(3).copied().unwrap_or(0),
            mem_used_max_bytes: nums.get(4).copied().unwrap_or(0),
            same_pages: nums.get(5).copied().unwrap_or(0),
            pages_compacted: nums.get(6).copied().unwrap_or(0),
        })
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.compr_data_bytes == 0 {
            1.0
        } else {
            self.orig_data_bytes as f64 / self.compr_data_bytes as f64
        }
    }

    pub fn saved_bytes(&self) -> u64 {
        self.orig_data_bytes.saturating_sub(self.compr_data_bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ZramReport {
    pub devices: Vec<ZramStat>,
}

impl ZramReport {
    pub fn discover_default() -> Option<Self> {
        let mut devices = Vec::new();

        // Check /sys/block/zram0 through zram7
        for i in 0..8 {
            let dev_name = format!("zram{}", i);
            let path = format!("/sys/block/{}/mm_stat", dev_name);
            if Path::new(&path).exists() {
                if let Ok(stat) = ZramStat::parse_file(&path, &dev_name) {
                    devices.push(stat);
                }
            }
        }

        if devices.is_empty() {
            None
        } else {
            Some(ZramReport { devices })
        }
    }

    pub fn total_orig_bytes(&self) -> u64 {
        self.devices.iter().map(|d| d.orig_data_bytes).sum()
    }

    pub fn total_compr_bytes(&self) -> u64 {
        self.devices.iter().map(|d| d.compr_data_bytes).sum()
    }

    pub fn total_mem_used_bytes(&self) -> u64 {
        self.devices.iter().map(|d| d.mem_used_total_bytes).sum()
    }

    pub fn average_compression_ratio(&self) -> f64 {
        let compr = self.total_compr_bytes();
        if compr == 0 {
            1.0
        } else {
            self.total_orig_bytes() as f64 / compr as f64
        }
    }
}
