//! World check page: run `lat_core::check::check_world` on a folder.

use eframe::egui;
use lat_core::check::{check_world, Report};

#[derive(Default)]
pub struct Page {
    world: String,
    report: Option<Report>,
}

impl Page {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("World folder:");
            ui.text_edit_singleline(&mut self.world);
            if ui.button("Check").clicked() {
                self.report = Some(check_world(std::path::Path::new(&self.world)));
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
            });
        }
    }
}
