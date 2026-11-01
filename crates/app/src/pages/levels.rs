//! Level editor page.
//!
//! The page holds what was typed and the open [`lat_core::level::LevelDoc`].
//! Listing, downloading, the tile edit, the RGBA preview and the upload
//! are all `lat-core`. A live link runs on a worker thread so the window
//! does not wait on it.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use eframe::egui;
use lat_core::level::LevelDoc;
use lat_core::preview::{coloured_sheet, BoardView, Sheet, TILE_PX};
use lat_core::remote::{self, FileSource};

use super::link::Where;
use crate::preview;

const VIEW_TILES: u32 = 32;

enum Msg {
    List(Result<Vec<String>, String>),
    Open(Result<Vec<u8>, String>),
    Save(Result<(), String>),
}

pub struct Page {
    place: Where,
    x: String,
    y: String,
    tile: String,
    doc: Option<LevelDoc>,
    preview_buf: Option<lat_core::preview::Rgba>,
    names: Vec<String>,
    message: String,
    pending: Option<Receiver<Msg>>,
    sheet: Vec<u8>,
    sheet_w: u32,
    sheet_h: u32,
}

impl Default for Page {
    fn default() -> Self {
        let (sheet, sheet_w, sheet_h) = coloured_sheet(256);
        Self {
            place: Where {
                dir: "levels".into(),
                ..Where::default()
            },
            x: "0".into(),
            y: "0".into(),
            tile: "1".into(),
            doc: None,
            preview_buf: None,
            names: Vec::new(),
            message: String::new(),
            pending: None,
            sheet,
            sheet_w,
            sheet_h,
        }
    }
}

impl Page {
    pub fn set_folder(&mut self, folder: &str) {
        self.place.set_folder(folder);
    }

    fn view(&self) -> BoardView {
        BoardView {
            origin_x: 0,
            origin_y: 0,
            width: VIEW_TILES * TILE_PX,
            height: VIEW_TILES * TILE_PX,
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
                rgb: self.sheet.as_slice(),
                width: self.sheet_w,
                height: self.sheet_h,
            };
            doc.paint(sheet, view)
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
        ui.label("Edit a level on a live server, or in a world folder on this machine. Upload writes GLEVNW01 text through the file link. The client's offline editor is unchanged.");
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
            ui.label("Tile x:");
            ui.add(egui::TextEdit::singleline(&mut self.x).desired_width(48.0));
            ui.label("y:");
            ui.add(egui::TextEdit::singleline(&mut self.y).desired_width(48.0));
            ui.label("id:");
            ui.add(egui::TextEdit::singleline(&mut self.tile).desired_width(64.0));
            if ui
                .add_enabled(self.doc.is_some(), egui::Button::new("Set tile"))
                .clicked()
            {
                self.apply_tile();
            }
        });
        ui.label("The preview is the top-left 32 by 32 tiles. Tile ids 0 to 255 use stand-in colours (0 red, 1 green); the world's own tile sheet is not loaded.");
        if let Some(buf) = &self.preview_buf {
            preview::show_buffer(ui, "lat-level-preview", buf);
        }
    }

    fn apply_tile(&mut self) {
        let x = match self.x.trim().parse::<i32>() {
            Ok(n) => n,
            Err(_) => {
                self.message = "Tile x is not a whole number.".into();
                return;
            }
        };
        let y = match self.y.trim().parse::<i32>() {
            Ok(n) => n,
            Err(_) => {
                self.message = "Tile y is not a whole number.".into();
                return;
            }
        };
        let id = match self.tile.trim().parse::<u16>() {
            Ok(n) => n,
            Err(_) => {
                self.message = "Tile id is not a whole number.".into();
                return;
            }
        };
        let edited = match &mut self.doc {
            Some(doc) => doc.set_tile(x, y, id),
            None => return,
        };
        match edited {
            Ok(()) => {
                self.message = format!("Tile {x},{y} is now {id}. Upload to send it.");
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
            Msg::Open(Ok(bytes)) => match LevelDoc::open(&bytes) {
                Ok(doc) => {
                    self.message = format!(
                        "Opened {} ({} bytes). Set a tile, then upload.",
                        self.place.path,
                        bytes.len()
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
            Msg::Save(Ok(())) => {
                self.message = format!("Uploaded {}.", self.place.path);
            }
            Msg::Save(Err(e)) => self.message = e,
        }
    }
}
