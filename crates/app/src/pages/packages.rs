//! Packages page: build a `.gpak` or `.gpake` for a gserver folder, and
//! show what an existing one holds. The same `lat_core::gpak` calls as
//! `seihai-servertools gpak pack --server`: the package goes to the
//! folder's `world/gpaks/` and a `.gpake`'s key to `config/gpakkeys.txt`,
//! where seihai-gserver-v2 finds both. Packing runs on a worker thread.

use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

use eframe::egui;
use lat_core::gpak::{self, Key, Packed};

/// What the form asks to pack, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRequest {
    pub source: PathBuf,
    pub out: PathBuf,
    pub key: Option<Key>,
    pub key_file: Option<PathBuf>,
}

/// A package's files and sizes, or why they could not be read.
type Contents = Result<Vec<(String, usize)>, String>;

/// One package in the folder's `world/gpaks/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub name: String,
    /// For a `.gpake`: whether `config/gpakkeys.txt` has its key.
    pub has_key: Option<bool>,
}

#[derive(Default)]
pub struct Page {
    /// The gserver folder, `<world>/servers/<name>`.
    pub server: String,
    /// The folder whose files go into the package.
    pub source: String,
    /// The package's file name, ending in `.gpake`, `.gpak` or `.zip`.
    pub name: String,
    /// 16 hex digits, or empty for a random key.
    pub key: String,
    server_picker: super::picker::Picker,
    source_picker: super::picker::Picker,
    job: Option<JoinHandle<Result<Packed, String>>>,
    result: Option<String>,
    packages: Vec<Listed>,
    contents: Option<(String, Contents)>,
}

impl Page {
    pub fn set_server(&mut self, server: &str) {
        self.server = server.to_string();
        self.refresh();
    }

    /// Check the form. The rules on names and keys that `gpak::pack`
    /// itself checks are left to it.
    pub fn request(&self) -> Result<PackRequest, String> {
        let server = Path::new(self.server.trim());
        if self.server.trim().is_empty() || !server.join("config").is_dir() {
            return Err("pick a gserver folder (servers/<name>, with a config folder)".into());
        }
        let source = self.source.trim();
        if source.is_empty() || !Path::new(source).is_dir() {
            return Err("pick the folder to pack".into());
        }
        let name = self.name.trim();
        if name.is_empty() || Path::new(name).components().count() != 1 {
            return Err("the package name is a file name, not a path".into());
        }
        let encrypted = gpak::is_encrypted(name);
        let key = match self.key.trim() {
            "" => None,
            k if !encrypted => return Err(format!("only a .gpake takes a key ({k})")),
            k => Some(gpak::parse_key(k).ok_or("the key is 16 hex digits")?),
        };
        Ok(PackRequest {
            source: source.into(),
            out: gpak::server_package_path(server, name),
            key,
            key_file: encrypted.then(|| gpak::keys_path(server)),
        })
    }

    /// The packages in the folder's `world/gpaks/`, by name.
    pub fn list_packages(server: &Path) -> Vec<Listed> {
        let keys = std::fs::read_to_string(gpak::keys_path(server)).unwrap_or_default();
        let mut out: Vec<Listed> = std::fs::read_dir(server.join("world").join("gpaks"))
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().is_file())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| {
                        let l = n.to_ascii_lowercase();
                        l.ends_with(".gpake") || l.ends_with(".gpak") || l.ends_with(".zip")
                    })
                    .map(|name| Listed {
                        has_key: gpak::is_encrypted(&name)
                            .then(|| gpak::key_in(&keys, &name).is_some()),
                        name,
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// What a package holds, decrypting a `.gpake` with its saved key.
    pub fn contents(server: &Path, name: &str) -> Contents {
        let key = if gpak::is_encrypted(name) {
            let keys = std::fs::read_to_string(gpak::keys_path(server)).unwrap_or_default();
            Some(gpak::key_in(&keys, name).ok_or("no key for it in config/gpakkeys.txt")?)
        } else {
            None
        };
        gpak::list(&gpak::server_package_path(server, name), key.as_ref())
            .map_err(|e| e.to_string())
    }

    /// Start packing on a worker thread.
    pub fn start(&mut self, req: PackRequest) {
        self.result = Some(format!("packing {} ...", req.source.display()));
        self.job = Some(std::thread::spawn(move || {
            gpak::pack(&req.source, &req.out, req.key, req.key_file.as_deref())
                .map_err(|e| e.to_string())
        }));
    }

    /// Collect a finished job. True when one finished on this call.
    pub fn poll(&mut self) -> bool {
        if !self.job.as_ref().is_some_and(|j| j.is_finished()) {
            return false;
        }
        let job = self.job.take().expect("checked above");
        self.result = Some(match job.join() {
            Ok(Ok(p)) => {
                let mut text = format!("wrote {} ({} files)", p.path.display(), p.files);
                if let (Some(_), Some(kf)) = (p.key, &p.key_file) {
                    text.push_str(&format!("; key saved in {}", kf.display()));
                }
                text.push_str(
                    "\nName it in the gserver's gpaks option to send it at login; the key goes first.",
                );
                text
            }
            Ok(Err(e)) => format!("error: {e}"),
            Err(_) => "error: the packing thread stopped".into(),
        });
        self.refresh();
        true
    }

    fn refresh(&mut self) {
        self.packages = Self::list_packages(Path::new(self.server.trim()));
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.poll();
        let busy = self.job.is_some();
        egui::Grid::new("pack-form").num_columns(2).show(ui, |ui| {
            ui.label("Gserver folder:");
            ui.horizontal(|ui| {
                let edit = ui.add_enabled(!busy, egui::TextEdit::singleline(&mut self.server));
                let picked = self.server_picker.button(ui, !busy, &mut self.server);
                if edit.lost_focus() || picked {
                    self.refresh();
                }
            });
            ui.end_row();
            ui.label("Files to pack:");
            ui.horizontal(|ui| {
                ui.add_enabled(!busy, egui::TextEdit::singleline(&mut self.source));
                self.source_picker.button(ui, !busy, &mut self.source);
            });
            ui.end_row();
            ui.label("Package name:");
            ui.add_enabled(
                !busy,
                egui::TextEdit::singleline(&mut self.name).hint_text("art.gpake"),
            );
            ui.end_row();
            ui.label("Key (.gpake):");
            ui.add_enabled(
                !busy,
                egui::TextEdit::singleline(&mut self.key).hint_text("empty for a random key"),
            );
            ui.end_row();
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(!busy, egui::Button::new("Pack")).clicked() {
                match self.request() {
                    Ok(req) => self.start(req),
                    Err(e) => self.result = Some(format!("error: {e}")),
                }
            }
            if ui.button("Refresh list").clicked() {
                self.refresh();
            }
        });
        ui.label(
            "Use .gpake when 4.20 players matter: 4.20r8 saves a plain .gpak but never loads it.",
        );
        if let Some(text) = &self.result {
            ui.monospace(text);
        }
        if busy {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(200));
        }
        ui.separator();
        ui.label("Packages in world/gpaks/:");
        let server = PathBuf::from(self.server.trim());
        let mut open = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.packages.is_empty() {
                ui.label("(none)");
            }
            for p in &self.packages {
                ui.horizontal(|ui| {
                    ui.monospace(&p.name);
                    match p.has_key {
                        Some(false) => {
                            ui.colored_label(
                                egui::Color32::from_rgb(200, 60, 50),
                                "no key in config/gpakkeys.txt: the gserver will not send it",
                            );
                        }
                        Some(true) => {
                            ui.label("key saved");
                        }
                        None => {}
                    }
                    if ui.button("Show contents").clicked() {
                        open = Some(p.name.clone());
                    }
                });
            }
            if let Some((name, list)) = &self.contents {
                ui.separator();
                ui.label(egui::RichText::new(name).strong());
                match list {
                    Ok(files) => {
                        for (f, size) in files {
                            ui.monospace(format!("{size:>10}  {f}"));
                        }
                    }
                    Err(e) => {
                        ui.monospace(format!("error: {e}"));
                    }
                }
            }
        });
        if let Some(name) = open {
            let list = Self::contents(&server, &name);
            self.contents = Some((name, list));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn folder() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let server = dir.path().join("servers/modern");
        fs::create_dir_all(server.join("config")).unwrap();
        let art = dir.path().join("art");
        fs::create_dir_all(art.join("sub")).unwrap();
        fs::write(art.join("tree.png"), b"0123456789abcdef-tail").unwrap();
        fs::write(art.join("sub/walk.gani"), b"GANI0001\n").unwrap();
        (dir, server, art)
    }

    fn page(server: &Path, art: &Path, name: &str, key: &str) -> Page {
        Page {
            server: server.display().to_string(),
            source: art.display().to_string(),
            name: name.into(),
            key: key.into(),
            ..Page::default()
        }
    }

    #[test]
    fn the_form_is_checked() {
        let (_d, server, art) = folder();
        let r = page(&server, &art, "art.gpake", "0123456789abcdef")
            .request()
            .unwrap();
        assert_eq!(r.out, server.join("world/gpaks/art.gpake"));
        assert_eq!(r.key_file, Some(server.join("config/gpakkeys.txt")));
        assert_eq!(r.key, gpak::parse_key("0123456789ABCDEF"));
        let plain = page(&server, &art, "art.gpak", "").request().unwrap();
        assert_eq!((plain.key, plain.key_file), (None, None));

        for (p, why) in [
            (
                page(&server, &art, "art.gpak", "0123456789abcdef"),
                "only a .gpake",
            ),
            (page(&server, &art, "art.gpake", "0123"), "16 hex"),
            (page(&server, &art, "a/art.gpake", ""), "not a path"),
            (
                page(&server, &server.join("nope"), "art.gpake", ""),
                "folder to pack",
            ),
            (page(&art, &art, "art.gpake", ""), "gserver folder"),
        ] {
            let e = p.request().unwrap_err();
            assert!(e.contains(why), "{e}");
        }
    }

    #[test]
    fn pack_list_and_show_contents() {
        let (_d, server, art) = folder();
        let mut p = page(&server, &art, "art.gpake", "");
        let req = p.request().unwrap();
        p.start(req);
        while !p.poll() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let text = p.result.clone().unwrap();
        assert!(
            text.contains("2 files") && text.contains("key saved"),
            "{text}"
        );
        assert_eq!(
            p.packages,
            [Listed {
                name: "art.gpake".into(),
                has_key: Some(true)
            }]
        );
        let files = Page::contents(&server, "art.gpake").unwrap();
        assert_eq!(
            files,
            [
                ("sub/walk.gani".to_string(), 9),
                ("tree.png".to_string(), 21)
            ]
        );
        // The decrypted bytes are the originals.
        let keys = fs::read_to_string(server.join("config/gpakkeys.txt")).unwrap();
        let key = gpak::key_in(&keys, "art.gpake").unwrap();
        let f = fs::File::open(server.join("world/gpaks/art.gpake")).unwrap();
        let entries = gpak::read(f, Some(&key)).unwrap();
        assert_eq!(entries[1].data, b"0123456789abcdef-tail");

        // A package with no saved key is shown as such and not opened.
        fs::copy(
            server.join("world/gpaks/art.gpake"),
            server.join("world/gpaks/other.gpake"),
        )
        .unwrap();
        fs::write(server.join("world/gpaks/notes.txt"), "x").unwrap();
        let listed = Page::list_packages(&server);
        assert_eq!(listed.len(), 2, "{listed:?}");
        assert_eq!(listed[1].has_key, Some(false));
        assert!(Page::contents(&server, "other.gpake").is_err());

        // A failed pack (empty folder) reports the error and writes nothing.
        let empty = art.join("empty");
        fs::create_dir_all(&empty).unwrap();
        let mut p = page(&server, &empty, "none.gpak", "");
        let req = p.request().unwrap();
        p.start(req);
        while !p.poll() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(p.result.unwrap().starts_with("error:"));
        assert!(!server.join("world/gpaks/none.gpak").exists());
    }
}
