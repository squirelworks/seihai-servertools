//! Resource packages: `.gpak` (a plain ZIP) and `.gpake` (the same ZIP with
//! every entry DES-encrypted), as the 4.20 and 6.x clients load them.
//!
//! What the clients do with a package (lab runs 2026-11-06, Graal_analysis
//! `gpak-sample` notes):
//!
//! - A package is an ordinary PKZIP archive, stored or deflate, no ZIP
//!   password. The client saves a downloaded one under `levels/gpaks/` and
//!   adds every entry to its file table under the entry's lower-case file
//!   name, as if the file lay loose in the levels folder. At most 10 000
//!   entries are read, an entry over 20 MiB is skipped and an entry name has
//!   at most 255 bytes.
//! - In a `.gpake` only the *uncompressed contents* of each entry are
//!   encrypted: DES-ECB over every full 8-byte block, the short tail left
//!   plain, no padding. The ZIP sizes and CRC describe the encrypted bytes.
//! - The client's DES reads each key byte with its bits reversed (bit 0 ↔
//!   bit 7 and so on); the block cipher is otherwise standard. So a package
//!   for the key `K` the server sends is encrypted with standard DES under
//!   `K` with every byte bit-reversed ([`des_key`]). Checked against the
//!   real 6.03r7 and 4.20r8 clients: they decrypt and draw a package made
//!   this way, and refuse one made with a wrong key.
//! - The server sends the key in `PLO 104` before it sends the package:
//!   `GSTRING name` (lower case, no path), `GCHAR 1`, then key bytes 0–3 and
//!   4–7, each read as a little-endian 32-bit value, as `GINT5`. The gserver
//!   writes that packet; [`plo104`] is here for the tests and the docs.
//!
//! Which clients load what (real clients, 2026-11-06): 6.03r7 loads `.gpak`
//! and `.gpake`. 4.20r8 loads `.gpake` and `.zip` but saves a plain `.gpak`
//! without ever loading it. 4.04r2 loads no packages. Clients before 4.x
//! have no packages at all.
//!
//! Keys live in the gserver folder's `config/gpakkeys.txt` (never in the
//! `world/` folder, which is served): one `name.gpake HEX16` line per package, the
//! hex being the key as `PLO 104` sends it. [`set_key`] edits that file line
//! by line.

use des::cipher::generic_array::GenericArray;
use des::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

/// The most entries a client reads from one package.
pub const MAX_ENTRIES: usize = 10_000;
/// Entries bigger than this (uncompressed) are skipped by the client.
pub const MAX_ENTRY_SIZE: usize = 0x140_0000;
/// The client's entry-name buffer holds 255 bytes and a NUL.
pub const MAX_NAME: usize = 255;

/// An 8-byte package key, in the order `PLO 104` sends it.
pub type Key = [u8; 8];

/// One file in a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The name inside the package: `/` between folders, lower case.
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Zip(String),
    /// A package rule the client would break on, with the reason.
    Invalid(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Zip(e) => write!(f, "zip: {e}"),
            Error::Invalid(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::Zip(e.to_string())
    }
}

/// Whether a package name is encrypted: it ends in `.gpake`.
pub fn is_encrypted(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".gpake")
}

/// The standard-DES key that does what the client's DES does with `key`:
/// every byte bit-reversed.
pub fn des_key(key: &Key) -> Key {
    key.map(u8::reverse_bits)
}

fn crypt(data: &[u8], key: &Key, encrypt: bool) -> Vec<u8> {
    let cipher = des::Des::new(GenericArray::from_slice(&des_key(key)));
    let mut out = data.to_vec();
    let full = out.len() & !7;
    for block in out[..full].chunks_exact_mut(8) {
        let b = GenericArray::from_mut_slice(block);
        if encrypt {
            cipher.encrypt_block(b);
        } else {
            cipher.decrypt_block(b);
        }
    }
    out
}

/// One entry's contents as a `.gpake` holds them.
pub fn encrypt_entry(data: &[u8], key: &Key) -> Vec<u8> {
    crypt(data, key, true)
}

/// The inverse of [`encrypt_entry`]: what the client reads.
pub fn decrypt_entry(data: &[u8], key: &Key) -> Vec<u8> {
    crypt(data, key, false)
}

/// A fresh random key.
pub fn random_key() -> Result<Key, Error> {
    let mut k = [0u8; 8];
    getrandom::getrandom(&mut k).map_err(|e| Error::Invalid(format!("no random key: {e}")))?;
    Ok(k)
}

/// A key from 16 hex digits.
pub fn parse_key(hex: &str) -> Option<Key> {
    let hex = hex.trim();
    if hex.len() != 16 || !hex.is_ascii() {
        return None;
    }
    let mut k = [0u8; 8];
    for (i, b) in k.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(k)
}

/// A key as 16 upper-case hex digits.
pub fn key_hex(key: &Key) -> String {
    key.iter().map(|b| format!("{b:02X}")).collect()
}

/// Every file under `folder` (any depth), named by its path below it,
/// lower case, `/` between folders, in name order.
pub fn collect(folder: &Path) -> Result<Vec<Entry>, Error> {
    let mut out = Vec::new();
    let mut dirs = vec![PathBuf::new()];
    while let Some(rel) = dirs.pop() {
        let mut items: Vec<_> = std::fs::read_dir(folder.join(&rel))?.collect::<Result<_, _>>()?;
        items.sort_by_key(|e| e.file_name());
        for e in items {
            let kind = e.file_type()?;
            let Some(n) = e.file_name().to_str().map(str::to_owned) else {
                return Err(Error::Invalid(format!(
                    "{}: the name is not UTF-8",
                    e.path().display()
                )));
            };
            let r = rel.join(&n);
            if kind.is_dir() {
                dirs.push(r);
            } else if kind.is_file() {
                let name = r
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push(Entry {
                    name,
                    data: std::fs::read(e.path())?,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Check the entries against what the client reads, and lower-case their
/// names. Two entries whose file names differ only in case, or share a
/// file name in different folders, would hide one another in the client's
/// file table; that is an error too.
fn prepare(entries: &[Entry]) -> Result<Vec<Entry>, Error> {
    if entries.is_empty() {
        return Err(Error::Invalid("a package needs at least one file".into()));
    }
    if entries.len() > MAX_ENTRIES {
        return Err(Error::Invalid(format!(
            "{} files; the client reads at most {MAX_ENTRIES}",
            entries.len()
        )));
    }
    let mut seen = std::collections::HashMap::new();
    let mut out = Vec::with_capacity(entries.len());
    for e in entries {
        let name = e.name.replace('\\', "/").to_lowercase();
        if name.is_empty() || name.starts_with('/') || name.split('/').any(|p| p == "..") {
            return Err(Error::Invalid(format!("{}: not a file name", e.name)));
        }
        if name.len() > MAX_NAME {
            return Err(Error::Invalid(format!(
                "{name}: the name is longer than the client's {MAX_NAME} bytes"
            )));
        }
        if e.data.len() > MAX_ENTRY_SIZE {
            return Err(Error::Invalid(format!(
                "{name}: {} bytes; the client skips files over {MAX_ENTRY_SIZE}",
                e.data.len()
            )));
        }
        let file = name.rsplit('/').next().unwrap_or(&name).to_string();
        if let Some(other) = seen.insert(file.clone(), name.clone()) {
            return Err(Error::Invalid(format!(
                "{other} and {name}: the client finds files by name alone, so one hides the other"
            )));
        }
        out.push(Entry {
            name,
            data: e.data.clone(),
        });
    }
    Ok(out)
}

/// Write a package. With a key, every entry is encrypted (a `.gpake`);
/// without, it is a plain `.gpak` (or `.zip`). Entries are deflated.
pub fn write<W: Write + Seek>(out: W, entries: &[Entry], key: Option<&Key>) -> Result<(), Error> {
    let entries = prepare(entries)?;
    let mut z = zip::ZipWriter::new(out);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        // A fixed time, so the same files make the same package.
        .last_modified_time(zip::DateTime::default());
    for e in &entries {
        z.start_file(e.name.as_str(), opts)?;
        match key {
            Some(k) => z.write_all(&encrypt_entry(&e.data, k))?,
            None => z.write_all(&e.data)?,
        }
    }
    z.finish()?;
    Ok(())
}

/// Read a package back, decrypting each entry with `key` when given.
pub fn read<R: Read + Seek>(input: R, key: Option<&Key>) -> Result<Vec<Entry>, Error> {
    let mut z = zip::ZipArchive::new(input)?;
    let mut out = Vec::with_capacity(z.len());
    for i in 0..z.len() {
        let mut f = z.by_index(i)?;
        if f.is_dir() {
            continue;
        }
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;
        if let Some(k) = key {
            data = decrypt_entry(&data, k);
        }
        out.push(Entry {
            name: f.name()?.into_owned(),
            data,
        });
    }
    Ok(out)
}

/// `PLO 104` for a package, without the line end: `GCHAR 104`, `GSTRING`
/// name, `GCHAR 1`, `GINT5` lo, `GINT5` hi.
pub fn plo104(name: &str, key: &Key) -> Vec<u8> {
    fn gint5(v: u32, out: &mut Vec<u8>) {
        for shift in [28, 21, 14, 7, 0] {
            out.push(((v >> shift) & 0x7F) as u8 + 32);
        }
    }
    let name = name.to_ascii_lowercase();
    let mut out = vec![104 + 32, name.len().min(223) as u8 + 32];
    out.extend_from_slice(&name.as_bytes()[..name.len().min(223)]);
    out.push(1 + 32);
    gint5(
        u32::from_le_bytes([key[0], key[1], key[2], key[3]]),
        &mut out,
    );
    gint5(
        u32::from_le_bytes([key[4], key[5], key[6], key[7]]),
        &mut out,
    );
    out
}

/// Where a gserver folder (`seihai-gserver-v2`'s `servers/<name>/`) keeps
/// its packages: `world/gpaks/`. The default `foldersconfig.txt` serves
/// `gpaks/*.gpak` and `gpaks/*.gpake` from there.
pub fn server_package_path(server: &Path, name: &str) -> PathBuf {
    server.join("world").join("gpaks").join(name)
}

/// A gserver folder's key file: `config/gpakkeys.txt`, which the server
/// reads but never serves.
pub fn keys_path(server: &Path) -> PathBuf {
    server.join("config").join("gpakkeys.txt")
}

const KEYS_HEADER: &str = "\
# Keys for the encrypted packages (.gpake) in world/gpaks/. One line each:
#   name.gpake KEY
# KEY is 16 hex digits, the key the gserver sends in PLO 104 before the
# package. Keep this file private: anyone with a key can unpack its package.
";

/// The key a key file gives `package`, if any.
pub fn key_in(text: &str, package: &str) -> Option<Key> {
    text.lines().find_map(|l| {
        let mut w = l.split_whitespace();
        let (n, k) = (w.next()?, w.next()?);
        (!n.starts_with('#') && n.eq_ignore_ascii_case(package))
            .then(|| parse_key(k))
            .flatten()
    })
}

/// Set `package`'s key in the key file at `path`: its line is replaced in
/// place, or a line is added at the end. Comments, order and other lines
/// stay. Written through a temporary file and a rename; an existing file
/// is kept as `gpakkeys.txt.bak`.
pub fn set_key(path: &Path, package: &str, key: &Key) -> Result<(), Error> {
    let package = package.to_ascii_lowercase();
    if package.contains(char::is_whitespace) || !is_encrypted(&package) {
        return Err(Error::Invalid(format!(
            "{package}: a key file names .gpake packages without spaces"
        )));
    }
    let old = match std::fs::read_to_string(path) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let line = format!("{package} {}", key_hex(key));
    let mut lines: Vec<String> = old
        .as_deref()
        .unwrap_or(KEYS_HEADER)
        .lines()
        .map(str::to_owned)
        .collect();
    let mine = |l: &String| {
        l.split_whitespace()
            .next()
            .is_some_and(|n| n.eq_ignore_ascii_case(&package))
    };
    match lines.iter().position(mine) {
        Some(i) => lines[i] = line,
        None => lines.push(line),
    }
    let mut text = lines.join("\n");
    text.push('\n');
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if let Some(old) = &old {
        std::fs::write(path.with_extension("txt.bak"), old)?;
    }
    let tmp = path.with_extension("txt.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const SAMPLE: Key = [0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1];

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    /// The client's own DES, run in an emulator, turns block
    /// 0123456789ABCDEF into CEE55ACD2386B23F under 133457799BBCDFF1 and
    /// into 617B3A0CE8F07100 under the zero key. Textbook DES gives
    /// 85E813540F0AB405 for the first.
    #[test]
    fn matches_the_clients_des() {
        let block = hex("0123456789ABCDEF");
        assert_eq!(encrypt_entry(&block, &SAMPLE), hex("CEE55ACD2386B23F"));
        assert_eq!(encrypt_entry(&block, &[0; 8]), hex("617B3A0CE8F07100"));
        // The textbook vector, through the bit-reversed key.
        assert_eq!(
            encrypt_entry(&block, &des_key(&SAMPLE)),
            hex("85E813540F0AB405")
        );
        assert_eq!(key_hex(&des_key(&SAMPLE)), "C82CEA9ED93DFB8F");
    }

    #[test]
    fn short_tails_stay_plain() {
        for n in 0..18 {
            let d: Vec<u8> = (0..n as u8).collect();
            let e = encrypt_entry(&d, &SAMPLE);
            assert_eq!(e.len(), n);
            assert_eq!(e[n & !7..], d[n & !7..]);
            assert_eq!(decrypt_entry(&e, &SAMPLE), d);
        }
    }

    /// The packet the lab sent before `sample.gpake` (the README's bytes).
    #[test]
    fn plo104_matches_the_lab_packet() {
        let want = hex("882c73616d706c652e677061 6b6521276a7c88332f2e9e993b"
            .replace(' ', "")
            .as_str());
        assert_eq!(plo104("Sample.gpake", &SAMPLE), want);
    }

    #[test]
    fn packages_round_trip() {
        let entries = vec![
            Entry {
                name: "Tiles/Tile.PNG".into(),
                data: (0..300u16).map(|i| i as u8).collect(),
            },
            Entry {
                name: "note.txt".into(),
                data: b"hello, seven".to_vec(),
            },
        ];
        for key in [None, Some(&SAMPLE)] {
            let mut buf = Cursor::new(Vec::new());
            write(&mut buf, &entries, key).unwrap();
            buf.set_position(0);
            let back = read(&mut buf, key).unwrap();
            assert_eq!(back[0].name, "tiles/tile.png");
            assert_eq!(back[0].data, entries[0].data);
            assert_eq!(back[1].data, entries[1].data);
            // Read without the key, an encrypted package is not the files.
            buf.set_position(0);
            let raw = read(&mut buf, None).unwrap();
            assert_eq!(raw[0].data == entries[0].data, key.is_none());
        }
    }

    #[test]
    fn the_clients_limits_are_kept() {
        let e = |n: &str| Entry {
            name: n.into(),
            data: vec![1],
        };
        let w = |es: &[Entry]| write(Cursor::new(Vec::new()), es, None);
        assert!(w(&[]).is_err());
        assert!(w(&[e("a/x.png"), e("b/X.png")]).is_err(), "same file name");
        assert!(w(&[e("../x.png")]).is_err());
        assert!(w(&[e(&"a".repeat(256))]).is_err());
        assert!(w(&[e("a/x.png"), e("b/y.png")]).is_ok());
    }

    #[test]
    fn collect_names_files_by_their_path() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("Sub")).unwrap();
        std::fs::write(d.path().join("Sub/a.png"), b"a").unwrap();
        std::fs::write(d.path().join("b.txt"), b"b").unwrap();
        let es = collect(d.path()).unwrap();
        let names: Vec<_> = es.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Sub/a.png", "b.txt"]);
    }

    #[test]
    fn key_file_keeps_its_other_lines() {
        let d = tempfile::tempdir().unwrap();
        let p = keys_path(d.path());
        set_key(&p, "A.gpake", &SAMPLE).unwrap();
        let first = std::fs::read_to_string(&p).unwrap();
        assert!(first.starts_with("# Keys"));
        assert_eq!(key_in(&first, "a.gpake"), Some(SAMPLE));
        std::fs::write(&p, format!("{first}# mine\nb.gpake 0000000000000000\n")).unwrap();
        set_key(&p, "a.gpake", &[1; 8]).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(key_in(&text, "A.GPAKE"), Some([1; 8]));
        assert_eq!(key_in(&text, "b.gpake"), Some([0; 8]));
        assert!(text.contains("# mine\n"));
        assert_eq!(text.matches("a.gpake").count(), 1);
        assert!(p.with_extension("txt.bak").exists());
        assert!(set_key(&p, "plain.gpak", &SAMPLE).is_err());
    }
}
