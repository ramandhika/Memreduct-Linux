use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReduceMode {
    #[default]
    Reclaim,
    PageCache,
    Slab,
    AllCaches,
    Compact,
}

impl ReduceMode {
    pub const ALL: [ReduceMode; 5] = [
        ReduceMode::Reclaim,
        ReduceMode::PageCache,
        ReduceMode::Slab,
        ReduceMode::AllCaches,
        ReduceMode::Compact,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reclaim => "reclaim",
            Self::PageCache => "pagecache",
            Self::Slab => "slab",
            Self::AllCaches => "all_caches",
            Self::Compact => "compact",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Reclaim => "Working Set Reclaim (cgroup v2)",
            Self::PageCache => "Clean Page Cache (drop_caches 1)",
            Self::Slab => "Clean Dentry/Inode Slab (drop_caches 2)",
            Self::AllCaches => "Clean All Caches (drop_caches 3)",
            Self::Compact => "Memory Compaction",
        }
    }

    pub fn is_aggressive(&self) -> bool {
        matches!(self, Self::AllCaches | Self::Slab)
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Reclaim => {
                "Reclaim inactive process memory pages via cgroup v2 without dropping disk caches"
            }
            Self::PageCache => "Flush and drop clean page cache pages from memory",
            Self::Slab => "Drop reclaimed filesystem directory and inode entries",
            Self::AllCaches => "Drop both page cache and slab objects (aggressive)",
            Self::Compact => {
                "Trigger kernel memory compaction to reduce physical page fragmentation"
            }
        }
    }
}

impl fmt::Display for ReduceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ReduceMode {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().replace('-', "_").as_str() {
            "reclaim" | "workingset" | "working_set" => Ok(Self::Reclaim),
            "pagecache" | "page_cache" | "cache" => Ok(Self::PageCache),
            "slab" | "dentry" | "inode" => Ok(Self::Slab),
            "all" | "all_caches" | "allcaches" => Ok(Self::AllCaches),
            "compact" | "compaction" => Ok(Self::Compact),
            other => Err(CoreError::InvalidMode(other.to_string())),
        }
    }
}
