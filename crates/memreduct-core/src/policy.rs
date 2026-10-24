use serde::{Deserialize, Serialize};

use crate::mode::ReduceMode;
use crate::snapshot::MemorySnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AutoReduceTrigger {
    Threshold(f64),
    PsiPressure(f64),
    Interval(u64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoReducePolicy {
    pub enabled_threshold: bool,
    pub threshold_used_percent: f64,
    pub enabled_psi: bool,
    pub psi_threshold_avg10: f64,
    pub enabled_interval: bool,
    pub interval_seconds: u64,
    pub cooldown_seconds: u64,
    pub default_mode: ReduceMode,
}

impl Default for AutoReducePolicy {
    fn default() -> Self {
        AutoReducePolicy {
            enabled_threshold: false,
            threshold_used_percent: 85.0,
            enabled_psi: false,
            psi_threshold_avg10: 25.0,
            enabled_interval: false,
            interval_seconds: 1800, // 30 minutes
            cooldown_seconds: 300,  // 5 minutes minimum cooldown
            default_mode: ReduceMode::Reclaim,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AutoReduceEngine {
    pub policy: AutoReducePolicy,
    pub last_reduce_timestamp: Option<u64>,
}

impl AutoReduceEngine {
    pub fn new(policy: AutoReducePolicy) -> Self {
        Self {
            policy,
            last_reduce_timestamp: None,
        }
    }

    pub fn can_reduce(&self, now_secs: u64) -> bool {
        match self.last_reduce_timestamp {
            None => true,
            Some(last) => now_secs.saturating_sub(last) >= self.policy.cooldown_seconds,
        }
    }

    pub fn check(
        &self,
        snapshot: &MemorySnapshot,
        now_secs: u64,
    ) -> Option<(ReduceMode, AutoReduceTrigger)> {
        if !self.can_reduce(now_secs) {
            return None;
        }

        // 1. Check RAM usage threshold
        if self.policy.enabled_threshold {
            let used_pct = snapshot.used_percent();
            if used_pct >= self.policy.threshold_used_percent {
                return Some((
                    self.policy.default_mode,
                    AutoReduceTrigger::Threshold(used_pct),
                ));
            }
        }

        // 2. Check PSI memory pressure
        if self.policy.enabled_psi {
            if let Some(psi) = &snapshot.psi {
                let some_avg10 = psi.some.map(|m| m.avg10).unwrap_or(0.0);
                if some_avg10 >= self.policy.psi_threshold_avg10 {
                    return Some((
                        self.policy.default_mode,
                        AutoReduceTrigger::PsiPressure(some_avg10),
                    ));
                }
            }
        }

        // 3. Check Interval
        if self.policy.enabled_interval {
            let elapsed = match self.last_reduce_timestamp {
                Some(last) => now_secs.saturating_sub(last),
                None => self.policy.interval_seconds,
            };
            if elapsed >= self.policy.interval_seconds {
                return Some((
                    self.policy.default_mode,
                    AutoReduceTrigger::Interval(elapsed),
                ));
            }
        }

        None
    }

    pub fn record_reduce(&mut self, now_secs: u64) {
        self.last_reduce_timestamp = Some(now_secs);
    }
}
