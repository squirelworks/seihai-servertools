//! `seihai-servertools code`: read and write client `.code` caches.
//!
//! Logic is [`lat_core::code_cache`]. This file only parses the command line
//! and prints.

use std::path::Path;
use std::process::ExitCode;

use lat_core::code_cache::{self, Cache};
use lat_core::gpak::{self, Key};

pub const USAGE: &str = "\
code show FILE [--key HEX16] [--level NAME]
    GWEBL001 (a level cache): host, signature, mod time, level name, body size.
    The level name defaults to the file name (start.nw-14802.code is start.nw).
    GRESCR01 (a script cache): the CRC32 of the plain body. Needs --key.
code crc FILE --key HEX16
    Prints that CRC32 and nothing else: the integer a 197 header carries.
code dump FILE --out PLAIN [--key HEX16] [--level NAME]
    Writes the plain level text, or the plain script body.
code pack PLAIN --out FILE [--key HEX16]
    A file that starts with GWEBL001 is packed as a level cache.
    Any other plain file is a script body and needs --key.
Hosts in files are whatever the cache stored (a lab file uses 127.0.0.1
or <host>). This command does not open a connection.
";

fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("code: {msg}");
    ExitCode::FAILURE
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

struct Opts {
    plain: Vec<String>,
    key: Option<Key>,
    level: Option<String>,
    out: Option<String>,
}

fn parse(args: &[&str]) -> Result<Opts, String> {
    let mut o = Opts {
        plain: Vec::new(),
        key: None,
        level: None,
        out: None,
    };
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        let mut value = || it.next().copied().ok_or(format!("{a} needs a value"));
        match a {
            "--key" => {
                let v = value()?;
                o.key = Some(gpak::parse_key(v).ok_or("--key takes 16 hex digits")?);
            }
            "--level" => o.level = Some(value()?.to_string()),
            "--out" => o.out = Some(value()?.to_string()),
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
            eprintln!("code: {e}");
            return usage();
        }
    };
    match cmd {
        "show" => show(&o),
        "crc" => crc(&o),
        "dump" => dump(&o),
        "pack" => pack(&o),
        _ => usage(),
    }
}

fn read_file(path: &str) -> Result<Vec<u8>, ExitCode> {
    std::fs::read(path).map_err(|e| fail(format!("{path}: {e}")))
}

fn open_file(o: &Opts, file: &str) -> Result<Cache, ExitCode> {
    let data = read_file(file)?;
    let owned = o
        .level
        .clone()
        .unwrap_or_else(|| code_cache::level_from_file_name(file));
    code_cache::open(&data, Some(&owned), o.key.as_ref()).map_err(fail)
}

fn show(o: &Opts) -> ExitCode {
    let Some(file) = o.plain.first() else {
        return usage();
    };
    if o.plain.len() != 1 {
        return usage();
    }
    match open_file(o, file) {
        Ok(cache) => {
            print!("{}", code_cache::show_text(&cache));
            ExitCode::SUCCESS
        }
        Err(code) => code,
    }
}

fn crc(o: &Opts) -> ExitCode {
    let Some(file) = o.plain.first() else {
        return usage();
    };
    if o.plain.len() != 1 || o.key.is_none() {
        return usage();
    }
    match open_file(o, file) {
        Ok(Cache::Script(script)) => {
            println!("{}", script.crc32);
            ExitCode::SUCCESS
        }
        Ok(Cache::Level(_)) => fail("crc is for a GRESCR01 script cache"),
        Err(code) => code,
    }
}

fn dump(o: &Opts) -> ExitCode {
    let Some(file) = o.plain.first() else {
        return usage();
    };
    if o.plain.len() != 1 {
        return usage();
    }
    let Some(out) = o.out.as_deref() else {
        return usage();
    };
    let plain = match open_file(o, file) {
        Ok(Cache::Level(level)) => match code_cache::level_plaintext(&level) {
            Ok(b) => b,
            Err(e) => return fail(e),
        },
        Ok(Cache::Script(script)) => script.body,
        Err(code) => return code,
    };
    match std::fs::write(out, plain) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(format!("{out}: {e}")),
    }
}

fn pack(o: &Opts) -> ExitCode {
    let Some(file) = o.plain.first() else {
        return usage();
    };
    if o.plain.len() != 1 {
        return usage();
    }
    let Some(out) = o.out.as_deref() else {
        return usage();
    };
    let data = match read_file(file) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let packed = if data.starts_with(b"GWEBL001") {
        match code_cache::parse_level_plaintext(&data)
            .and_then(|level| code_cache::write_level(&level))
        {
            Ok(b) => b,
            Err(e) => return fail(e),
        }
    } else {
        let Some(key) = o.key.as_ref() else {
            return fail("packing a script needs --key");
        };
        code_cache::write_script(&data, key)
    };
    match std::fs::write(Path::new(out), packed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(format!("{out}: {e}")),
    }
}
