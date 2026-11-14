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
//! file browser shows them. `..` and absolute paths are refused. Local
//! paths also resolve existing links and must remain inside the world.
//! This check assumes the folder is not concurrently modified by an
//! adversary: checking a path and opening it are separate operations.
//!
//! The file link's directory reply is names only: no size and no folder
//! flag. [`RcFiles::list`] therefore reports `size` 0 and `is_dir` false.

use std::fs;
use std::io;
use std::io::Write;
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
        let relative = safe_relative(path)?;
        let root = fs::canonicalize(&self.root)?;
        let mut existing = root.join(relative);
        let mut missing = Vec::new();
        loop {
            match fs::symlink_metadata(&existing) {
                Ok(_) => break,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    missing.push(existing.file_name().ok_or(e)?.to_owned());
                    existing.pop();
                }
                Err(e) => return Err(e),
            }
        }
        let mut resolved = fs::canonicalize(existing)?;
        if !resolved.starts_with(&root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "path outside the world",
            ));
        }
        for part in missing.into_iter().rev() {
            resolved.push(part);
        }
        Ok(resolved)
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
        safe_relative(path)?;
        fs::create_dir_all(&self.root)?;
        let target = self.resolve(path)?;
        let dir = target.parent().unwrap_or(&self.root).to_path_buf();
        fs::create_dir_all(&dir)?;
        let mut tmp = tempfile::Builder::new()
            .prefix(".servertools-")
            .suffix(".tmp")
            .tempfile_in(&dir)?;
        tmp.write_all(data)?;
        tmp.persist(&target).map(|_| ()).map_err(|e| e.error)
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
            identity: None,
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
        self.session
            .put_file(path, data, RC_TIMEOUT)
            .map_err(io_file)
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

/// What [`put_if_unchanged`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Upload {
    /// The server still had the downloaded bytes, and `data` replaced them.
    Written,
    /// The server's copy is no longer the downloaded bytes. Nothing was
    /// written. This is what the server has now.
    Changed(Vec<u8>),
}

/// Upload `data` only if `path` still holds `downloaded`, the bytes the
/// editor opened.
///
/// The file is downloaded again just before the upload and compared. The
/// file link has no compare-and-swap, so a write that lands between that
/// download and the upload is not seen; the window is one round trip.
pub fn put_if_unchanged(
    files: &mut dyn RemoteFiles,
    path: &str,
    downloaded: &[u8],
    data: &[u8],
) -> io::Result<Upload> {
    let now = files.get(path)?;
    if now != downloaded {
        return Ok(Upload::Changed(now));
    }
    files.put(path, data)?;
    Ok(Upload::Written)
}

/// [`put_if_unchanged`] on `source`, with one login for both calls.
pub fn write_file_if_unchanged(
    source: &FileSource,
    path: &str,
    downloaded: &[u8],
    data: &[u8],
) -> io::Result<Upload> {
    put_if_unchanged(source.open()?.as_mut(), path, downloaded, data)
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

    #[test]
    fn upload_is_refused_when_the_copy_changed() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = LocalFolder::new(dir.path());
        f.put("levels/a.nw", b"one\n").unwrap();
        let opened = f.get("levels/a.nw").unwrap();

        f.put("levels/a.nw", b"someone else\n").unwrap();
        let got = put_if_unchanged(&mut f, "levels/a.nw", &opened, b"mine\n").unwrap();
        assert_eq!(got, Upload::Changed(b"someone else\n".to_vec()));
        assert_eq!(f.get("levels/a.nw").unwrap(), b"someone else\n");

        let opened = f.get("levels/a.nw").unwrap();
        let got = put_if_unchanged(&mut f, "levels/a.nw", &opened, b"mine\n").unwrap();
        assert_eq!(got, Upload::Written);
        assert_eq!(f.get("levels/a.nw").unwrap(), b"mine\n");
    }

    #[test]
    fn upload_of_a_removed_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = LocalFolder::new(dir.path());
        assert!(put_if_unchanged(&mut f, "levels/gone.nw", b"x", b"y").is_err());
    }

    #[test]
    fn saves_preserve_unrelated_temp_names_and_clean_up_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = LocalFolder::new(dir.path());
        let unrelated = dir.path().join(".victim.txt.servertools-tmp");
        fs::write(&unrelated, b"unrelated").unwrap();
        fs::write(dir.path().join("victim.txt"), b"old").unwrap();
        f.put("victim.txt", b"complete replacement").unwrap();
        assert_eq!(fs::read(&unrelated).unwrap(), b"unrelated");
        assert_eq!(f.get("victim.txt").unwrap(), b"complete replacement");
        let blocked = dir.path().join("blocked");
        fs::create_dir(&blocked).unwrap();
        fs::write(blocked.join("keep.txt"), b"old target").unwrap();
        let before = fs::read_dir(dir.path()).unwrap().count();
        assert!(f.put("blocked", b"cannot replace a directory").is_err());
        assert_eq!(fs::read(blocked.join("keep.txt")).unwrap(), b"old target");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), before);
        assert_eq!(fs::read(&unrelated).unwrap(), b"unrelated");
    }

    #[test]
    fn concurrent_saves_use_independent_temporary_files() {
        let dir = tempfile::tempdir().unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let jobs: Vec<_> = [b'a', b'b']
            .into_iter()
            .map(|byte| {
                let root = dir.path().to_path_buf();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let mut f = LocalFolder::new(root);
                    let data = vec![byte; 65536];
                    barrier.wait();
                    f.put("same.txt", &data).unwrap();
                })
            })
            .collect();
        for job in jobs {
            job.join().unwrap();
        }
        let saved = fs::read(dir.path().join("same.txt")).unwrap();
        assert!(saved == vec![b'a'; 65536] || saved == vec![b'b'; 65536]);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[cfg(any(unix, windows))]
    fn directory_link(target: &Path, link: &Path) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, link).unwrap();
        #[cfg(windows)]
        {
            // Directory junctions do not require symlink privileges.
            let out = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .output()
                .unwrap();
            assert!(out.status.success(), "junction creation failed: {:?}", out);
        }
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn existing_links_cannot_escape_the_world_for_reads_or_writes() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("world");
        let outside = dir.path().join("outside");
        fs::create_dir(&world).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("sentinel.txt"), b"outside").unwrap();
        let link = world.join("escape");
        directory_link(&outside, &link);
        let mut f = LocalFolder::new(&world);
        assert_eq!(
            f.get("escape/sentinel.txt").unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(f.list("escape").is_err());
        assert!(f.put("escape/sentinel.txt", b"overwritten").is_err());
        assert!(f.put("escape/missing/nested.txt", b"created").is_err());
        assert_eq!(fs::read(outside.join("sentinel.txt")).unwrap(), b"outside");
        assert!(!outside.join("missing").exists());
        // Remove the link itself before the temporary fixture is dropped.
        #[cfg(unix)]
        fs::remove_file(&link).unwrap();
        #[cfg(windows)]
        fs::remove_dir(&link).unwrap();
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn links_within_the_world_still_work() {
        let dir = tempfile::tempdir().unwrap();
        let actual = dir.path().join("actual");
        fs::create_dir(&actual).unwrap();
        let link = dir.path().join("alias");
        directory_link(&actual, &link);
        let mut f = LocalFolder::new(dir.path());
        f.put("alias/new/sub.txt", b"inside").unwrap();
        assert_eq!(f.get("alias/new/sub.txt").unwrap(), b"inside");
        assert_eq!(fs::read(actual.join("new/sub.txt")).unwrap(), b"inside");
        #[cfg(unix)]
        fs::remove_file(&link).unwrap();
        #[cfg(windows)]
        fs::remove_dir(&link).unwrap();
    }
}
