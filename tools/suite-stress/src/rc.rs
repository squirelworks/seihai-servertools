//! `seihai-rc` against the running suite: the RC list from the listserver
//! and the player list from the gserver, timed. Each probe runs the real
//! `seihai-rc` binary, so what is measured is what an admin would see.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// One RC probe's result.
#[derive(Debug, Clone)]
pub struct RcProbe {
    /// `servers` or `players list`.
    pub what: &'static str,
    pub took: Duration,
    pub ok: bool,
    /// The first useful line of output: a row count or the error.
    pub note: String,
}

fn run(rc: &Path, what: &'static str, args: &[&str]) -> RcProbe {
    let t = Instant::now();
    let out = Command::new(rc)
        .args(args)
        .env("SEIHAI_RC_PASSWORD", crate::LAB_PASSWORD)
        .stdin(Stdio::null())
        .output();
    let took = t.elapsed();
    match out {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            let err = String::from_utf8_lossy(&o.stderr);
            let rows = text.lines().filter(|l| !l.trim().is_empty()).count();
            let note = if o.status.success() {
                format!("{rows} line(s)")
            } else {
                err.lines()
                    .chain(text.lines())
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("no output")
                    .trim()
                    .to_string()
            };
            RcProbe {
                what,
                took,
                ok: o.status.success(),
                note,
            }
        }
        Err(e) => RcProbe {
            what,
            took,
            ok: false,
            note: format!("could not run seihai-rc: {e}"),
        },
    }
}

/// The RC list from the listserver (`seihai-rc servers`).
pub fn servers(rc: &Path, listserver: &str, account: &str) -> RcProbe {
    run(
        rc,
        "servers",
        &[
            "servers",
            "--listserver",
            listserver,
            "--account",
            account,
            "--timeout",
            "10",
        ],
    )
}

/// The player list from the gserver (`seihai-rc players list`).
pub fn players(rc: &Path, gserver: &str, account: &str) -> RcProbe {
    run(
        rc,
        "players list",
        &[
            "players",
            "list",
            "--server",
            gserver,
            "--account",
            account,
            "--tsv",
            "--timeout",
            "10",
            "--settle",
            "500",
        ],
    )
}
