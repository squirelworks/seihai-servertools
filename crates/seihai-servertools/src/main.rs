//! `seihai-servertools`: the command line. It parses options, calls
//! `setup-core` / `lat-core`, prints, and sets the exit code. It is the only
//! place that prints or reads the environment (see docs/ARCHITECTURE.md).

mod lab;
mod setup;

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
seihai-servertools: Seihai server tools

Usage:
  seihai-servertools lab start [--world DIR] [--listserver PATH | --no-listserver]
                               [--gserver PATH] [SERVER...]
  seihai-servertools lab plan  (same options; prints what start would run)
  seihai-servertools check WORLD
  seihai-servertools setup status|public|lab|undo [--world DIR] [SERVER]
                                    take a gserver folder public or back to the lab
                                    (`setup --help` for the options)
  seihai-servertools gui [--world DIR] [--page lab|setup|levels|npcs|ganis|check]
                                    open the window
  seihai-servertools --version | --help

lab start: starts the listserver (if one is found) and then the gservers of a
lab world (default: servers/default and servers/classic), each console in a
log file, and stops them all on Ctrl+C. --world defaults to the current folder.

check: reads every level, gmap and gani under WORLD and reports parse errors,
links or gmap cells that point at missing levels, and images or sounds a
level NPC or a gani names that are not in the folder. Exit code 1 on errors
(a missing image or sound is a warning).";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["--help" | "-h"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["--version" | "-V"] => {
            println!("seihai-servertools {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["lab", "start", rest @ ..] => lab::run(rest, true),
        ["lab", "plan", rest @ ..] => lab::run(rest, false),
        ["check", world] => check(Path::new(world)),
        ["setup", rest @ ..] => setup::run(rest),
        ["gui", rest @ ..] => gui(rest),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn check(world: &Path) -> ExitCode {
    if !world.is_dir() {
        eprintln!("check: no folder {}", world.display());
        return ExitCode::from(2);
    }
    let report = lat_core::check::check_world(world);
    for f in &report.findings {
        let level = match f.severity {
            lat_core::check::Severity::Error => "error",
            lat_core::check::Severity::Warn => "warning",
        };
        println!("{level}: {}: {}", f.file, f.text);
    }
    println!(
        "checked {} levels, {} gmaps, {} ganis: {} errors, {} warnings",
        report.levels,
        report.gmaps,
        report.ganis,
        report.errors(),
        report.findings.len() - report.errors()
    );
    if report.errors() > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(feature = "gui")]
fn gui(args: &[&str]) -> ExitCode {
    let mut opts = servertools_app::StartOptions::default();
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        match (a, it.next()) {
            ("--world", Some(w)) => opts.world = Some(w.into()),
            ("--page", Some(p)) => match servertools_app::Tab::from_name(p) {
                Some(t) => opts.tab = Some(t),
                None => {
                    eprintln!("gui: no page {p} (lab, setup, levels, npcs, ganis, check)");
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

#[cfg(not(feature = "gui"))]
fn gui(_: &[&str]) -> ExitCode {
    eprintln!("gui: this build has no window (built without the `gui` feature)");
    ExitCode::from(2)
}
