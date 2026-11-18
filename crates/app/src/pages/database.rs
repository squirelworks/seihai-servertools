//! NPC-server database page: database NPCs, classes and weapons over an
//! NPC Control (NC) login (`lat_core::ncdb`).
//!
//! Log in with an NC account, list what the server has, pick one, edit its
//! script and save. A save is kept only when the read that follows shows
//! the text sent. Classes and weapons can be added (type a new name) and
//! deleted; an NPC can be reset when the server supports it.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use eframe::egui;
use lat_core::ncdb::{self, Login, NcLists, Target};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Npcs,
    Classes,
    Weapons,
}

enum Msg {
    List(Result<NcLists, String>),
    Read(Target, Result<ncdb::Script, String>),
    Done(Result<String, String>),
}

pub struct Page {
    addr: String,
    account: String,
    password: String,
    kind: Kind,
    lists: Option<NcLists>,
    /// The object open in the editor, and its text and image.
    open: Option<Target>,
    script: String,
    image: String,
    /// A new class or weapon name.
    new_name: String,
    checked: Vec<String>,
    message: String,
    pending: Option<Receiver<Msg>>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            addr: String::new(),
            account: String::new(),
            password: String::new(),
            kind: Kind::Npcs,
            lists: None,
            open: None,
            script: String::new(),
            image: String::new(),
            new_name: String::new(),
            checked: Vec::new(),
            message: String::new(),
            pending: None,
        }
    }
}

impl Page {
    fn login(&self) -> Login {
        Login {
            addr: self.addr.clone(),
            account: self.account.clone(),
            password: self.password.clone(),
            timeout: Duration::from_secs(10),
        }
    }

    fn spawn(&mut self, ctx: &egui::Context, job: impl FnOnce(Login) -> Msg + Send + 'static) {
        let login = self.login();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(job(login));
        });
        self.pending = Some(rx);
        self.message = "Working...".into();
        ctx.request_repaint();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Edit what the NPC-server keeps: database NPCs, classes and weapons. This logs in as NPC Control, so the account needs NC rights on that server. A save counts only when reading it back shows the same script.");
        ui.horizontal(|ui| {
            ui.label("NC address:");
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
        let busy = self.pending.is_some();
        ui.horizontal(|ui| {
            if ui.add_enabled(!busy, egui::Button::new("List")).clicked() {
                self.spawn(ui.ctx(), |l| Msg::List(ncdb::list(&l)));
            }
            ui.separator();
            ui.selectable_value(&mut self.kind, Kind::Npcs, "NPCs");
            ui.selectable_value(&mut self.kind, Kind::Classes, "Classes");
            ui.selectable_value(&mut self.kind, Kind::Weapons, "Weapons");
        });
        self.poll(ui);
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        ui.separator();
        self.show_list(ui, busy);
        ui.separator();
        self.show_editor(ui, busy);
    }

    fn poll(&mut self, ui: &egui::Ui) {
        let Some(rx) = &self.pending else {
            return;
        };
        match rx.try_recv() {
            Ok(msg) => {
                self.pending = None;
                self.take(msg);
            }
            Err(mpsc::TryRecvError::Empty) => {
                ui.ctx().request_repaint_after(Duration::from_millis(200));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.message = "The NC call stopped.".into();
            }
        }
    }

    fn show_list(&mut self, ui: &mut egui::Ui, busy: bool) {
        let Some(lists) = &self.lists else {
            ui.label("List to see the server's database.");
            return;
        };
        let mut pick: Option<Target> = None;
        egui::ScrollArea::vertical()
            .id_salt("db-list")
            .max_height(160.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| match self.kind {
                    Kind::Npcs => {
                        for (id, row) in &lists.npcs {
                            let text = format!(
                                "{} ({}{})",
                                row.name.as_deref().unwrap_or("?"),
                                id,
                                row.level
                                    .as_deref()
                                    .filter(|l| !l.is_empty())
                                    .map(|l| format!(", {l}"))
                                    .unwrap_or_default()
                            );
                            let t = Target::Npc(*id);
                            if ui
                                .selectable_label(self.open == Some(t.clone()), text)
                                .clicked()
                            {
                                pick = Some(t);
                            }
                        }
                    }
                    Kind::Classes => {
                        for n in &lists.classes {
                            let t = Target::Class(n.clone());
                            if ui
                                .selectable_label(self.open == Some(t.clone()), n)
                                .clicked()
                            {
                                pick = Some(t);
                            }
                        }
                    }
                    Kind::Weapons => {
                        for n in &lists.weapons {
                            let t = Target::Weapon(n.clone());
                            if ui
                                .selectable_label(self.open == Some(t.clone()), n)
                                .clicked()
                            {
                                pick = Some(t);
                            }
                        }
                    }
                });
            });
        if self.kind != Kind::Npcs {
            ui.horizontal(|ui| {
                ui.label("New name:");
                ui.add(egui::TextEdit::singleline(&mut self.new_name).desired_width(200.0));
                if ui
                    .add_enabled(!self.new_name.trim().is_empty(), egui::Button::new("New"))
                    .clicked()
                {
                    let n = self.new_name.trim().to_string();
                    self.open = Some(match self.kind {
                        Kind::Classes => Target::Class(n),
                        _ => Target::Weapon(n),
                    });
                    self.script.clear();
                    self.image.clear();
                    self.checked.clear();
                    self.message = "New: type the script, then Save.".into();
                }
            });
        }
        if let Some(t) = pick {
            if !busy {
                self.spawn(ui.ctx(), move |l| {
                    let r = ncdb::read(&l, &t);
                    Msg::Read(t, r)
                });
            }
        }
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, busy: bool) {
        let Some(open) = self.open.clone() else {
            ui.label("Pick one to edit its script.");
            return;
        };
        let what = match &open {
            Target::Npc(id) => format!("NPC {id}"),
            Target::Class(n) => format!("class {n}"),
            Target::Weapon(n) => format!("weapon {n}"),
        };
        ui.label(format!("Editing {what}."));
        if matches!(open, Target::Weapon(_)) {
            ui.horizontal(|ui| {
                ui.label("Image:");
                ui.add(egui::TextEdit::singleline(&mut self.image).desired_width(220.0));
            });
        }
        ui.horizontal(|ui| {
            if ui.button("Check").clicked() {
                self.checked = super::npcs::check_lines(&self.script);
            }
            if ui.add_enabled(!busy, egui::Button::new("Save")).clicked() {
                let (t, text, image) = (open.clone(), self.script.clone(), self.image.clone());
                self.spawn(ui.ctx(), move |l| Msg::Done(ncdb::save(&l, &t, &text, &image)));
            }
            let action = if matches!(open, Target::Npc(_)) {
                "Reset"
            } else {
                "Delete"
            };
            if ui
                .add_enabled(!busy, egui::Button::new(action))
                .on_hover_text("A reset starts the NPC again; a delete removes the class or weapon from the server.")
                .clicked()
            {
                let t = open.clone();
                self.spawn(ui.ctx(), move |l| Msg::Done(ncdb::remove_or_reset(&l, &t)));
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("db-script")
            .max_height(320.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.script)
                        .code_editor()
                        .desired_rows(16)
                        .desired_width(f32::INFINITY),
                );
            });
        for line in &self.checked {
            ui.label(line);
        }
    }

    fn take(&mut self, msg: Msg) {
        match msg {
            Msg::List(Ok(lists)) => {
                self.message = format!(
                    "{} NPCs, {} classes, {} weapons.",
                    lists.npcs.len(),
                    lists.classes.len(),
                    lists.weapons.len()
                );
                self.lists = Some(lists);
            }
            Msg::List(Err(e)) => self.message = e,
            Msg::Read(t, Ok(s)) => {
                self.open = Some(t);
                self.script = s.text;
                self.image = s.image.unwrap_or_default();
                self.checked.clear();
                self.message = "Opened.".into();
            }
            Msg::Read(_, Err(e)) => self.message = e,
            Msg::Done(Ok(line)) => self.message = format!("{line}. List again to refresh."),
            Msg::Done(Err(e)) => self.message = e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_opens_the_editor_and_a_failure_keeps_it() {
        let mut p = Page::default();
        p.take(Msg::Read(
            Target::Weapon("Bow".into()),
            Ok(ncdb::Script {
                text: "say2 hi;".into(),
                image: Some("bow.png".into()),
            }),
        ));
        assert_eq!(p.open, Some(Target::Weapon("Bow".into())));
        assert_eq!(
            (p.script.as_str(), p.image.as_str()),
            ("say2 hi;", "bow.png")
        );
        p.take(Msg::Read(
            Target::Class("x".into()),
            Err("no class x arrived".into()),
        ));
        assert_eq!(p.open, Some(Target::Weapon("Bow".into())));
        assert_eq!(p.message, "no class x arrived");
    }
}
