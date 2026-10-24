mod chart;
mod client;
mod config;
mod history;
mod preferences;
mod tray;
mod window;

use adw::prelude::*;
use gtk4::glib;
use ksni::blocking::TrayMethods;
use memreduct_core::policy::AutoReduceEngine;
use memreduct_core::snapshot::MemorySnapshot;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::time::SystemTime;
use tracing::{info, warn};

use client::DaemonClient;
use config::AppConfig;
use tray::{MemTray, TrayAction};
use window::{create_main_window, update_ui_with_snapshot};

const APP_ID: &str = "org.memreduct.MemReduct";

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "memreduct_gui=info".into()),
        )
        .init();

    info!("Starting memreduct-gui");

    let app = adw::Application::builder().application_id(APP_ID).build();

    let use_session = std::env::args().any(|a| a == "--session");
    let client = DaemonClient::new(use_session);
    let config = Rc::new(RefCell::new(AppConfig::load()));

    // Channel for Tray -> GTK UI communication
    let (tx_tray, rx_tray) = channel::<TrayAction>();

    let cfg_tray = config.borrow().clone();

    // Spawn ksni tray in a separate background thread if tray is enabled
    let tray_handle = if cfg_tray.show_tray {
        let initial_snapshot = MemorySnapshot::capture().ok();
        let initial_pct = initial_snapshot
            .as_ref()
            .map(|s| s.used_percent())
            .unwrap_or(0.0);
        let initial_tooltip = initial_snapshot
            .as_ref()
            .map(|s| s.tray_tooltip())
            .unwrap_or_else(|| "memreduct-linux".to_string());

        let tray = MemTray {
            ram_percent: initial_pct,
            tooltip_desc: initial_tooltip,
            tx: tx_tray,
        };

        match tray.spawn() {
            Ok(handle) => {
                info!("StatusNotifierItem tray icon initialized successfully");
                Some(handle)
            }
            Err(e) => {
                warn!("Could not spawn StatusNotifierItem tray: {e}");
                None
            }
        }
    } else {
        None
    };

    let rx_tray_cell = Rc::new(RefCell::new(rx_tray));

    app.connect_activate(move |application| {
        let widgets = Rc::new(create_main_window(
            application,
            config.clone(),
            client.clone(),
        ));

        let auto_engine = Rc::new(RefCell::new(AutoReduceEngine::new(
            config.borrow().auto_reduce.clone(),
        )));

        // Handle start_minimized
        if !config.borrow().start_minimized {
            widgets.window.present();
        }

        let widgets_timer = widgets.clone();
        let client_timer = client.clone();
        let auto_timer = auto_engine.clone();
        let tray_handle_timer = tray_handle.clone();
        let rx_tray_timer = rx_tray_cell.clone();
        let config_timer = config.clone();

        // 1-second periodic tick
        glib::timeout_add_seconds_local(1, move || {
            // Process any pending Tray actions
            while let Ok(action) = rx_tray_timer.borrow_mut().try_recv() {
                match action {
                    TrayAction::ToggleWindow => {
                        if widgets_timer.window.is_visible() {
                            widgets_timer.window.hide();
                        } else {
                            widgets_timer.window.present();
                        }
                    }
                    TrayAction::OpenSettings => {
                        preferences::open_preferences_window(
                            &widgets_timer.window,
                            config_timer.clone(),
                        );
                    }
                    TrayAction::RequestReduce(mode) => {
                        let client_req = client_timer.clone();
                        let toast_req = widgets_timer.toast_overlay.clone();
                        glib::MainContext::default().spawn_local(async move {
                            match client_req.reduce(mode).await {
                                Ok(res) => {
                                    let freed_mb = res.bytes_freed as f64 / (1024.0 * 1024.0);
                                    let toast = adw::Toast::new(&format!(
                                        "{}: {:.1} MB freed",
                                        mode.display_name(),
                                        freed_mb
                                    ));
                                    toast_req.add_toast(toast);
                                }
                                Err(e) => {
                                    let toast = adw::Toast::new(&format!("Reduce error: {e}"));
                                    toast_req.add_toast(toast);
                                }
                            }
                        });
                    }
                    TrayAction::Quit => {
                        std::process::exit(0);
                    }
                }
            }

            // Capture memory state
            if let Ok(snapshot) = MemorySnapshot::capture() {
                update_ui_with_snapshot(&widgets_timer, &snapshot);

                // Update tray icon & tooltip
                if let Some(tray_h) = &tray_handle_timer {
                    let pct = snapshot.used_percent();
                    let tooltip = snapshot.tray_tooltip();
                    let _ = tray_h.update(move |t: &mut MemTray| {
                        t.ram_percent = pct;
                        t.tooltip_desc = tooltip;
                    });
                }

                // Auto-Reduce evaluation
                let now_secs = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                // Keep engine policy synced with current config
                auto_timer.borrow_mut().policy = config_timer.borrow().auto_reduce.clone();

                if let Some((mode, trigger)) = auto_timer.borrow().check(&snapshot, now_secs) {
                    info!("Auto-reduce triggered: {:?} with mode {:?}", trigger, mode);
                    auto_timer.borrow_mut().record_reduce(now_secs);

                    let client_auto = client_timer.clone();
                    let toast_auto = widgets_timer.toast_overlay.clone();

                    glib::MainContext::default().spawn_local(async move {
                        match client_auto.reduce(mode).await {
                            Ok(res) => {
                                let freed_mb = res.bytes_freed as f64 / (1024.0 * 1024.0);
                                let toast = adw::Toast::new(&format!(
                                    "Auto-Reduce: {:.1} MB freed via {}",
                                    freed_mb,
                                    mode.display_name()
                                ));
                                toast_auto.add_toast(toast);
                            }
                            Err(e) => {
                                warn!("Auto-reduce failed: {e}");
                            }
                        }
                    });
                }
            }

            // Check daemon connectivity every 5 ticks
            let banner_w = widgets_timer.daemon_banner.clone();
            let client_check = client_timer.clone();
            glib::MainContext::default().spawn_local(async move {
                let available = client_check.is_daemon_available().await;
                banner_w.set_revealed(!available);
            });

            glib::ControlFlow::Continue
        });
    });

    app.run()
}
