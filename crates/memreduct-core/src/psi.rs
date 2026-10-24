use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct PsiMetric {
    pub avg10: f64,
    pub avg60: f64,
    pub avg300: f64,
    pub total_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PsiLevel {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

impl PsiLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn color_code(&self) -> &'static str {
        match self {
            Self::Low => "green",
            Self::Medium => "yellow",
            Self::High => "orange",
            Self::Critical => "red",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PsiReport {
    pub some: Option<PsiMetric>,
    pub full: Option<PsiMetric>,
}

impl PsiReport {
    pub const DEFAULT_PATH: &'static str = "/proc/pressure/memory";

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
        let mut report = PsiReport::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let Some(kind) = parts.next() else { continue };

            let mut metric = PsiMetric::default();

            for item in parts {
                let Some((k, v)) = item.split_once('=') else {
                    continue;
                };
                match k {
                    "avg10" => {
                        metric.avg10 = v.parse().map_err(|_| CoreError::Parse {
                            path: p.to_path_buf(),
                            field: "avg10".into(),
                            detail: v.to_string(),
                        })?;
                    }
                    "avg60" => {
                        metric.avg60 = v.parse().map_err(|_| CoreError::Parse {
                            path: p.to_path_buf(),
                            field: "avg60".into(),
                            detail: v.to_string(),
                        })?;
                    }
                    "avg300" => {
                        metric.avg300 = v.parse().map_err(|_| CoreError::Parse {
                            path: p.to_path_buf(),
                            field: "avg300".into(),
                            detail: v.to_string(),
                        })?;
                    }
                    "total" => {
                        metric.total_us = v.parse().map_err(|_| CoreError::Parse {
                            path: p.to_path_buf(),
                            field: "total".into(),
                            detail: v.to_string(),
                        })?;
                    }
                    _ => {}
                }
            }

            match kind {
                "some" => report.some = Some(metric),
                "full" => report.full = Some(metric),
                _ => {}
            }
        }

        Ok(report)
    }

    pub fn level(&self) -> PsiLevel {
        let some_avg10 = self.some.map(|m| m.avg10).unwrap_or(0.0);
        let full_avg10 = self.full.map(|m| m.avg10).unwrap_or(0.0);

        if full_avg10 >= 20.0 || some_avg10 >= 60.0 {
            PsiLevel::Critical
        } else if full_avg10 >= 5.0 || some_avg10 >= 30.0 {
            PsiLevel::High
        } else if some_avg10 >= 10.0 {
            PsiLevel::Medium
        } else {
            PsiLevel::Low
        }
    }
}
