//! `seihai-servertools-gui` (also `seihai-servertools gui`): opens the
//! window (`servertools-app`).

use std::process::ExitCode;

/// Help for `seihai-servertools-gui`.
pub const USAGE: &str = "\
gui [--world DIR] [--page lab|setup|levels|npcs|database|ganis|packages|check]
    Opens the window: the lab, setup and LAT editor pages.";

/// Runs one command line: the words after `seihai-servertools-gui` (or
/// after `seihai-servertools gui`).
pub fn run(args: &[&str]) -> ExitCode {
    if matches!(args, ["--help" | "-h"]) {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let mut opts = servertools_app::StartOptions::default();
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        match (a, it.next()) {
            ("--world", Some(w)) => opts.world = Some(w.into()),
            ("--page", Some(p)) => match servertools_app::Tab::from_name(p) {
                Some(t) => opts.tab = Some(t),
                None => {
                    eprintln!(
                        "gui: no page {p} (lab, setup, levels, npcs, database, ganis, packages, check)"
                    );
                    return ExitCode::from(2);
                }
            },
            _ => {
                eprintln!("gui [--world DIR] [--page NAME]");
                return ExitCode::from(2);
            }
        }
    }
    match servertools_app::run(opts) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gui: {e}");
            ExitCode::FAILURE
        }
    }
}
