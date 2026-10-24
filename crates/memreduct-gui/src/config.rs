use memreduct_core::mode::ReduceMode;
use memreduct_core::policy::AutoReducePolicy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub refresh_interval_secs: u64,
    pub start_minimized: bool,
    pub show_tray: bool,
    pub show_reduct_confirmation: bool,
    pub tray_warning_level: f64,
    pub tray_danger_level: f64,
    pub auto_reduce: AutoReducePolicy,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            refresh_interval_secs: 1,
            start_minimized: false,
            show_tray: true,
            show_reduct_confirmation: true,
            tray_warning_level: 75.0,
            tray_danger_level: 90.0,
            auto_reduce: AutoReducePolicy {
                enabled_threshold: false,
                threshold_used_percent: 85.0,
                enabled_psi: false,
                psi_threshold_avg10: 25.0,
                enabled_interval: false,
                interval_seconds: 1800,
                cooldown_seconds: 300,
                default_mode: ReduceMode::Reclaim,
            },
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg).join("memreduct").join("config.toml")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("memreduct")
                .join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<AppConfig>(&content) {
                    Ok(cfg) => {
                        info!("Loaded configuration from {}", path.display());
                        return cfg;
                    }
                    Err(e) => {
                        warn!("Failed to parse config at {}: {e}", path.display());
                    }
                },
                Err(e) => {
                    warn!("Failed to read config at {}: {e}", path.display());
                }
            }
        }
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())?;
        info!("Saved configuration to {}", path.display());
        Ok(())
    }
}
