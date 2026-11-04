//! The review the level and gani pages share: the bytes an editor
//! downloaded, the diff against them, and the refused upload when the
//! server's copy changed in between. The diff is `lat_core::diff`; the
//! check before the upload is `lat_core::remote::write_file_if_unchanged`.

use eframe::egui;
use lat_core::diff;
use lat_core::remote::{self, FileSource, Upload};

/// What a finished upload call hands back to the page.
pub type Saved = Result<(String, Vec<u8>, Upload), String>;

/// Lines of unchanged text shown around each change.
const CONTEXT: usize = 2;

fn lines_word(n: usize) -> String {
    if n == 1 {
        "1 line removed".into()
    } else {
        format!("{n} lines removed")
    }
}

#[derive(Default)]
pub struct Review {
    /// The file that was opened, and its bytes as downloaded.
    opened: Option<(String, Vec<u8>)>,
    title: String,
    text: String,
}

impl Review {
    /// A file was opened: these bytes are what the upload compares against.
    pub fn opened(&mut self, path: &str, bytes: &[u8]) {
        self.opened = Some((path.to_string(), bytes.to_vec()));
        self.clear();
    }

    fn clear(&mut self) {
        self.title.clear();
        self.text.clear();
    }

    fn set(&mut self, title: String, old: &[u8], new: &[u8]) {
        let lines = diff::lines(old, new);
        let (removed, added) = diff::counts(&lines);
        self.title = format!("{title} ({}, {} added)", lines_word(removed), added);
        self.text = diff::render(&lines, CONTEXT);
    }

    /// Show the edit against the downloaded bytes.
    pub fn show_mine(&mut self, new: &[u8]) -> String {
        let Some((path, old)) = self.opened.clone() else {
            return "Open a file first.".into();
        };
        self.set(format!("Your changes to {path}"), &old, new);
        if self.text.is_empty() {
            format!("No changes to {path} yet.")
        } else {
            format!("Changes to {path} are below.")
        }
    }

    /// The upload job: upload `data` over the opened file only if the
    /// server still has the downloaded bytes. `None` when nothing is open.
    pub fn upload_job(&self, data: Vec<u8>) -> Option<impl FnOnce(FileSource) -> Saved + Send> {
        let (path, old) = self.opened.clone()?;
        Some(move |source: FileSource| {
            remote::write_file_if_unchanged(&source, &path, &old, &data)
                .map(|up| (path, data, up))
                .map_err(|e| e.to_string())
        })
    }

    /// Take a finished upload call and say what happened.
    pub fn take(&mut self, saved: Saved) -> String {
        match saved {
            Ok((path, data, Upload::Written)) => {
                self.opened = Some((path.clone(), data));
                self.clear();
                format!("Uploaded {path}.")
            }
            Ok((path, _, Upload::Changed(now))) => {
                let old = self
                    .opened
                    .as_ref()
                    .map(|(_, b)| b.clone())
                    .unwrap_or_default();
                self.set(
                    format!("What changed on the server in {path} since you opened it"),
                    &old,
                    &now,
                );
                format!(
                    "Not uploaded: someone changed {path} on the server after you opened it. \
                     Their change is below. Open the file again to start from the server's copy, \
                     then redo your edit."
                )
            }
            Err(e) => e,
        }
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        if self.title.is_empty() {
            return;
        }
        ui.label(&self.title);
        if self.text.is_empty() {
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("lat-review")
            .max_height(180.0)
            .show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(&self.text).monospace()).wrap());
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder_with(text: &[u8]) -> (tempfile::TempDir, FileSource) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("levels")).unwrap();
        std::fs::write(dir.path().join("levels/a.nw"), text).unwrap();
        let source = FileSource::Folder(dir.path().to_path_buf());
        (dir, source)
    }

    #[test]
    fn upload_then_a_second_upload_from_the_new_base() {
        let (dir, source) = folder_with(b"GLEVNW01\nBOARD 0 0 1 0 AA\n");
        let mut review = Review::default();
        review.opened("levels/a.nw", b"GLEVNW01\nBOARD 0 0 1 0 AA\n");
        let mine = b"GLEVNW01\nBOARD 0 0 1 0 AB\n".to_vec();
        assert_eq!(review.show_mine(&mine), "Changes to levels/a.nw are below.");
        assert!(review.text.contains("+ BOARD 0 0 1 0 AB"));

        let job = review.upload_job(mine.clone()).unwrap();
        assert_eq!(review.take(job(source)), "Uploaded levels/a.nw.");
        assert!(review.text.is_empty());

        // The uploaded bytes are the new base, so a second edit goes through.
        let source = FileSource::Folder(dir.path().to_path_buf());
        let job = review.upload_job(b"GLEVNW01\n".to_vec()).unwrap();
        assert_eq!(review.take(job(source)), "Uploaded levels/a.nw.");
    }

    #[test]
    fn upload_is_refused_and_the_server_change_is_shown() {
        let (dir, source) = folder_with(b"GLEVNW01\nBOARD 0 0 1 0 AA\n");
        let mut review = Review::default();
        review.opened("levels/a.nw", b"GLEVNW01\nBOARD 0 0 1 0 AA\n");
        std::fs::write(
            dir.path().join("levels/a.nw"),
            b"GLEVNW01\nBOARD 0 0 1 0 AC\n",
        )
        .unwrap();

        let job = review
            .upload_job(b"GLEVNW01\nBOARD 0 0 1 0 AB\n".to_vec())
            .unwrap();
        let said = review.take(job(source));
        assert!(said.starts_with("Not uploaded"), "{said}");
        assert!(
            review.text.contains("+ BOARD 0 0 1 0 AC"),
            "{}",
            review.text
        );
        assert_eq!(
            std::fs::read(dir.path().join("levels/a.nw")).unwrap(),
            b"GLEVNW01\nBOARD 0 0 1 0 AC\n"
        );
    }

    #[test]
    fn nothing_open_means_no_upload() {
        assert!(Review::default().upload_job(Vec::new()).is_none());
    }
}
