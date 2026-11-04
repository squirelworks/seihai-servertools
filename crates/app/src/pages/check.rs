//! World check page: run `lat_core::check::check_world` on a folder, plus
//! the setup checks of its server settings
//! (`setup_core::folder::check_world_config`).

use eframe::egui;
use lat_core::check::{check_world, Report};
use setup_core::folder::{check_world_config, ConfigFinding};

#[derive(Default)]
pub struct Page {
    world: String,
    report: Option<Report>,
    config: Vec<ConfigFinding>,
}

impl Page {
    pub fn set_world(&mut self, world: &str) {
        self.world = world.to_string();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("World folder:");
            ui.text_edit_singleline(&mut self.world);
            if ui.button("Check").clicked() {
                let world = std::path::Path::new(&self.world);
                self.report = Some(check_world(world));
                self.config = check_world_config(world);
            }
        });
        if let Some(r) = &self.report {
            ui.separator();
            ui.label(format!(
                "{} levels, {} gmaps, {} ganis: {} errors, {} findings",
                r.levels,
                r.gmaps,
                r.ganis,
                r.errors(),
                r.findings.len()
            ));
            egui::ScrollArea::vertical().show(ui, |ui| {
                for f in &r.findings {
                    ui.monospace(format!("{:?}  {}  {}", f.severity, f.file, f.text));
                }
                for c in &self.config {
                    let key = c.finding.key.as_deref().unwrap_or("");
                    ui.monospace(format!(
                        "{:?}  {}  {key}  {}",
                        c.finding.severity, c.file, c.finding.text
                    ));
                }
            });
        }
    }
}
