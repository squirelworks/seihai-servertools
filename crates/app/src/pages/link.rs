//! The folder or RC login an editor page asks for.
//!
//! Typing only. Opening and saving go through `lat-core`.

use eframe::egui;
use lat_core::remote::FileSource;

/// What the owner typed. The password stays in this form until a call
/// logs in; the page does not print it.
#[derive(Clone, Default)]
pub struct Where {
    pub folder: String,
    pub use_rc: bool,
    pub addr: String,
    pub account: String,
    pub password: String,
    pub dir: String,
    pub path: String,
}

impl Where {
    pub fn set_folder(&mut self, folder: &str) {
        self.folder = folder.to_string();
    }

    /// The source to pass to `lat-core`, or why the form is not ready.
    pub fn source(&self) -> Result<FileSource, String> {
        if self.use_rc {
            let addr = self.addr.trim();
            if addr.is_empty() {
                return Err("Type the RC address (host:port).".into());
            }
            if self.account.trim().is_empty() {
                return Err("Type the RC account.".into());
            }
            Ok(FileSource::Rc {
                addr: addr.to_string(),
                account: self.account.trim().to_string(),
                password: self.password.clone(),
            })
        } else {
            let folder = self.folder.trim();
            if folder.is_empty() {
                return Err("Type a world folder, or switch to an RC login.".into());
            }
            Ok(FileSource::Folder(folder.into()))
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(
            &mut self.use_rc,
            "Live server (RC file link). Leave this off to use a folder on this machine.",
        );
        if self.use_rc {
            ui.horizontal(|ui| {
                ui.label("RC address:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.addr)
                        .hint_text("127.0.0.1:<port>")
                        .desired_width(220.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Account:");
                ui.add(egui::TextEdit::singleline(&mut self.account).desired_width(160.0));
                ui.label("Password:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.password)
                        .password(true)
                        .desired_width(160.0),
                );
            });
            ui.label("The password is sent only to the address above.");
        } else {
            ui.horizontal(|ui| {
                ui.label("World folder:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.folder)
                        .hint_text("path to the world")
                        .desired_width(360.0),
                );
            });
        }
        ui.horizontal(|ui| {
            ui.label("Folder on the server:");
            ui.add(
                egui::TextEdit::singleline(&mut self.dir)
                    .hint_text("levels")
                    .desired_width(200.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label("File:");
            ui.add(
                egui::TextEdit::singleline(&mut self.path)
                    .hint_text("levels/room.nw")
                    .desired_width(280.0),
            );
        });
    }
}
