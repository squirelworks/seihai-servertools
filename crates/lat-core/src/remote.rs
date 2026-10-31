//! The file link LAT tools work through.
//!
//! [`RemoteFiles`] is what every editor uses to read and write world files.
//! Two implementations are planned:
//!
//! - [`LocalFolder`] (here): a world folder on disk. Used by the tests, by the
//!   world checker, and by owners who run the gserver on the same machine.
//! - `RcFiles` (later, `docs/NEXT.md`): the RC file browser of a live server
//!   (packet ids 89 to 93, 96 to 98, 155/156, 160), through `seihai-rc`'s
//!   library pinned by tag, logged in with an account that has LAT rights.
//!   The file browser is `seihai-rc` stage 4; this waits for it.
//!
//! Paths are always `/`-separated and relative to the world root, as the RC
//! file browser shows them. `..` and absolute paths are refused, so a tool
//! can never reach outside the world.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// One entry of a folder listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Name inside the folder.
    pub name: String,
    pub is_dir: bool,
    /// Size in bytes (0 for folders).
    pub size: u64,
}

/// List, download and upload world files.
pub trait RemoteFiles {
    /// The entries of a folder (`""` is the world root), sorted by name.
    fn list(&mut self, dir: &str) -> io::Result<Vec<Entry>>;
    /// A file's bytes.
    fn get(&mut self, path: &str) -> io::Result<Vec<u8>>;
    /// Write a file, replacing it. Must not leave half a file behind.
    fn put(&mut self, path: &str, data: &[u8]) -> io::Result<()>;
}

/// Turn a world path into a relative path, refusing `..`, roots and drives.
pub fn safe_relative(path: &str) -> io::Result<PathBuf> {
    let p = Path::new(path);
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => out.push(s),
            Component::CurDir => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("path outside the world: {path}"),
                ))
            }
        }
    }
    Ok(out)
}

/// A world folder on this machine.
#[derive(Debug, Clone)]
pub struct LocalFolder {
    pub root: PathBuf,
}

impl LocalFolder {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        LocalFolder { root: root.into() }
    }

    fn resolve(&self, path: &str) -> io::Result<PathBuf> {
        Ok(self.root.join(safe_relative(path)?))
    }
}

impl RemoteFiles for LocalFolder {
    fn list(&mut self, dir: &str) -> io::Result<Vec<Entry>> {
        let mut out = Vec::new();
        for e in fs::read_dir(self.resolve(dir)?)? {
            let e = e?;
            let meta = e.metadata()?;
            out.push(Entry {
                name: e.file_name().to_string_lossy().into_owned(),
                is_dir: meta.is_dir(),
                size: if meta.is_dir() { 0 } else { meta.len() },
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn get(&mut self, path: &str) -> io::Result<Vec<u8>> {
        fs::read(self.resolve(path)?)
    }

    fn put(&mut self, path: &str, data: &[u8]) -> io::Result<()> {
        let target = self.resolve(path)?;
        let dir = target.parent().unwrap_or(&self.root).to_path_buf();
        fs::create_dir_all(&dir)?;
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tmp = dir.join(format!(".{name}.servertools-tmp"));
        fs::write(&tmp, data)?;
        fs::rename(&tmp, &target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_paths_outside_the_world() {
        for bad in ["../x", "/etc/passwd", "a/../../b"] {
            assert!(safe_relative(bad).is_err(), "{bad}");
        }
        assert_eq!(
            safe_relative("levels/a.nw").unwrap(),
            Path::new("levels/a.nw")
        );
    }

    #[test]
    fn put_get_list_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = LocalFolder::new(dir.path());
        f.put("levels/a.nw", b"GLEVNW01\n").unwrap();
        f.put("levels/a.nw", b"GLEVNW01\nNPC - 1 1\n").unwrap();
        assert_eq!(f.get("levels/a.nw").unwrap(), b"GLEVNW01\nNPC - 1 1\n");
        let list = f.list("levels").unwrap();
        assert_eq!(list.len(), 1, "no temporary file left: {list:?}");
        assert_eq!(list[0].name, "a.nw");
        assert!(f.list("").unwrap()[0].is_dir);
    }
}
