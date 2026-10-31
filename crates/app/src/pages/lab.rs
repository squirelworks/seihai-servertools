//! Lab world page: show what the one-step start would run.
//!
//! Starting and stopping from the window is planned (`docs/NEXT.md`): run
//! the plan with `setup_core::run::Group` on a worker thread and show each
//! console's tail. Until then the page shows the plan and the command.

use eframe::egui;
use setup_core::lab::{plan, LabOptions};

#[derive(Default)]
pub struct Page {
    world: String,
    result: Option<String>,
}

impl Page {
    pub fn set_world(&mut self, world: &str) {
        self.world = world.to_string();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("World folder:");
            ui.text_edit_singleline(&mut self.world);
            if ui.button("Show plan").clicked() {
                let opts = LabOptions {
                    world: self.world.clone().into(),
                    ..Default::default()
                };
                self.result = Some(match plan(&opts) {
                    Ok(p) => p.to_string(),
                    Err(e) => format!("error: {e}"),
                });
            }
        });
        ui.label("Start it from a terminal with: seihai-servertools lab start --world <folder>");
        if let Some(text) = &self.result {
            ui.separator();
            ui.monospace(text);
        }
    }
}
