mod executor;
mod polkit;
mod service;

use std::error::Error;
use std::future::pending;
use tracing::info;
use zbus::connection;

use service::DaemonService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "memreductd=info,zbus=info".into()),
        )
        .init();

    info!("Starting memreductd v{}", env!("CARGO_PKG_VERSION"));

    // Check if --session flag is passed (useful for dev/test without root)
    let use_session_bus = std::env::args().any(|arg| arg == "--session");
    let service = DaemonService::new(use_session_bus);

    let _conn = if use_session_bus {
        info!("Connecting to D-Bus SESSION bus (development mode)");
        connection::Builder::session()?
            .name("org.memreduct.Daemon")?
            .serve_at("/org/memreduct/Daemon", service)?
            .build()
            .await?
    } else {
        info!("Connecting to D-Bus SYSTEM bus");
        connection::Builder::system()?
            .name("org.memreduct.Daemon")?
            .serve_at("/org/memreduct/Daemon", service)?
            .build()
            .await?
    };

    info!("memreductd service registered and listening at /org/memreduct/Daemon");

    // Wait forever until SIGINT/SIGTERM
    pending::<()>().await;

    Ok(())
}
