//! "Browse..." next to a world folder field: a small window that walks the
//! folders (`setup_core::browse`) and fills the field. No native dialog, so
//! it works the same on every system and needs nothing extra to build.

use eframe::egui;
use setup_core::browse::{self, Listing};

#[derive(Default, Clone)]
pub struct Picker {
    /// The open listing while the window is up.
    listing: Option<Listing>,
    error: String,
}

impl Picker {
    fn go(&mut self, path: &std::path::Path) {
        match browse::list(path) {
            Ok(l) => {
                self.listing = Some(l);
                self.error.clear();
            }
            Err(e) => self.error = e,
        }
    }

    /// The "Browse..." button. True on the frame a folder was picked.
    pub fn button(&mut self, ui: &mut egui::Ui, enabled: bool, value: &mut String) -> bool {
        if ui
            .add_enabled(enabled, egui::Button::new("Browse..."))
            .clicked()
        {
            self.go(&browse::start(value));
        }
        self.window(ui.ctx(), value)
    }

    fn window(&mut self, ctx: &egui::Context, value: &mut String) -> bool {
        let Some(listing) = self.listing.clone() else {
            return false;
        };
        let mut open = true;
        let mut picked = false;
        let mut go_to = None;
        egui::Window::new("Pick a folder")
            .open(&mut open)
            .collapsible(false)
            .default_width(480.0)
            .show(ctx, |ui| {
                ui.label(listing.path.display().to_string());
                if listing.world {
                    ui.label("This is a world folder.");
                }
                ui.horizontal(|ui| {
                    if let Some(parent) = &listing.parent {
                        if ui.button("Up").clicked() {
                            go_to = Some(parent.clone());
                        }
                    }
                    if ui.button("Use this folder").clicked() {
                        *value = listing.path.display().to_string();
                        picked = true;
                    }
                });
                if !self.error.is_empty() {
                    ui.label(&self.error);
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        if listing.subs.is_empty() {
                            ui.label("No folders here.");
                        }
                        for sub in &listing.subs {
                            let text = if sub.world {
                                format!("{} (world folder)", sub.name)
                            } else {
                                sub.name.clone()
                            };
                            if ui.button(text).clicked() {
                                go_to = Some(listing.path.join(&sub.name));
                            }
                        }
                    });
            });
        if let Some(p) = go_to {
            self.go(&p);
        }
        if picked || !open {
            self.listing = None;
            self.error.clear();
        }
        picked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn going_into_a_folder_lists_it_and_a_bad_one_keeps_the_last() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("w/servers/default/config")).unwrap();
        std::fs::write(
            d.path().join("w/servers/default/config/serveroptions.txt"),
            "",
        )
        .unwrap();
        let mut p = Picker::default();
        p.go(d.path());
        let l = p.listing.clone().unwrap();
        assert_eq!(l.subs.len(), 1);
        assert!(l.subs[0].world);
        p.go(&d.path().join("missing"));
        assert!(!p.error.is_empty());
        assert_eq!(
            p.listing.unwrap().path,
            d.path(),
            "still on the last folder"
        );
    }
}
