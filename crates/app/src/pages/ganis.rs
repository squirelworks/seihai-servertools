//! Gani editor page.
//!
//! Same split as the level page: this holds the form and the open
//! [`lat_core::gani::GaniDoc`]. The file link, the sprite-frame edit and
//! the RGBA preview are `lat-core`.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use eframe::egui;
use lat_core::gani::GaniDoc;
use lat_core::preview::{pattern_image, FrameView, GaniPics, Sheet};
use lat_core::remote::{self, FileSource};

use super::link::Where;
use crate::preview;

const PATTERN: u32 = 64;

enum Msg {
    List(Result<Vec<String>, String>),
    Open(Result<Vec<u8>, String>),
    Save(Result<(), String>),
}

pub struct Page {
    place: Where,
    sprite: String,
    x: String,
    y: String,
    w: String,
    h: String,
    frame: usize,
    dir: u8,
    doc: Option<GaniDoc>,
    preview_buf: Option<lat_core::preview::Rgba>,
    names: Vec<String>,
    message: String,
    pending: Option<Receiver<Msg>>,
    pattern: Vec<u8>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            place: Where::default(),
            sprite: "0".into(),
            x: "0".into(),
            y: "0".into(),
            w: "1".into(),
            h: "1".into(),
            frame: 0,
            dir: 2,
            doc: None,
            preview_buf: None,
            names: Vec::new(),
            message: String::new(),
            pending: None,
            pattern: pattern_image(PATTERN),
        }
    }
}

impl Page {
    pub fn set_folder(&mut self, folder: &str) {
        self.place.set_folder(folder);
    }

    fn view(&self) -> FrameView {
        FrameView {
            frame: self.frame,
            dir: self.dir,
            x: 32,
            y: 32,
            width: 96,
            height: 96,
        }
    }

    fn repaint(&mut self) {
        let view = self.view();
        let painted = {
            let Some(doc) = &self.doc else {
                self.preview_buf = None;
                return;
            };
            let sheet = Sheet {
                rgb: self.pattern.as_slice(),
                width: PATTERN,
                height: PATTERN,
            };
            doc.paint(&GaniPics::all(sheet), view)
        };
        self.preview_buf = Some(painted);
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

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Edit a gani on a live server, or in a world folder on this machine. A sprite frame is the rectangle on its sheet. Upload writes the gani text through the file link.");
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
                    self.message = "Type the file to open.".into();
                } else {
                    self.spawn(ui.ctx(), move |source| {
                        Msg::Open(remote::read_file(&source, &path).map_err(|e| e.to_string()))
                    });
                }
            }
            if ui
                .add_enabled(!busy && self.doc.is_some(), egui::Button::new("Upload"))
                .clicked()
            {
                let path = self.place.path.clone();
                let bytes = self.doc.as_ref().map(|doc| doc.encode());
                if let Some(bytes) = bytes {
                    self.spawn(ui.ctx(), move |source| {
                        Msg::Save(
                            remote::write_file(&source, &path, &bytes).map_err(|e| e.to_string()),
                        )
                    });
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
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Sprite:");
            ui.add(egui::TextEdit::singleline(&mut self.sprite).desired_width(40.0));
            ui.label("x:");
            ui.add(egui::TextEdit::singleline(&mut self.x).desired_width(40.0));
            ui.label("y:");
            ui.add(egui::TextEdit::singleline(&mut self.y).desired_width(40.0));
            ui.label("w:");
            ui.add(egui::TextEdit::singleline(&mut self.w).desired_width(40.0));
            ui.label("h:");
            ui.add(egui::TextEdit::singleline(&mut self.h).desired_width(40.0));
            if ui
                .add_enabled(self.doc.is_some(), egui::Button::new("Set sprite frame"))
                .clicked()
            {
                self.apply_frame();
            }
        });
        ui.horizontal(|ui| {
            let count = self.doc.as_ref().map(|d| d.frame_count()).unwrap_or(0);
            ui.label(format!("Animation frame ({} steps):", count.max(1)));
            if ui.add_enabled(count > 0, egui::Button::new("<")).clicked() && self.frame > 0 {
                self.frame -= 1;
                self.repaint();
            }
            ui.label(self.frame.to_string());
            if ui.add_enabled(count > 0, egui::Button::new(">")).clicked() && self.frame + 1 < count
            {
                self.frame += 1;
                self.repaint();
            }
            ui.label("Direction:");
            for (dir, name) in [(0, "up"), (1, "left"), (2, "down"), (3, "right")] {
                if ui.selectable_label(self.dir == dir, name).clicked() {
                    self.dir = dir;
                    self.repaint();
                }
            }
        });
        ui.label("The preview draws body, head, sword, shield, sprites and attributes from one stand-in picture, so a sprite frame change shows without the world's images. A sprite that names some other file is skipped.");
        if let Some(buf) = &self.preview_buf {
            preview::show_buffer(ui, "lat-gani-preview", buf);
        }
    }

    fn apply_frame(&mut self) {
        let sprite = match self.sprite.trim().parse::<u32>() {
            Ok(n) => n,
            Err(_) => {
                self.message = "Sprite id is not a whole number.".into();
                return;
            }
        };
        let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
            self.x.trim().parse::<i32>(),
            self.y.trim().parse::<i32>(),
            self.w.trim().parse::<i32>(),
            self.h.trim().parse::<i32>(),
        ) else {
            self.message =
                "The sprite frame needs whole numbers for x, y, width and height.".into();
            return;
        };
        let edited = match &mut self.doc {
            Some(doc) => doc.set_sprite_frame(sprite, x, y, w, h),
            None => return,
        };
        match edited {
            Ok(()) => {
                self.message =
                    format!("Sprite {sprite} frame is {x},{y} {w} by {h}. Upload to send it.");
                self.repaint();
            }
            Err(e) => self.message = e,
        }
    }

    fn take(&mut self, msg: Msg) {
        match msg {
            Msg::List(Ok(names)) => {
                self.message = format!("{} names.", names.len());
                self.names = names;
            }
            Msg::List(Err(e)) => self.message = e,
            Msg::Open(Ok(bytes)) => match GaniDoc::open(&bytes) {
                Ok(doc) => {
                    if let Some((x, y, w, h)) = doc.sprite_frame(0) {
                        self.sprite = "0".into();
                        self.x = x.to_string();
                        self.y = y.to_string();
                        self.w = w.to_string();
                        self.h = h.to_string();
                    }
                    self.frame = 0;
                    self.message = format!(
                        "Opened {} ({} bytes, {} frames). Set a sprite frame, then upload.",
                        self.place.path,
                        bytes.len(),
                        doc.frame_count()
                    );
                    self.doc = Some(doc);
                    self.repaint();
                }
                Err(e) => {
                    self.doc = None;
                    self.preview_buf = None;
                    self.message = e;
                }
            },
            Msg::Open(Err(e)) => self.message = e,
            Msg::Save(Ok(())) => self.message = format!("Uploaded {}.", self.place.path),
            Msg::Save(Err(e)) => self.message = e,
        }
    }
}
