use ksni::menu::*;
use ksni::ToolTip;
use memreduct_core::mode::ReduceMode;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum TrayAction {
    ToggleWindow,
    OpenSettings,
    RequestReduce(ReduceMode),
    Quit,
}

pub struct MemTray {
    pub ram_percent: f64,
    pub tooltip_desc: String,
    pub tx: Sender<TrayAction>,
}

impl ksni::Tray for MemTray {
    fn id(&self) -> String {
        "memreduct-linux".to_string()
    }

    fn icon_name(&self) -> String {
        "utilities-system-monitor".to_string()
    }

    fn title(&self) -> String {
        format!("{:.0}%", self.ram_percent)
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "memreduct-linux".to_string(),
            description: self.tooltip_desc.clone(),
            icon_name: "utilities-system-monitor".to_string(),
            icon_pixmap: Vec::new(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayAction::ToggleWindow);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let tx1 = self.tx.clone();
        let tx2 = self.tx.clone();
        let tx3 = self.tx.clone();
        let tx_quit = self.tx.clone();
        let tx_modes = self.tx.clone();

        vec![
            StandardItem {
                label: format!("Memory Usage: {:.1}%", self.ram_percent),
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Reduce Now (Default)".into(),
                icon_name: "edit-clear".into(),
                activate: Box::new(move |_| {
                    let _ = tx1.send(TrayAction::RequestReduce(ReduceMode::Reclaim));
                }),
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Reduce Mode".into(),
                submenu: vec![
                    create_mode_item(&tx_modes, ReduceMode::Reclaim),
                    create_mode_item(&tx_modes, ReduceMode::PageCache),
                    create_mode_item(&tx_modes, ReduceMode::Slab),
                    create_mode_item(&tx_modes, ReduceMode::AllCaches),
                    create_mode_item(&tx_modes, ReduceMode::Compact),
                ],
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Show / Hide".into(),
                activate: Box::new(move |_| {
                    let _ = tx2.send(TrayAction::ToggleWindow);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Settings".into(),
                icon_name: "preferences-system".into(),
                activate: Box::new(move |_| {
                    let _ = tx3.send(TrayAction::OpenSettings);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(move |_| {
                    let _ = tx_quit.send(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn create_mode_item(tx: &Sender<TrayAction>, mode: ReduceMode) -> ksni::MenuItem<MemTray> {
    let tx_clone = tx.clone();
    StandardItem {
        label: mode.display_name().to_string(),
        activate: Box::new(move |_| {
            let _ = tx_clone.send(TrayAction::RequestReduce(mode));
        }),
        ..Default::default()
    }
    .into()
}
