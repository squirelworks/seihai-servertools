//! `seihai-stats` (also `seihai-servertools stats`): leaderboard pages from
//! seihai-gserver accounts (`setup_core::stats`, gserverstat's job).

use std::path::Path;
use std::process::ExitCode;

/// Help for `seihai-stats`.
pub const USAGE: &str = "\
stats ACCOUNTS OUTDIR [MODE...]
    Leaderboard pages (one MODE.html each) from a seihai-gserver world's
    accounts folder. Modes: kills, deaths, rupees, highap, lowap, maxpower,
    power, arrows, bombs, glovepower, swordpower, shieldpower (default all).
    Passwords are never read into a page.";

/// Runs one command line: the words after `seihai-stats` (or after
/// `seihai-servertools stats`).
pub fn run(args: &[&str]) -> ExitCode {
    match args {
        ["--help" | "-h"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [accounts, out, modes @ ..] => stats(accounts, out, modes),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn stats(accounts: &str, out: &str, modes: &[&str]) -> ExitCode {
    use setup_core::stats;
    let chosen: Vec<stats::Mode> = if modes.is_empty() {
        stats::MODES.to_vec()
    } else {
        let mut v = Vec::new();
        for m in modes {
            match stats::mode(m) {
                Some(m) => v.push(m),
                None => {
                    eprintln!("stats: unknown mode {m}");
                    return ExitCode::from(2);
                }
            }
        }
        v
    };
    let entries = match stats::read_accounts(Path::new(accounts)) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("stats: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("stats: {out}: {e}");
        return ExitCode::FAILURE;
    }
    for m in chosen {
        let path = Path::new(out).join(format!("{}.html", m.name));
        let html = stats::page(m, &stats::rank(&entries, m));
        if let Err(e) = std::fs::write(&path, html) {
            eprintln!("stats: {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("{}", path.display());
    }
    println!("{} accounts ranked", entries.len());
    ExitCode::SUCCESS
}
