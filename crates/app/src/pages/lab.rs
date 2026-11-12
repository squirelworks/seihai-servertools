//! Lab world page: show what the one-step start would run, then start and
//! stop it from the window. The processes run on a worker thread
//! (`setup_core::lab_run::LabRun`, the same order as
//! `seihai-servertools lab start`); this page shows each one's state and
//! the tail of its console. Leaving the window stops them. An optional
//! `seihai-gserver` world folder starts next to the fork's server folders.

use std::time::Duration;

use eframe::egui::{self, RichText};
use setup_core::lab::{plan, LabOptions};
use setup_core::lab_run::{LabRun, Phase};

/// Console lines shown per server.
const TAIL_LINES: usize = 12;

#[derive(Default)]
pub struct Page {
    world: String,
    picker: super::picker::Picker,
    /// A `seihai-gserver` world folder to start too; empty for none.
    rust_world: String,
    rust_picker: super::picker::Picker,
    result: Option<String>,
    run: Option<LabRun>,
}

impl Page {
    pub fn set_world(&mut self, world: &str) {
        self.world = world.to_string();
    }

    fn options(&self) -> LabOptions {
        let rust = self.rust_world.trim();
        LabOptions {
            world: self.world.clone().into(),
            rust_worlds: if rust.is_empty() {
                Vec::new()
            } else {
                vec![rust.into()]
            },
            ..Default::default()
        }
    }

    fn running(&self) -> bool {
        self.run.as_ref().is_some_and(|r| !r.is_finished())
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let busy = self.running();
        ui.horizontal(|ui| {
            ui.label("World folder:");
            ui.add_enabled(!busy, egui::TextEdit::singleline(&mut self.world));
            self.picker.button(ui, !busy, &mut self.world);
            if ui
                .add_enabled(!busy, egui::Button::new("Show plan"))
                .clicked()
            {
                self.result = Some(match plan(&self.options()) {
                    Ok(p) => p.to_string(),
                    Err(e) => format!("error: {e}"),
                });
            }
            if ui.add_enabled(!busy, egui::Button::new("Start")).clicked() {
                match plan(&self.options()) {
                    Ok(p) => {
                        self.result = Some(p.to_string());
                        // A finished run is dropped here; it has nothing
                        // left to stop.
                        self.run = Some(LabRun::start(p));
                    }
                    Err(e) => self.result = Some(format!("error: {e}")),
                }
            }
            if ui.add_enabled(busy, egui::Button::new("Stop")).clicked() {
                if let Some(r) = &self.run {
                    r.stop();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("seihai-gserver world (optional):");
            ui.add_enabled(!busy, egui::TextEdit::singleline(&mut self.rust_world));
            self.rust_picker.button(ui, !busy, &mut self.rust_world);
        });
        ui.label(
            "Or from a terminal: seihai-servertools lab start --world <folder> [--rust <folder>]",
        );

        if let Some(run) = &self.run {
            let s = run.status();
            ui.separator();
            let (text, color) = match &s.phase {
                Phase::Starting => ("Starting...".to_string(), None),
                Phase::Running => (
                    "Running".to_string(),
                    Some(egui::Color32::from_rgb(40, 160, 70)),
                ),
                Phase::Stopping => ("Stopping...".to_string(), None),
                Phase::Stopped => ("Stopped".to_string(), None),
                Phase::Failed(_) => (
                    "Start failed".to_string(),
                    Some(egui::Color32::from_rgb(200, 60, 50)),
                ),
            };
            let mut heading = RichText::new(text).strong();
            if let Some(c) = color {
                heading = heading.color(c);
            }
            ui.label(heading);
            if let Phase::Failed(why) = &s.phase {
                ui.monospace(why);
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for line in &s.log {
                    ui.monospace(line);
                }
                for (p, (_, tail)) in s.procs.iter().zip(run.tails(TAIL_LINES)) {
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{} (pid {}): {}  {}",
                            p.label,
                            p.pid,
                            p.exited.as_deref().unwrap_or("running"),
                            p.console.display()
                        ))
                        .strong(),
                    );
                    ui.monospace(if tail.is_empty() {
                        "(no output yet)"
                    } else {
                        &tail
                    });
                }
            });
            if !run.is_finished() {
                ui.ctx().request_repaint_after(Duration::from_millis(500));
            }
        } else if let Some(text) = &self.result {
            ui.separator();
            ui.monospace(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_world_field_becomes_an_option() {
        let mut p = Page::default();
        p.set_world("w");
        assert!(p.options().rust_worlds.is_empty());
        p.rust_world = "  rw ".into();
        assert_eq!(p.options().rust_worlds, [std::path::PathBuf::from("rw")]);
    }
}
