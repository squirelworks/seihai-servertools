//! The file link LAT tools work through.
//!
//! [`RemoteFiles`] is what every editor uses to read and write world files.
//!
//! - [`LocalFolder`]: a world folder on disk. Used by the tests, by the
//!   world checker, and by owners who run the gserver on the same machine.
//! - [`RcFiles`]: a live server, through `seihai-rc`'s file link
//!   (`list_dir`, `get_file`, `put_file`). This crate does not build those
//!   packets itself. The account and password come from the caller (a
//!   prompt or an environment variable), never from a built-in login.
//!
//! Paths are always `/`-separated and relative to the world root, as the RC
//! file browser shows them. `..` and absolute paths are refused, so a tool
//! can never reach outside the world.
//!
//! The file link's directory reply is names only: no size and no folder
//! flag. [`RcFiles::list`] therefore reports `size` 0 and `is_dir` false.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use seihai_rc::files::FileError;
use seihai_rc::rc::{LoginError, LoginParams, RcKind, Session};

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

/// How long one list, get or the login may take.
const RC_TIMEOUT: Duration = Duration::from_secs(5);

/// A live server's files, through `seihai-rc`.
pub struct RcFiles {
    session: Session,
}

impl RcFiles {
    /// Log in to `addr` (`host:port`) with the account the caller typed.
    ///
    /// The login is the newer RC kind (`seihai-rc`'s gen-5). `addr` is used
    /// as given: there is no built-in server list.
    pub fn connect(addr: &str, account: &str, password: &str) -> io::Result<Self> {
        let addr = addr.trim();
        if addr.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "RC address is empty",
            ));
        }
        if account.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "RC account is empty",
            ));
        }
        let params = LoginParams {
            kind: RcKind::Gen5,
            token: RcKind::Gen5.default_token().into(),
            account: account.trim().into(),
            password: password.into(),
            key: 0x35,
            nickname: None,
        };
        let session = Session::login(addr, &params, RC_TIMEOUT).map_err(io_login)?;
        Ok(RcFiles { session })
    }
}

impl RemoteFiles for RcFiles {
    fn list(&mut self, dir: &str) -> io::Result<Vec<Entry>> {
        let mut names = self.session.list_dir(dir, RC_TIMEOUT).map_err(io_file)?;
        names.sort();
        Ok(names
            .into_iter()
            .map(|name| Entry {
                name,
                is_dir: false,
                size: 0,
            })
            .collect())
    }

    fn get(&mut self, path: &str) -> io::Result<Vec<u8>> {
        self.session.get_file(path, RC_TIMEOUT).map_err(io_file)
    }

    fn put(&mut self, path: &str, data: &[u8]) -> io::Result<()> {
        self.session.put_file(path, data).map_err(io_file)
    }
}

fn io_login(err: LoginError) -> io::Error {
    match err {
        LoginError::Io(e) => e,
        LoginError::Timeout => io::Error::new(io::ErrorKind::TimedOut, "no answer from the server"),
        other => io::Error::other(other.to_string()),
    }
}

fn io_file(err: FileError) -> io::Error {
    match err {
        FileError::Io(e) => e,
        FileError::Timeout => {
            io::Error::new(io::ErrorKind::TimedOut, "no file reply from the server")
        }
        FileError::InvalidPath(why) => io::Error::new(io::ErrorKind::InvalidInput, why),
        other => io::Error::other(other.to_string()),
    }
}

/// Where an editor reads and writes: a folder on this machine, or a live
/// server. The password is only held so a call can log in; nothing here
/// prints it.
pub enum FileSource {
    /// A world folder.
    Folder(PathBuf),
    /// `addr` is `host:port`, as the owner typed it.
    Rc {
        addr: String,
        account: String,
        password: String,
    },
}

impl FileSource {
    fn open(&self) -> io::Result<Box<dyn RemoteFiles>> {
        match self {
            FileSource::Folder(root) => Ok(Box::new(LocalFolder::new(root.clone()))),
            FileSource::Rc {
                addr,
                account,
                password,
            } => Ok(Box::new(RcFiles::connect(addr, account, password)?)),
        }
    }
}

/// `dir` plus `name`, with `/`. An empty side is left out.
pub fn join_path(dir: &str, name: &str) -> String {
    let dir = dir.trim_matches('/');
    let name = name.trim_matches('/');
    if dir.is_empty() {
        name.to_string()
    } else if name.is_empty() {
        dir.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// List `dir` on `source`.
pub fn list_dir(source: &FileSource, dir: &str) -> io::Result<Vec<Entry>> {
    source.open()?.list(dir)
}

/// Download `path` from `source`.
pub fn read_file(source: &FileSource, path: &str) -> io::Result<Vec<u8>> {
    source.open()?.get(path)
}

/// Upload `data` to `path` on `source`.
pub fn write_file(source: &FileSource, path: &str, data: &[u8]) -> io::Result<()> {
    source.open()?.put(path, data)
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
