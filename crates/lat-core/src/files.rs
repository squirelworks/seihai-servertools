//! Finding a world's files by name for the offline tools (`level render`,
//! `gmap`, `gani movie render`): an index of a folder by lower-case file
//! name, and the loaders the painters in [`crate::render`] and
//! [`crate::movie`] take. Shared here so each standalone tool uses one copy.

use crate::gani::Gani;
use crate::level::Level;
use crate::preview::{GaniPics, Rgba};
use crate::render::{self, OwnedSheet};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The lower-case extension of `path`, or "".
pub fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// The last path part of a name as a script or gmap writes it, in lower case.
pub fn base_name(name: &str) -> String {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase()
}

/// Every file under `dir` (and below) whose extension is one of `exts`, by
/// lower-case file name. The first one found for a name wins.
pub fn file_index(dir: &Path, exts: &[&str]) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                stack.push(p);
            } else if exts.contains(&extension(&p).as_str()) {
                if let Some(name) = p.file_name() {
                    out.entry(name.to_string_lossy().to_ascii_lowercase())
                        .or_insert(p);
                }
            }
        }
    }
    out
}

/// Every level file under `dir` (and below), by lower-case file name.
pub fn level_index(dir: &Path) -> HashMap<String, PathBuf> {
    file_index(dir, &["nw", "graal", "zelda"])
}

/// Reads and parses one level file; the error names the file.
pub fn read_level(path: &Path) -> Result<Level, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    crate::level::open(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// A tile sheet from a PNG file, or Seihai's built-in `pics1.png`.
pub fn sheet(path: Option<&Path>) -> Result<OwnedSheet, String> {
    match path {
        Some(p) => {
            let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
            OwnedSheet::from_png(&bytes)
        }
        None => OwnedSheet::builtin(),
    }
}

/// Levels by name from an index (a name's folder part is ignored).
pub fn level_loader(index: &HashMap<String, PathBuf>) -> impl FnMut(&str) -> Option<Level> + '_ {
    |name| index.get(&base_name(name)).and_then(|p| read_level(p).ok())
}

/// Pictures (.png, .gif, .mng, first frame) by name from an index.
pub fn image_loader(index: &HashMap<String, PathBuf>) -> impl FnMut(&str) -> Option<Rgba> + '_ {
    |name| {
        let bytes = std::fs::read(index.get(&base_name(name))?).ok()?;
        render::decode_image(&bytes)
    }
}

/// Ganis by name from an index (".gani" added when missing), else a stock
/// gani built into Seihai.
pub fn gani_loader(index: &HashMap<String, PathBuf>) -> impl FnMut(&str) -> Option<Gani> + '_ {
    |name| {
        let mut key = base_name(name);
        if !key.ends_with(".gani") {
            key.push_str(".gani");
        }
        if let Some(bytes) = index.get(&key).and_then(|p| std::fs::read(p).ok()) {
            if let Some(gani) = render::read_gani(&bytes) {
                return Some(gani);
            }
        }
        render::builtin_gani(&key)
    }
}

/// Body, head, sword, shield and sprite sheets for a character gani.
/// Folder files win; body and head fall back to the built-in pictures.
#[derive(Default)]
pub struct CharacterPics {
    pub body: Option<OwnedSheet>,
    pub head: Option<OwnedSheet>,
    pub sword: Option<OwnedSheet>,
    pub shield: Option<OwnedSheet>,
    pub sprites: Option<OwnedSheet>,
}

impl CharacterPics {
    /// The sheets found in `index`, with the built-in body and head.
    pub fn from_index(index: &HashMap<String, PathBuf>) -> Self {
        let png = |file: &str, builtin: Option<&str>| -> Option<OwnedSheet> {
            if let Some(bytes) = index.get(file).and_then(|p| std::fs::read(p).ok()) {
                if let Ok(sheet) = OwnedSheet::from_png(&bytes) {
                    return Some(sheet);
                }
            }
            builtin.and_then(render::builtin_png)
        };
        Self {
            body: png("body.png", Some("bodies/body.png")),
            head: png("head0.png", Some("heads/head0.png")),
            sword: png("sword1.png", None),
            shield: png("shield1.png", None),
            sprites: png("sprites.png", None),
        }
    }

    /// The borrowed view the painters take.
    pub fn view(&self) -> GaniPics<'_> {
        GaniPics {
            body: self.body.as_ref().map(OwnedSheet::sheet),
            head: self.head.as_ref().map(OwnedSheet::sheet),
            sword: self.sword.as_ref().map(OwnedSheet::sheet),
            shield: self.shield.as_ref().map(OwnedSheet::sheet),
            sprites: self.sprites.as_ref().map(OwnedSheet::sheet),
            ..GaniPics::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_finds_files_by_lower_case_name_first_wins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("A.NW"), "x").unwrap();
        std::fs::write(dir.path().join("sub/a.nw"), "y").unwrap();
        std::fs::write(dir.path().join("b.png"), "z").unwrap();
        let index = level_index(dir.path());
        assert_eq!(index.len(), 1);
        assert_eq!(index["a.nw"], dir.path().join("A.NW"));
        assert_eq!(base_name("levels\\Sub/Foo.NW"), "foo.nw");
    }
}
