use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar, PreferencesGroup, Toast, ToastOverlay};
use gtk4::{glib, Align, Box, Button, DropDown, Label, LevelBar, Orientation, StringList};
use memreduct_core::mode::ReduceMode;
use memreduct_core::snapshot::MemorySnapshot;
use std::cell::RefCell;
use std::rc::Rc;

use crate::chart::create_chart_widget;
use crate::client::DaemonClient;
use crate::config::AppConfig;
use crate::history::HistoryBuffer;
use crate::preferences::open_preferences_window;

pub struct MainWindowWidgets {
    pub window: ApplicationWindow,
    pub toast_overlay: ToastOverlay,
    pub daemon_banner: adw::Banner,
    pub ram_level: LevelBar,
    pub ram_label: Label,
    pub ram_details_label: Label,
    pub swap_level: LevelBar,
    pub swap_label: Label,
    pub zram_label: Label,
    pub psi_label: Label,
    pub history: Rc<RefCell<HistoryBuffer>>,
    pub chart_area: gtk4::DrawingArea,
}

pub fn create_main_window(
    app: &adw::Application,
    config: Rc<RefCell<AppConfig>>,
    client: DaemonClient,
) -> MainWindowWidgets {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("memreduct-linux")
        .default_width(460)
        .default_height(620)
        .build();

    let toast_overlay = ToastOverlay::new();

    let main_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .build();

    // 1. HeaderBar
    let header = HeaderBar::new();
    let settings_btn = Button::builder()
        .icon_name("preferences-system-symbolic")
        .tooltip_text("Settings")
        .build();

    let win_weak = window.downgrade();
    let cfg_clone = config.clone();
    settings_btn.connect_clicked(move |_| {
        if let Some(win) = win_weak.upgrade() {
            open_preferences_window(&win, cfg_clone.clone());
        }
    });
    header.pack_end(&settings_btn);
    main_box.append(&header);

    // 2. Daemon status banner (hidden if daemon active)
    let daemon_banner = adw::Banner::builder()
        .title("⚠️ memreductd inactive — Running in Monitor-Only mode")
        .revealed(false)
        .build();
    main_box.append(&daemon_banner);

    // 3. Content scrolled / padded
    let content_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_start(18)
        .margin_end(18)
        .margin_top(10)
        .margin_bottom(18)
        .build();

    // --- Chart Section ---
    let history = Rc::new(RefCell::new(HistoryBuffer::new(60)));
    let chart_area = create_chart_widget(history.clone());

    let chart_group = PreferencesGroup::builder()
        .title("Memory History (Last 60 Seconds)")
        .build();
    chart_group.add(&chart_area);
    content_box.append(&chart_group);

    // --- RAM Section ---
    let ram_group = PreferencesGroup::builder().title("Physical Memory").build();

    let ram_level = LevelBar::builder()
        .min_value(0.0)
        .max_value(1.0)
        .value(0.5)
        .margin_bottom(6)
        .build();

    let ram_label = Label::builder()
        .label("RAM: 0 MB / 0 MB (0.0% used)")
        .halign(Align::Start)
        .css_classes(["title-4"])
        .build();

    let ram_details_label = Label::builder()
        .label("Available: 0 MB | Free: 0 MB | Cache: 0 MB")
        .halign(Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();

    ram_group.add(&ram_label);
    ram_group.add(&ram_level);
    ram_group.add(&ram_details_label);
    content_box.append(&ram_group);

    // --- Swap & zram Section ---
    let swap_group = PreferencesGroup::builder()
        .title("Swap & zram (CachyOS)")
        .build();

    let swap_level = LevelBar::builder()
        .min_value(0.0)
        .max_value(1.0)
        .value(0.0)
        .margin_bottom(4)
        .build();

    let swap_label = Label::builder()
        .label("Swap: 0 MB / 0 MB (0.0% used)")
        .halign(Align::Start)
        .build();

    let zram_label = Label::builder()
        .label("zram: Checking...")
        .halign(Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();

    swap_group.add(&swap_label);
    swap_group.add(&swap_level);
    swap_group.add(&zram_label);
    content_box.append(&swap_group);

    // --- Pressure Stall Information (PSI) Section ---
    let psi_group = PreferencesGroup::builder()
        .title("Pressure Stall Information (PSI)")
        .build();

    let psi_label = Label::builder()
        .label("Pressure Level: Low | some avg10: 0.00%")
        .halign(Align::Start)
        .build();

    psi_group.add(&psi_label);
    content_box.append(&psi_group);

    // --- Action Section ---
    let action_group = PreferencesGroup::builder()
        .title("Memory Reduction")
        .build();

    let action_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();

    let modes_list = StringList::new(&[
        "Working Set Reclaim (cgroup v2)",
        "Page Cache (drop_caches 1)",
        "Dentry/Inode Slab (drop_caches 2)",
        "All Caches (Aggressive)",
        "Memory Compaction",
    ]);

    let mode_dropdown = DropDown::builder()
        .model(&modes_list)
        .selected(0)
        .hexpand(true)
        .build();

    let reduce_btn = Button::builder()
        .label("Reduce Now")
        .css_classes(["suggested-action", "pill"])
        .build();

    action_box.append(&mode_dropdown);
    action_box.append(&reduce_btn);
    action_group.add(&action_box);
    content_box.append(&action_group);

    main_box.append(&content_box);
    toast_overlay.set_child(Some(&main_box));
    window.set_content(Some(&toast_overlay));

    // Connect Reduce Button
    let win_weak_btn = window.downgrade();
    let client_clone = client.clone();
    let toast_clone = toast_overlay.clone();
    let dropdown_clone = mode_dropdown.clone();
    let reduce_btn_clone = reduce_btn.clone();
    let cfg_reduce = config.clone();

    reduce_btn.connect_clicked(move |_| {
        let selected_idx = dropdown_clone.selected();
        let mode = match selected_idx {
            0 => ReduceMode::Reclaim,
            1 => ReduceMode::PageCache,
            2 => ReduceMode::Slab,
            3 => ReduceMode::AllCaches,
            4 => ReduceMode::Compact,
            _ => ReduceMode::Reclaim,
        };

        let confirm_needed = cfg_reduce.borrow().show_reduct_confirmation && mode.is_aggressive();

        let do_reduce = {
            let client = client_clone.clone();
            let toast = toast_clone.clone();
            let btn = reduce_btn_clone.clone();
            move || {
                btn.set_sensitive(false);
                btn.set_label("Optimizing...");

                let btn_done = btn.clone();
                let toast_done = toast.clone();

                glib::MainContext::default().spawn_local(async move {
                    match client.reduce(mode).await {
                        Ok(res) => {
                            let freed_mb = res.bytes_freed as f64 / (1024.0 * 1024.0);
                            let toast_msg = Toast::new(&format!(
                                "{}: {:.1} MB freed ({} ms)",
                                mode.display_name(),
                                freed_mb,
                                res.duration_ms
                            ));
                            toast_done.add_toast(toast_msg);
                        }
                        Err(e) => {
                            let toast_msg = Toast::new(&format!("Reduction failed: {e}"));
                            toast_done.add_toast(toast_msg);
                        }
                    }
                    btn_done.set_sensitive(true);
                    btn_done.set_label("Reduce Now");
                });
            }
        };

        if confirm_needed {
            if let Some(win) = win_weak_btn.upgrade() {
                let alert = adw::AlertDialog::builder()
                    .heading("Confirm Aggressive Memory Reduction?")
                    .body(format!(
                        "You selected '{}'.\n\nDropping caches forces the kernel to reload filesystem data from disk upon next access, which may temporarily increase disk I/O.",
                        mode.display_name()
                    ))
                    .build();

                alert.add_response("cancel", "Cancel");
                alert.add_response("proceed", "Proceed");
                alert.set_close_response("cancel");
                alert.set_default_response(Some("proceed"));

                alert.choose(Some(&win), gtk4::gio::Cancellable::NONE, move |choice| {
                    if choice == "proceed" {
                        do_reduce();
                    }
                });
            }
        } else {
            do_reduce();
        }
    });

    MainWindowWidgets {
        window,
        toast_overlay,
        daemon_banner,
        ram_level,
        ram_label,
        ram_details_label,
        swap_level,
        swap_label,
        zram_label,
        psi_label,
        history,
        chart_area,
    }
}

pub fn update_ui_with_snapshot(widgets: &MainWindowWidgets, snapshot: &MemorySnapshot) {
    let used_pct = snapshot.used_percent();
    widgets.ram_level.set_value(used_pct / 100.0);
    widgets.ram_label.set_text(&format!(
        "RAM: {} MB / {} MB ({:.1}% used)",
        snapshot.used_mb(),
        snapshot.total_mb(),
        used_pct
    ));
    widgets.ram_details_label.set_text(&format!(
        "Available: {} MB | Free: {} MB | Cache/Buffers: {} MB",
        snapshot.available_mb(),
        snapshot.free_mb(),
        snapshot.cache_mb()
    ));

    // Update history buffer & redraw chart
    widgets.history.borrow_mut().push(used_pct);
    widgets.chart_area.queue_draw();

    // Swap
    let swap_pct = snapshot.meminfo.swap_used_percent();
    widgets.swap_level.set_value(swap_pct / 100.0);
    widgets.swap_label.set_text(&format!(
        "Swap: {} MB / {} MB ({:.1}% used)",
        snapshot.swap_used_mb(),
        snapshot.swap_total_mb(),
        swap_pct
    ));

    // zram
    if let Some(zram) = &snapshot.zram {
        let orig_mb = zram.total_orig_bytes() as f64 / (1024.0 * 1024.0);
        let compr_mb = zram.total_compr_bytes() as f64 / (1024.0 * 1024.0);
        widgets.zram_label.set_text(&format!(
            "zram: {:.1} MB compressed to {:.1} MB (ratio: {:.2}x, saved: {:.1} MB)",
            orig_mb,
            compr_mb,
            zram.average_compression_ratio(),
            orig_mb - compr_mb
        ));
    } else {
        widgets.zram_label.set_text("zram: Not active");
    }

    // PSI
    if let Some(psi) = &snapshot.psi {
        let some = psi.some.map(|m| m.avg10).unwrap_or(0.0);
        let full = psi.full.map(|m| m.avg10).unwrap_or(0.0);
        widgets.psi_label.set_text(&format!(
            "Level: {:?} | some avg10: {:.2}% | full avg10: {:.2}%",
            psi.level(),
            some,
            full
        ));
    } else {
        widgets
            .psi_label
            .set_text("PSI: Kernel PSI disabled or unavailable");
    }
}
