use memreduct_core::mode::ReduceMode;
use memreduct_core::snapshot::MemorySnapshot;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;
use tracing::{error, info, warn};
use zbus::fdo;
use zbus::interface;
use zbus::object_server::SignalEmitter;

use crate::executor::Executor;
use crate::polkit::{check_polkit_auth, ACTION_REDUCE};

pub struct DaemonService {
    executor: Mutex<Executor>,
}

impl DaemonService {
    pub fn new() -> Self {
        Self {
            executor: Mutex::new(Executor::new()),
        }
    }
}

#[interface(name = "org.memreduct.Daemon")]
impl DaemonService {
    pub async fn reduce(
        &self,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        mode_str: &str,
    ) -> fdo::Result<String> {
        let sender = hdr
            .sender()
            .ok_or_else(|| fdo::Error::Failed("Missing message sender".into()))?
            .as_str();

        info!("D-Bus request 'Reduce' from {sender} with mode '{mode_str}'");

        let mode = ReduceMode::from_str(mode_str).map_err(|e| {
            fdo::Error::InvalidArgs(format!("Invalid reduction mode '{mode_str}': {e}"))
        })?;

        // 1. Authorize with Polkit
        let mut details = HashMap::new();
        details.insert("mode", mode.as_str());

        let authorized = check_polkit_auth(conn, sender, ACTION_REDUCE, details)
            .await
            .map_err(|e| fdo::Error::Failed(format!("Polkit communication error: {e}")))?;

        if !authorized {
            warn!("Polkit rejected Reduce request from {sender}");
            return Err(fdo::Error::AccessDenied(
                "Authorization rejected by Polkit".into(),
            ));
        }

        // 2. Execute reduction logic under lock
        let stats_result = {
            let mut exec = self.executor.lock().unwrap();
            exec.execute(mode)
        };

        match stats_result {
            Ok(stats) => {
                let _ = Self::reduce_finished(&emitter, &stats.mode, stats.bytes_freed, true).await;
                serde_json::to_string(&stats)
                    .map_err(|e| fdo::Error::Failed(format!("JSON serialization error: {e}")))
            }
            Err(e) => {
                error!("Reduce failed: {e}");
                let _ = Self::reduce_finished(&emitter, mode.as_str(), 0, false).await;
                Err(fdo::Error::Failed(e))
            }
        }
    }

    pub fn get_snapshot(&self) -> fdo::Result<String> {
        let snap = MemorySnapshot::capture()
            .map_err(|e| fdo::Error::Failed(format!("Failed to capture memory snapshot: {e}")))?;
        serde_json::to_string(&snap)
            .map_err(|e| fdo::Error::Failed(format!("JSON serialization error: {e}")))
    }

    pub fn get_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[zbus(signal)]
    pub async fn reduce_finished(
        emitter: &SignalEmitter<'_>,
        mode: &str,
        bytes_freed: u64,
        success: bool,
    ) -> zbus::Result<()>;
}
