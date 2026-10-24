use clap::{Parser, Subcommand, ValueEnum};
use memreduct_core::mode::ReduceMode;
use memreduct_core::snapshot::MemorySnapshot;
use serde_json::Value;
use std::process::ExitCode;
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

#[derive(Parser)]
#[command(
    name = "memreduct",
    about = "Real-time Memory Monitor & Optimizer for CachyOS / Linux",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Use D-Bus session bus instead of system bus (development/testing)
    #[arg(long, global = true)]
    session: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current memory, swap, zram, and PSI status
    Status {
        /// Output status in raw JSON format
        #[arg(long)]
        json: bool,
    },

    /// Request memory reduction via memreductd daemon
    Reduce {
        /// Reduction mode (reclaim, pagecache, slab, all_caches, compact)
        #[arg(short, long, default_value = "reclaim")]
        mode: CliReduceMode,
    },

    /// Show system diagnostic and daemon connection info
    Info,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliReduceMode {
    Reclaim,
    Pagecache,
    Slab,
    AllCaches,
    Compact,
}

impl From<CliReduceMode> for ReduceMode {
    fn from(c: CliReduceMode) -> Self {
        match c {
            CliReduceMode::Reclaim => ReduceMode::Reclaim,
            CliReduceMode::Pagecache => ReduceMode::PageCache,
            CliReduceMode::Slab => ReduceMode::Slab,
            CliReduceMode::AllCaches => ReduceMode::AllCaches,
            CliReduceMode::Compact => ReduceMode::Compact,
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Status { json: false }) {
        Commands::Status { json } => handle_status(json),
        Commands::Reduce { mode } => handle_reduce(mode.into(), cli.session).await,
        Commands::Info => handle_info(cli.session).await,
    }
}

fn handle_status(json: bool) -> ExitCode {
    let snapshot = match MemorySnapshot::capture() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error capturing memory snapshot: {e}");
            return ExitCode::FAILURE;
        }
    };

    if json {
        match serde_json::to_string_pretty(&snapshot) {
            Ok(j) => {
                println!("{j}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("JSON serialization failed: {e}");
                ExitCode::FAILURE
            }
        }
    } else {
        print_status_table(&snapshot);
        ExitCode::SUCCESS
    }
}

fn print_status_table(s: &MemorySnapshot) {
    println!("=== memreduct-linux: Memory Status ===");
    println!(
        "Physical RAM: {:>6} MB used / {:>6} MB total ({:.1}% used, {:.1}% available)",
        s.used_mb(),
        s.total_mb(),
        s.used_percent(),
        s.available_percent()
    );
    println!(
        "              {:>6} MB free | {:>6} MB available | {:>6} MB cache/buffers",
        s.free_mb(),
        s.available_mb(),
        s.cache_mb()
    );

    if s.swap_total_mb() > 0 {
        println!(
            "Swap / zram:  {:>6} MB used / {:>6} MB total ({:.1}% used)",
            s.swap_used_mb(),
            s.swap_total_mb(),
            s.meminfo.swap_used_percent()
        );
    } else {
        println!("Swap / zram:  Not configured / 0 MB");
    }

    if let Some(zram) = &s.zram {
        for dev in &zram.devices {
            let orig_mb = dev.orig_data_bytes as f64 / (1024.0 * 1024.0);
            let compr_mb = dev.compr_data_bytes as f64 / (1024.0 * 1024.0);
            let saved_mb = dev.saved_bytes() as f64 / (1024.0 * 1024.0);
            println!(
                "zram ({:<5}):  {:.1} MB compressed into {:.1} MB (ratio: {:.2}x, saved: {:.1} MB)",
                dev.device_name,
                orig_mb,
                compr_mb,
                dev.compression_ratio(),
                saved_mb
            );
        }
    }

    if let Some(psi) = &s.psi {
        let some = psi.some.map(|m| m.avg10).unwrap_or(0.0);
        let full = psi.full.map(|m| m.avg10).unwrap_or(0.0);
        println!(
            "Pressure PSI: Level={:?} | some avg10={:.2}%, full avg10={:.2}%",
            psi.level(),
            some,
            full
        );
    }
}

async fn get_connection(session: bool) -> Result<Connection, zbus::Error> {
    if session {
        Connection::session().await
    } else {
        Connection::system().await
    }
}

async fn handle_reduce(mode: ReduceMode, session: bool) -> ExitCode {
    println!("Requesting reduction: {}...", mode.display_name());

    let conn = match get_connection(session).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to D-Bus: {e}");
            return ExitCode::FAILURE;
        }
    };

    let proxy = match DaemonProxy::new(&conn).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create Daemon proxy: {e}");
            eprintln!("Is memreductd running? Check: systemctl status memreductd");
            return ExitCode::FAILURE;
        }
    };

    match proxy.reduce(mode.as_str()).await {
        Ok(result_json) => {
            if let Ok(val) = serde_json::from_str::<Value>(&result_json) {
                let bytes_freed = val["bytes_freed"].as_u64().unwrap_or(0);
                let freed_mb = bytes_freed as f64 / (1024.0 * 1024.0);
                let duration_ms = val["duration_ms"].as_u64().unwrap_or(0);
                let msg = val["message"].as_str().unwrap_or("Reduction completed");
                println!("[SUCCESS] {msg}");
                println!("Result:   {:.2} MB freed in {} ms", freed_mb, duration_ms);
            } else {
                println!("[SUCCESS] {result_json}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[FAILED] Reduction request rejected: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn handle_info(session: bool) -> ExitCode {
    println!("=== memreduct-linux: Diagnostics ===");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Check daemon
    match get_connection(session).await {
        Ok(conn) => match DaemonProxy::new(&conn).await {
            Ok(proxy) => match proxy.get_version().await {
                Ok(ver) => println!("Daemon:  Connected (memreductd v{ver})"),
                Err(_) => println!("Daemon:  Not responding (service inactive)"),
            },
            Err(_) => println!("Daemon:  Not found on bus"),
        },
        Err(e) => println!("Daemon:  Cannot connect to D-Bus ({e})"),
    }

    // Check kernel features
    println!("Kernel /proc/meminfo:           Present");
    println!(
        "Kernel /proc/pressure/memory:   {}",
        if std::path::Path::new("/proc/pressure/memory").exists() {
            "Supported (PSI active)"
        } else {
            "Not available"
        }
    );
    println!(
        "cgroup v2 memory.reclaim:       {}",
        if std::path::Path::new("/sys/fs/cgroup/memory.reclaim").exists() {
            "Supported"
        } else {
            "Not available"
        }
    );
    println!(
        "zram support:                   {}",
        if std::path::Path::new("/sys/block/zram0").exists() {
            "Active (/dev/zram0 detected)"
        } else {
            "No zram block device detected"
        }
    );

    ExitCode::SUCCESS
}
