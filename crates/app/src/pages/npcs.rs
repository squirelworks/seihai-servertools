//! NPC editor page: level NPCs.
//!
//! Open a level (folder or live server), pick an NPC, edit its script, check
//! it with `seihai-core`'s GS1 parser (`lat_core::npc::check_script`), apply
//! it to the level and upload. The upload goes through the same review as
//! the level page: Show changes, and a refusal when the server's copy
//! changed since it was opened.
//!
//! Database NPCs, classes and weapons need an NC login (`seihai-rc` stage
//! 5) and are not here yet.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use eframe::egui;
use lat_core::level::LevelDoc;
use lat_core::npc::{self, FileKind};
use lat_core::remote::{self, FileSource};

use super::link::Where;
use super::review::{Review, Saved};

enum Msg {
    List(Result<Vec<String>, String>),
    Open(String, Result<Vec<u8>, String>),
    Save(Saved),
}

pub struct Page {
    place: Where,
    review: Review,
    doc: Option<LevelDoc>,
    /// The NPC being edited (file order) and the text in the editor.
    picked: Option<usize>,
    script: String,
    /// The last check of `script`, as lines to show.
    checked: Vec<String>,
    names: Vec<String>,
    message: String,
    pending: Option<Receiver<Msg>>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            place: Where {
                dir: "levels".into(),
                ..Where::default()
            },
            review: Review::default(),
            doc: None,
            picked: None,
            script: String::new(),
            checked: Vec::new(),
            names: Vec::new(),
            message: String::new(),
            pending: None,
        }
    }
}

/// The check of `script`, as lines for the page.
pub fn check_lines(script: &str) -> Vec<String> {
    let c = npc::check_script(script);
    let mut out = Vec::new();
    if c.problems.is_empty() {
        out.push("GS1: no parse problems.".to_string());
    }
    for p in &c.problems {
        out.push(format!("GS1 problem, {p} (the client skips that part)"));
    }
    for u in &c.unknown {
        out.push(format!(
            "Not a command the parser knows, {u}. Server-side commands land here too."
        ));
    }
    for f in &c.files {
        let kind = match f.kind {
            FileKind::Image => "image",
            FileKind::Sound => "sound",
        };
        out.push(format!(
            "Names {kind} {} (line {}, {}).",
            f.name, f.line, f.command
        ));
    }
    out
}

impl Page {
    pub fn set_folder(&mut self, folder: &str) {
        self.place.set_folder(folder);
    }

    fn spawn(&mut self, ctx: &egui::Context, job: impl FnOnce(FileSource) -> Msg + Send + 'static) {
        let source = match self.place.source() {
            Ok(s) => s,
            Err(e) => {
                self.message = e;
                return;
            }
        };
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(job(source));
        });
        self.pending = Some(rx);
        self.message = "Working...".into();
        ctx.request_repaint();
    }

    /// The editor holds text that is not applied to the level yet.
    fn unapplied(&self) -> bool {
        match (&self.doc, self.picked) {
            (Some(doc), Some(i)) => doc.npcs().get(i).is_some_and(|n| {
                n.script.replace("\r\n", "\n") != self.script.replace("\r\n", "\n")
            }),
            _ => false,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Edit the scripts of a level's NPCs, on a live server or in a world folder on this machine. Check runs the GS1 parser; Apply puts the script into the level; Upload sends the level. Database NPCs, classes and weapons need an NC login and are not here yet.");
        self.place.show(ui);
        let busy = self.pending.is_some();
        ui.horizontal(|ui| {
            if ui.add_enabled(!busy, egui::Button::new("List")).clicked() {
                let dir = self.place.dir.clone();
                self.spawn(ui.ctx(), move |source| {
                    Msg::List(
                        remote::list_dir(&source, &dir)
                            .map(|entries| entries.into_iter().map(|e| e.name).collect())
                            .map_err(|e| e.to_string()),
                    )
                });
            }
            if ui.add_enabled(!busy, egui::Button::new("Open")).clicked() {
                let path = self.place.path.clone();
                if path.trim().is_empty() {
                    self.message = "Type the level to open.".into();
                } else {
                    self.spawn(ui.ctx(), move |source| {
                        let got = remote::read_file(&source, &path).map_err(|e| e.to_string());
                        Msg::Open(path, got)
                    });
                }
            }
            if ui
                .add_enabled(self.doc.is_some(), egui::Button::new("Show changes"))
                .clicked()
            {
                if let Some(doc) = &self.doc {
                    self.message = self.review.show_mine(&doc.encode());
                }
            }
            if ui
                .add_enabled(!busy && self.doc.is_some(), egui::Button::new("Upload"))
                .on_hover_text("Refused if the server's copy changed since you opened it.")
                .clicked()
            {
                if self.unapplied() {
                    self.message = "The script in the editor is not applied yet. Apply it, or Revert, then upload.".into();
                } else {
                    let job = self
                        .doc
                        .as_ref()
                        .and_then(|doc| self.review.upload_job(doc.encode()));
                    if let Some(job) = job {
                        self.spawn(ui.ctx(), move |source| Msg::Save(job(source)));
                    }
                }
            }
        });
        if !self.names.is_empty() {
            ui.label("Names from the file link (it does not say which are folders):");
            ui.horizontal_wrapped(|ui| {
                let mut picked = None;
                for name in &self.names {
                    if ui.button(name).clicked() {
                        picked = Some(name.clone());
                    }
                }
                if let Some(name) = picked {
                    self.place.path = remote::join_path(&self.place.dir, &name);
                }
            });
        }
        if let Some(rx) = &self.pending {
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
                    self.message = "The file call stopped.".into();
                }
            }
        }
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        self.review.show(ui);
        ui.separator();
        self.show_npcs(ui);
    }

    fn show_npcs(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = &self.doc else {
            ui.label("Open a level to see its NPCs.");
            return;
        };
        if doc.npcs().is_empty() {
            ui.label("This level has no NPCs.");
            return;
        }
        let mut pick = None;
        ui.horizontal_wrapped(|ui| {
            for (i, n) in doc.npcs().iter().enumerate() {
                let image = if n.image.is_empty() {
                    "-"
                } else {
                    n.image.as_str()
                };
                let text = format!(
                    "{} at {},{} ({} lines)",
                    image,
                    n.x,
                    n.y,
                    n.script.lines().count()
                );
                if ui.selectable_label(self.picked == Some(i), text).clicked() {
                    pick = Some(i);
                }
            }
        });
        if let Some(i) = pick {
            self.pick(i);
        }
        let Some(i) = self.picked else {
            ui.label("Pick an NPC to edit its script.");
            return;
        };
        ui.horizontal(|ui| {
            if ui.button("Check").clicked() {
                self.checked = check_lines(&self.script);
            }
            if ui.button("Apply to level").clicked() {
                self.apply(i);
            }
            if ui.button("Revert").clicked() {
                self.pick(i);
            }
            if self.unapplied() {
                ui.label("Not applied yet.");
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("npc-script")
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

    fn pick(&mut self, i: usize) {
        if let Some(n) = self.doc.as_ref().and_then(|d| d.npcs().get(i)) {
            self.picked = Some(i);
            self.script = n.script.clone();
            self.checked.clear();
        }
    }

    fn apply(&mut self, i: usize) {
        let Some(doc) = &mut self.doc else {
            return;
        };
        match doc.set_npc_script(i, &self.script) {
            Ok(()) => {
                self.checked = check_lines(&self.script);
                self.message =
                    "Applied to the level. Show changes to review, Upload to send it.".into();
            }
            Err(e) => self.message = format!("Not applied: {e}."),
        }
    }

    fn take(&mut self, msg: Msg) {
        match msg {
            Msg::List(Ok(names)) => {
                self.message = format!("{} names.", names.len());
                self.names = names;
            }
            Msg::List(Err(e)) => self.message = e,
            Msg::Open(path, Ok(bytes)) => match LevelDoc::open(&bytes) {
                Ok(doc) => {
                    self.review.opened(&path, &bytes);
                    self.message = format!(
                        "Opened {path} ({} bytes, {} NPCs).",
                        bytes.len(),
                        doc.npcs().len()
                    );
                    self.doc = Some(doc);
                    self.picked = None;
                    self.script.clear();
                    self.checked.clear();
                }
                Err(e) => {
                    self.doc = None;
                    self.picked = None;
                    self.message = e;
                }
            },
            Msg::Open(_, Err(e)) => self.message = e,
            Msg::Save(saved) => self.message = self.review.take(saved),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_lines_lists_problems_and_files() {
        let lines = check_lines("if (created) {\n  setimg door.png;\n");
        assert!(
            lines.iter().any(|l| l.starts_with("GS1 problem")),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l == "Names image door.png (line 2, setimg)."),
            "{lines:?}"
        );
        assert_eq!(check_lines("message hi;"), ["GS1: no parse problems."]);
    }

    #[test]
    fn unapplied_text_blocks_nothing_until_picked() {
        let mut page = Page::default();
        assert!(!page.unapplied());
        page.doc = Some(LevelDoc::open(b"GLEVNW01\nNPC - 1 1\nmessage hi;\nNPCEND\n").unwrap());
        page.pick(0);
        assert!(!page.unapplied());
        page.script.push_str("\nplay a.wav;");
        assert!(page.unapplied());
        page.apply(0);
        assert!(!page.unapplied(), "{}", page.message);
        assert!(page.doc.as_ref().unwrap().is_dirty());
    }
}
