use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesPage, PreferencesWindow};
use gtk4::{Adjustment, Align, SpinButton, Switch};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::AppConfig;

pub fn open_preferences_window(parent: &adw::ApplicationWindow, config: Rc<RefCell<AppConfig>>) {
    let window = PreferencesWindow::builder()
        .title("Settings — memreduct")
        .transient_for(parent)
        .modal(true)
        .default_width(480)
        .default_height(580)
        .build();

    let cfg_val = config.borrow().clone();

    // --- Page 1: General ---
    let page_general = PreferencesPage::builder()
        .title("General")
        .icon_name("preferences-system-symbolic")
        .build();

    let group_ui = PreferencesGroup::builder().title("Behavior").build();

    // Start Minimized
    let sw_minimized = Switch::builder()
        .active(cfg_val.start_minimized)
        .valign(Align::Center)
        .build();
    let row_minimized = ActionRow::builder()
        .title("Start minimized to tray")
        .subtitle("Launch app directly in the background system tray")
        .activatable_widget(&sw_minimized)
        .build();
    row_minimized.add_suffix(&sw_minimized);

    let cfg_min = config.clone();
    sw_minimized.connect_active_notify(move |sw| {
        cfg_min.borrow_mut().start_minimized = sw.is_active();
        let _ = cfg_min.borrow().save();
    });
    group_ui.add(&row_minimized);

    // Confirmation dialog
    let sw_confirm = Switch::builder()
        .active(cfg_val.show_reduct_confirmation)
        .valign(Align::Center)
        .build();
    let row_confirm = ActionRow::builder()
        .title("Confirm aggressive reduction")
        .subtitle("Show confirmation dialog before running Drop Caches (All/Slab)")
        .activatable_widget(&sw_confirm)
        .build();
    row_confirm.add_suffix(&sw_confirm);

    let cfg_conf = config.clone();
    sw_confirm.connect_active_notify(move |sw| {
        cfg_conf.borrow_mut().show_reduct_confirmation = sw.is_active();
        let _ = cfg_conf.borrow().save();
    });
    group_ui.add(&row_confirm);

    page_general.add(&group_ui);
    window.add(&page_general);

    // --- Page 2: Auto-reduce ---
    let page_auto = PreferencesPage::builder()
        .title("Auto-Reduce")
        .icon_name("system-run-symbolic")
        .build();

    let group_auto = PreferencesGroup::builder()
        .title("Automatic Optimization Triggers")
        .description("Conservative automated memory reduction (disabled by default)")
        .build();

    // Threshold trigger
    let sw_thresh = Switch::builder()
        .active(cfg_val.auto_reduce.enabled_threshold)
        .valign(Align::Center)
        .build();
    let row_thresh_enable = ActionRow::builder()
        .title("Enable threshold trigger")
        .subtitle("Trigger reduction when RAM utilization exceeds target percentage")
        .activatable_widget(&sw_thresh)
        .build();
    row_thresh_enable.add_suffix(&sw_thresh);

    let adj_thresh = Adjustment::new(
        cfg_val.auto_reduce.threshold_used_percent,
        50.0,
        98.0,
        1.0,
        5.0,
        0.0,
    );
    let spin_thresh = SpinButton::builder()
        .adjustment(&adj_thresh)
        .valign(Align::Center)
        .build();
    let row_thresh_val = ActionRow::builder()
        .title("Threshold (%)")
        .subtitle("RAM utilization limit (default 85%)")
        .activatable_widget(&spin_thresh)
        .build();
    row_thresh_val.add_suffix(&spin_thresh);

    let cfg_th = config.clone();
    sw_thresh.connect_active_notify(move |sw| {
        cfg_th.borrow_mut().auto_reduce.enabled_threshold = sw.is_active();
        let _ = cfg_th.borrow().save();
    });

    let cfg_th_val = config.clone();
    adj_thresh.connect_value_changed(move |adj| {
        cfg_th_val.borrow_mut().auto_reduce.threshold_used_percent = adj.value();
        let _ = cfg_th_val.borrow().save();
    });

    group_auto.add(&row_thresh_enable);
    group_auto.add(&row_thresh_val);

    // PSI trigger
    let sw_psi = Switch::builder()
        .active(cfg_val.auto_reduce.enabled_psi)
        .valign(Align::Center)
        .build();
    let row_psi_enable = ActionRow::builder()
        .title("Enable PSI pressure trigger")
        .subtitle("Trigger reduction when kernel memory pressure avg10 spikes")
        .activatable_widget(&sw_psi)
        .build();
    row_psi_enable.add_suffix(&sw_psi);

    let adj_psi = Adjustment::new(
        cfg_val.auto_reduce.psi_threshold_avg10,
        5.0,
        80.0,
        1.0,
        5.0,
        0.0,
    );
    let spin_psi = SpinButton::builder()
        .adjustment(&adj_psi)
        .valign(Align::Center)
        .build();
    let row_psi_val = ActionRow::builder()
        .title("PSI Pressure Threshold (avg10 %)")
        .subtitle("Stall percentage over 10 seconds")
        .activatable_widget(&spin_psi)
        .build();
    row_psi_val.add_suffix(&spin_psi);

    let cfg_psi = config.clone();
    sw_psi.connect_active_notify(move |sw| {
        cfg_psi.borrow_mut().auto_reduce.enabled_psi = sw.is_active();
        let _ = cfg_psi.borrow().save();
    });

    let cfg_psi_val = config.clone();
    adj_psi.connect_value_changed(move |adj| {
        cfg_psi_val.borrow_mut().auto_reduce.psi_threshold_avg10 = adj.value();
        let _ = cfg_psi_val.borrow().save();
    });

    group_auto.add(&row_psi_enable);
    group_auto.add(&row_psi_val);

    // Cooldown
    let adj_cool = Adjustment::new(
        (cfg_val.auto_reduce.cooldown_seconds / 60) as f64,
        1.0,
        60.0,
        1.0,
        5.0,
        0.0,
    );
    let spin_cool = SpinButton::builder()
        .adjustment(&adj_cool)
        .valign(Align::Center)
        .build();
    let row_cooldown = ActionRow::builder()
        .title("Minimum Cooldown (minutes)")
        .subtitle("Prevents thrashing by enforcing a wait interval between auto-reductions")
        .activatable_widget(&spin_cool)
        .build();
    row_cooldown.add_suffix(&spin_cool);

    let cfg_cool = config.clone();
    adj_cool.connect_value_changed(move |adj| {
        cfg_cool.borrow_mut().auto_reduce.cooldown_seconds = (adj.value() * 60.0) as u64;
        let _ = cfg_cool.borrow().save();
    });
    group_auto.add(&row_cooldown);

    page_auto.add(&group_auto);
    window.add(&page_auto);

    window.present();
}
