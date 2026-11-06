//! `seihai-servertools gpak pack|unpack`: resource packages for 4.20 and
//! 6.x clients (`lat_core::gpak`).

use lat_core::gpak::{self, Key};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub const USAGE: &str = "\
gpak pack FOLDER OUT [--server DIR] [--key HEX16] [--keys FILE]
    Packs every file under FOLDER. OUT ends in .gpake (encrypted), .gpak
    (plain) or .zip (plain, for 4.20, which loads .zip but not .gpak).
    --server DIR: a gserver folder (servers/<name>); OUT is written to
    DIR/world/gpaks/ and a .gpake's key to DIR/config/gpakkeys.txt, where
    the gserver finds both. Name OUT in the gserver's gpaks option
    to send it at login. Without --server, a .gpake's key goes to --keys FILE if given and
    is printed. Without --key a .gpake gets a random key.
gpak unpack PACKAGE DIR [--key HEX16 | --keys FILE | --server DIR]
    Writes the files in PACKAGE to DIR, decrypting a .gpake.";

fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("gpak: {msg}");
    ExitCode::FAILURE
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

struct Opts {
    plain: Vec<String>,
    server: Option<PathBuf>,
    key: Option<Key>,
    keys: Option<PathBuf>,
}

fn parse(args: &[&str]) -> Result<Opts, String> {
    let mut o = Opts {
        plain: Vec::new(),
        server: None,
        key: None,
        keys: None,
    };
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        let mut value = || it.next().copied().ok_or(format!("{a} needs a value"));
        match a {
            "--server" => o.server = Some(value()?.into()),
            "--keys" => o.keys = Some(value()?.into()),
            "--key" => {
                let v = value()?;
                o.key = Some(gpak::parse_key(v).ok_or("--key takes 16 hex digits")?);
            }
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => o.plain.push(s.to_string()),
        }
    }
    Ok(o)
}

pub fn run(args: &[&str]) -> ExitCode {
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (*c, r),
        None => return usage(),
    };
    let o = match parse(rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("gpak: {e}");
            return usage();
        }
    };
    match (cmd, o.plain.as_slice()) {
        ("pack", [folder, out]) => pack(Path::new(folder), out, &o),
        ("unpack", [package, dir]) => unpack(Path::new(package), Path::new(dir), &o),
        _ => usage(),
    }
}

fn pack(folder: &Path, out: &str, o: &Opts) -> ExitCode {
    let lower = out.to_ascii_lowercase();
    if ![".gpake", ".gpak", ".zip"]
        .iter()
        .any(|e| lower.ends_with(e))
    {
        return fail(format!("{out}: the name must end in .gpake, .gpak or .zip"));
    }
    let encrypted = gpak::is_encrypted(out);
    if !encrypted && (o.key.is_some() || o.keys.is_some()) {
        return fail("only a .gpake takes a key");
    }
    let (path, keys) = match &o.server {
        Some(w) => {
            if Path::new(out).components().count() != 1 {
                return fail("with --server, OUT is a file name, not a path");
            }
            (gpak::server_package_path(w, out), Some(gpak::keys_path(w)))
        }
        None => (PathBuf::from(out), o.keys.clone()),
    };
    let entries = match gpak::collect(folder) {
        Ok(e) => e,
        Err(e) => return fail(format!("{}: {e}", folder.display())),
    };
    let key = match (encrypted, o.key) {
        (false, _) => None,
        (true, Some(k)) => Some(k),
        (true, None) => match gpak::random_key() {
            Ok(k) => Some(k),
            Err(e) => return fail(e),
        },
    };
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        if let Err(e) = std::fs::create_dir_all(dir) {
            return fail(format!("{}: {e}", dir.display()));
        }
    }
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => return fail(format!("{}: {e}", path.display())),
    };
    if let Err(e) = gpak::write(std::io::BufWriter::new(file), &entries, key.as_ref()) {
        let _ = std::fs::remove_file(&path);
        return fail(e);
    }
    println!("wrote {} ({} files)", path.display(), entries.len());
    if let Some(k) = key {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match &keys {
            Some(kf) => match gpak::set_key(kf, &name, &k) {
                Ok(()) => println!("key for {name} saved in {}", kf.display()),
                Err(e) => return fail(format!("{}: {e}", kf.display())),
            },
            None => println!("key for {name}: {}", gpak::key_hex(&k)),
        }
    }
    ExitCode::SUCCESS
}

fn unpack(package: &Path, dir: &Path, o: &Opts) -> ExitCode {
    let name = package
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let key = if gpak::is_encrypted(&name) {
        let keys = o.keys.clone().or(o.server.as_deref().map(gpak::keys_path));
        let k = o.key.or_else(|| {
            let text = std::fs::read_to_string(keys?).ok()?;
            gpak::key_in(&text, &name)
        });
        match k {
            Some(k) => Some(k),
            None => {
                return fail(format!(
                    "{name} is encrypted: give --key, --keys or --server"
                ))
            }
        }
    } else {
        None
    };
    let file = match std::fs::File::open(package) {
        Ok(f) => f,
        Err(e) => return fail(format!("{}: {e}", package.display())),
    };
    let entries = match gpak::read(std::io::BufReader::new(file), key.as_ref()) {
        Ok(e) => e,
        Err(e) => return fail(format!("{}: {e}", package.display())),
    };
    for e in &entries {
        let rel = Path::new(&e.name);
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return fail(format!("{}: unsafe entry name", e.name));
        }
        let p = dir.join(rel);
        if let Some(d) = p.parent() {
            if let Err(err) = std::fs::create_dir_all(d) {
                return fail(format!("{}: {err}", d.display()));
            }
        }
        if let Err(err) = std::fs::write(&p, &e.data) {
            return fail(format!("{}: {err}", p.display()));
        }
        println!("{}: {} bytes", e.name, e.data.len());
    }
    ExitCode::SUCCESS
}
