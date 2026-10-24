use memreduct_core::mode::ReduceMode;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::info;
use zbus::Connection;

#[zbus::proxy(
    interface = "org.memreduct.Daemon",
    default_service = "org.memreduct.Daemon",
    default_path = "/org/memreduct/Daemon"
)]
trait Daemon {
    fn reduce(&self, mode: &str) -> zbus::Result<String>;
    fn get_snapshot(&self) -> zbus::Result<String>;
    fn get_version(&self) -> zbus::Result<String>;
}

#[derive(Clone)]
pub struct DaemonClient {
    connection: Arc<Mutex<Option<Connection>>>,
    use_session: bool,
}

impl DaemonClient {
    pub fn new(use_session: bool) -> Self {
        Self {
            connection: Arc::new(Mutex::new(None)),
            use_session,
        }
    }

    async fn get_or_connect(&self) -> Result<Connection, String> {
        let mut lock = self.connection.lock().await;
        if let Some(conn) = &*lock {
            return Ok(conn.clone());
        }

        let conn = if self.use_session {
            Connection::session()
                .await
                .map_err(|e| format!("Failed to connect to D-Bus session bus: {e}"))?
        } else {
            Connection::system()
                .await
                .map_err(|e| format!("Failed to connect to D-Bus system bus: {e}"))?
        };

        *lock = Some(conn.clone());
        Ok(conn)
    }

    pub async fn is_daemon_available(&self) -> bool {
        let conn = match self.get_or_connect().await {
            Ok(c) => c,
            Err(_) => return false,
        };

        match DaemonProxy::new(&conn).await {
            Ok(proxy) => tokio::time::timeout(Duration::from_secs(1), proxy.get_version())
                .await
                .map(|r| r.is_ok())
                .unwrap_or(false),
            Err(_) => false,
        }
    }

    pub async fn reduce(&self, mode: ReduceMode) -> Result<ReduceResult, String> {
        let conn = self.get_or_connect().await?;
        let proxy = DaemonProxy::new(&conn)
            .await
            .map_err(|e| format!("Failed to create Daemon D-Bus proxy: {e}"))?;

        info!("Sending D-Bus Reduce request with mode '{}'", mode.as_str());

        let raw_json = proxy
            .reduce(mode.as_str())
            .await
            .map_err(|e| format!("Daemon rejected request: {e}"))?;

        let val: Value = serde_json::from_str(&raw_json)
            .map_err(|e| format!("Invalid JSON from daemon: {e}"))?;

        let bytes_freed = val["bytes_freed"].as_u64().unwrap_or(0);
        let duration_ms = val["duration_ms"].as_u64().unwrap_or(0);
        let message = val["message"]
            .as_str()
            .unwrap_or("Memory reduced successfully")
            .to_string();

        Ok(ReduceResult {
            bytes_freed,
            duration_ms,
            message,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReduceResult {
    pub bytes_freed: u64,
    pub duration_ms: u64,
    #[allow(dead_code)]
    pub message: String,
}
