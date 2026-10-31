//! Saving the owner's config files safely: dated backups, write through a
//! temporary file and a rename, and "Undo last change".
//!
//! - [`save`] copies the current file to `<name>.bak-YYYYMMDD-HHMMSS` (UTC)
//!   in the same folder, then writes the new text to a temporary file there
//!   and renames it over the old one, so an interrupted save never leaves
//!   half a file.
//! - [`undo`] puts the newest backup back. The file it replaces becomes a
//!   backup of its own first, so an undo can itself be undone.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::stamp::Stamp;

fn file_name(path: &Path) -> io::Result<String> {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "not a file path"))
}

fn folder(path: &Path) -> PathBuf {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// The backup name for `path` at `stamp`. If that name is taken (two saves
/// in one second), `-2`, `-3`, ... is added.
pub fn backup_path(path: &Path, stamp: Stamp) -> io::Result<PathBuf> {
    let base = format!("{}.bak-{}", file_name(path)?, stamp.compact());
    let dir = folder(path);
    let mut candidate = dir.join(&base);
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{base}-{n}"));
        n += 1;
    }
    Ok(candidate)
}

/// Every backup of `path`, oldest first.
pub fn backups(path: &Path) -> io::Result<Vec<PathBuf>> {
    let prefix = format!("{}.bak-", file_name(path)?);
    let mut out: Vec<PathBuf> = fs::read_dir(folder(path))?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
        .map(|e| e.path())
        .collect();
    // Names sort by time; a `-N` suffix sorts after the plain name.
    out.sort_by_key(|p| {
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let rest = &name[prefix.len()..];
        let (time, n) = match rest.get(15..).and_then(|s| s.strip_prefix('-')) {
            Some(n) => (rest[..15].to_string(), n.parse::<u32>().unwrap_or(0)),
            None => (rest.to_string(), 1),
        };
        (time, n)
    });
    Ok(out)
}

/// Write `text` atomically: a temporary file in the same folder, then a rename.
pub fn write_atomic(path: &Path, text: &[u8]) -> io::Result<()> {
    let tmp = folder(path).join(format!(".{}.servertools-tmp", file_name(path)?));
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Back up `path` (if it exists) and write `text` to it. Returns the backup's
/// path, or `None` when there was no file to back up.
pub fn save(path: &Path, text: &[u8], stamp: Stamp) -> io::Result<Option<PathBuf>> {
    let backup = if path.exists() {
        let b = backup_path(path, stamp)?;
        fs::copy(path, &b)?;
        Some(b)
    } else {
        None
    };
    write_atomic(path, text)?;
    Ok(backup)
}

/// Put the newest backup back. The current file is kept as a new backup
/// (named for `stamp`), and the restored backup is removed, so repeated
/// undos walk back through the history and one more undo redoes. Returns
/// the backup that was restored.
pub fn undo(path: &Path, stamp: Stamp) -> io::Result<PathBuf> {
    let list = backups(path)?;
    let newest = list
        .last()
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no backup to undo to"))?;
    let restored = fs::read(&newest)?;
    if path.exists() {
        // Kept as a backup dated now, so the next undo goes back to it.
        let keep = backup_path(path, stamp)?;
        fs::copy(path, &keep)?;
    }
    write_atomic(path, &restored)?;
    fs::remove_file(&newest)?;
    Ok(newest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: i64) -> Stamp {
        Stamp::from_unix(1_793_473_631 + secs)
    }

    #[test]
    fn save_backs_up_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("serveroptions.txt");
        assert_eq!(save(&f, b"a = 1\n", at(0)).unwrap(), None);
        let b = save(&f, b"a = 2\n", at(1)).unwrap().unwrap();
        assert_eq!(
            b.file_name().unwrap().to_string_lossy(),
            "serveroptions.txt.bak-20261031-190712"
        );
        assert_eq!(fs::read(&b).unwrap(), b"a = 1\n");
        assert_eq!(fs::read(&f).unwrap(), b"a = 2\n");
        let files = fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(files, 2, "no temporary file left behind");
    }

    #[test]
    fn two_saves_in_one_second_keep_both_backups() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("o.txt");
        fs::write(&f, "1").unwrap();
        save(&f, b"2", at(0)).unwrap();
        save(&f, b"3", at(0)).unwrap();
        let list = backups(&f).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(fs::read(&list[0]).unwrap(), b"1");
        assert_eq!(fs::read(&list[1]).unwrap(), b"2");
    }

    #[test]
    fn undo_restores_and_can_be_undone() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("o.txt");
        fs::write(&f, "lab").unwrap();
        save(&f, b"public", at(0)).unwrap();
        undo(&f, at(10)).unwrap();
        assert_eq!(fs::read(&f).unwrap(), b"lab");
        undo(&f, at(20)).unwrap();
        assert_eq!(fs::read(&f).unwrap(), b"public");
    }

    #[test]
    fn undo_without_backup_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("o.txt");
        fs::write(&f, "x").unwrap();
        assert_eq!(undo(&f, at(0)).unwrap_err().kind(), io::ErrorKind::NotFound);
    }
}
