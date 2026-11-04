//! The folder picker's listing: the subfolders of one folder, and whether
//! a folder looks like a world folder.
//!
//! Plain `std::fs`, no native dialog: the same picker works in every build
//! of the window and on every system. Nothing is written.

use std::fs;
use std::path::{Path, PathBuf};

/// One subfolder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sub {
    pub name: String,
    /// It has `servers/<name>/config/serveroptions.txt` or a
    /// `seihai-listserver.toml`.
    pub world: bool,
}

/// What is in a folder, for the picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub path: PathBuf,
    pub parent: Option<PathBuf>,
    /// Subfolders by name, case-insensitive; hidden ones (`.name`) left out.
    pub subs: Vec<Sub>,
    pub world: bool,
}

/// A world folder: at least one `servers/<name>/config/serveroptions.txt`,
/// or a `seihai-listserver.toml` at the top.
pub fn is_world(path: &Path) -> bool {
    if path.join("seihai-listserver.toml").is_file() {
        return true;
    }
    let Ok(entries) = fs::read_dir(path.join("servers")) else {
        return false;
    };
    entries
        .flatten()
        .any(|e| e.path().join("config/serveroptions.txt").is_file())
}

/// List `path`. Unreadable subfolders are still listed; a folder that
/// cannot be read at all is an error.
pub fn list(path: &Path) -> Result<Listing, String> {
    let entries = fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut subs: Vec<Sub> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            (!name.starts_with('.')).then(|| Sub {
                world: is_world(&e.path()),
                name,
            })
        })
        .collect();
    subs.sort_by_key(|s| s.name.to_lowercase());
    Ok(Listing {
        path: path.to_path_buf(),
        parent: path.parent().map(Path::to_path_buf),
        world: is_world(path),
        subs,
    })
}

/// Where the picker starts: `typed` if it is a folder, else the nearest
/// folder above it that exists, else the current folder.
pub fn start(typed: &str) -> PathBuf {
    let typed = typed.trim();
    let mut p = if typed.is_empty() {
        PathBuf::new()
    } else {
        PathBuf::from(typed)
    };
    while !p.as_os_str().is_empty() {
        if p.is_dir() {
            return p;
        }
        if !p.pop() {
            break;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_subfolders_and_marks_worlds() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        fs::create_dir_all(root.join("b-world/servers/default/config")).unwrap();
        fs::write(
            root.join("b-world/servers/default/config/serveroptions.txt"),
            "",
        )
        .unwrap();
        fs::create_dir_all(root.join("A-list")).unwrap();
        fs::write(root.join("A-list/seihai-listserver.toml"), "").unwrap();
        fs::create_dir_all(root.join("c-plain/servers/notes")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("file.txt"), "").unwrap();

        let l = list(root).unwrap();
        let got: Vec<(&str, bool)> = l.subs.iter().map(|s| (s.name.as_str(), s.world)).collect();
        assert_eq!(
            got,
            [("A-list", true), ("b-world", true), ("c-plain", false)]
        );
        assert!(!l.world);
        assert_eq!(l.parent.as_deref(), root.parent());
        assert!(list(&root.join("b-world")).unwrap().world);
        assert!(list(&root.join("missing")).is_err());
    }

    #[test]
    fn starts_at_the_nearest_folder_that_exists() {
        let d = tempfile::tempdir().unwrap();
        let typed = d.path().join("not/there/yet");
        assert_eq!(start(&typed.display().to_string()), d.path());
        assert_eq!(start(&d.path().display().to_string()), d.path());
        assert!(start("").is_dir());
    }
}
