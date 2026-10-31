//! `seihai-servertools`: the command line. It parses options, calls
//! `setup-core` / `lat-core`, prints, and sets the exit code. It is the only
//! place that prints or reads the environment (see docs/ARCHITECTURE.md).

mod lab;

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
seihai-servertools: Seihai server tools

Usage:
  seihai-servertools lab start [--world DIR] [--listserver PATH | --no-listserver]
                               [--gserver PATH] [SERVER...]
  seihai-servertools lab plan  (same options; prints what start would run)
  seihai-servertools check WORLD
  seihai-servertools setup ...      go-public setup (not built yet, see docs/NEXT.md)
  seihai-servertools gui            open the window
  seihai-servertools --version | --help

lab start: starts the listserver (if one is found) and then the gservers of a
lab world (default: servers/default and servers/classic), each console in a
log file, and stops them all on Ctrl+C. --world defaults to the current folder.

check: reads every level, gmap and gani under WORLD and reports parse errors
and links or gmap cells that point at missing levels. Exit code 1 on errors.";

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
        ["setup", ..] => {
            eprintln!("seihai-servertools setup: not built yet. The plan is in docs/NEXT.md.");
            ExitCode::from(2)
        }
        ["gui"] => gui(),
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
fn gui() -> ExitCode {
    match servertools_app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gui: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(feature = "gui"))]
fn gui() -> ExitCode {
    eprintln!("gui: this build has no window (built without the `gui` feature)");
    ExitCode::from(2)
}
