//! `seihai-servertools`: the command line. It parses options, calls
//! `setup-core` / `lat-core`, prints, and sets the exit code. It is the only
//! place that prints or reads the environment (see docs/ARCHITECTURE.md).

mod code;
mod gpak;
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
  seihai-servertools check [--json] WORLD
  seihai-servertools setup status|public|lab|undo [--world DIR] [SERVER]
                                    take a gserver folder public or back to the lab
                                    (`setup --help` for the options)
  seihai-servertools code show|crc|dump|pack FILE  (`.code` caches; `code --help`)
  seihai-servertools gpak pack FOLDER OUT [--server DIR] [--key HEX16] [--keys FILE]
  seihai-servertools gpak unpack PACKAGE DIR [--key HEX16 | --keys FILE | --server DIR]
                                    resource packages (.gpak, .gpake) for 4.20 and 6.x
                                    clients (`gpak --help`)
  seihai-servertools gui [--world DIR] [--page lab|setup|levels|npcs|ganis|check]
                                    open the window
  seihai-servertools --version | --help

lab start: starts the listserver (if one is found) and then the gservers of a
lab world (default: servers/default and servers/classic), each console in a
log file, and stops them all on Ctrl+C. --world defaults to the current folder.

check: reads every level, gmap and gani under WORLD and reports parse errors,
links or gmap cells that point at missing levels, and images or sounds a
level NPC or a gani names that are not in the folder. It also runs the setup
checks on every servers/<name>/config/serveroptions.txt (and the accounts/ of
public folders) and on seihai-listserver.toml: a setting `setup` would refuse
to save is an error. Exit code 1 on errors (a missing image or sound is a
warning). --json prints one JSON object instead, for CI: format (1), the
counts (levels, gmaps, ganis, errors, warnings) and findings, each with level
(error, warning, note), source (world or settings), file, key (or null) and
text.";

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
        ["check", world] => check(Path::new(world), false),
        ["check", "--json", world] | ["check", world, "--json"] => check(Path::new(world), true),
        ["setup", rest @ ..] => setup::run(rest),
        ["code", "--help" | "-h"] => {
            println!("{}", code::USAGE);
            ExitCode::SUCCESS
        }
        ["code", rest @ ..] => code::run(rest),
        ["gpak", "--help" | "-h"] => {
            println!("{}", gpak::USAGE);
            ExitCode::SUCCESS
        }
        ["gpak", rest @ ..] => gpak::run(rest),
        ["gui", rest @ ..] => gui(rest),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// One line of the `check` report.
struct Line {
    /// `error`, `warning` or `note`.
    level: &'static str,
    /// `world` (levels, gmaps, ganis) or `settings` (the setup checks).
    source: &'static str,
    file: String,
    key: Option<String>,
    text: String,
}

fn check(world: &Path, json: bool) -> ExitCode {
    if !world.is_dir() {
        eprintln!("check: no folder {}", world.display());
        return ExitCode::from(2);
    }
    let report = lat_core::check::check_world(world);
    let mut lines: Vec<Line> = report
        .findings
        .iter()
        .map(|f| Line {
            level: match f.severity {
                lat_core::check::Severity::Error => "error",
                lat_core::check::Severity::Warn => "warning",
            },
            source: "world",
            file: f.file.clone(),
            key: None,
            text: f.text.clone(),
        })
        .collect();
    // The setup checks: a setting `setup` would refuse to save is an error.
    for c in setup_core::folder::check_world_config(world) {
        lines.push(Line {
            level: match c.finding.severity {
                setup_core::checks::Severity::Block => "error",
                setup_core::checks::Severity::Warn => "warning",
                setup_core::checks::Severity::Info => "note",
            },
            source: "settings",
            file: c.file,
            key: c.finding.key,
            text: c.finding.text,
        });
    }
    let count = |level: &str| lines.iter().filter(|l| l.level == level).count();
    let (errors, warnings) = (count("error"), count("warning"));
    if json {
        let findings: Vec<String> = lines
            .iter()
            .map(|l| {
                format!(
                    "{{\"level\":{},\"source\":{},\"file\":{},\"key\":{},\"text\":{}}}",
                    json_str(l.level),
                    json_str(l.source),
                    json_str(&l.file),
                    l.key.as_deref().map(json_str).unwrap_or("null".into()),
                    json_str(&l.text)
                )
            })
            .collect();
        println!(
            "{{\"format\":1,\"levels\":{},\"gmaps\":{},\"ganis\":{},\"errors\":{errors},\"warnings\":{warnings},\"findings\":[{}]}}",
            report.levels,
            report.gmaps,
            report.ganis,
            findings.join(",")
        );
    } else {
        for l in &lines {
            let key = l
                .key
                .as_deref()
                .map(|k| format!(" [{k}]"))
                .unwrap_or_default();
            println!("{}: {}{key}: {}", l.level, l.file, l.text);
        }
        println!(
            "checked {} levels, {} gmaps, {} ganis and the server settings: {errors} errors, {warnings} warnings",
            report.levels, report.gmaps, report.ganis,
        );
    }
    if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// A JSON string literal.
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
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

#[cfg(test)]
mod tests {
    use super::json_str;

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(json_str("plain"), "\"plain\"");
        assert_eq!(
            json_str("a \"b\" c\\d\ne\tf\u{1}"),
            "\"a \\\"b\\\" c\\\\d\\ne\\tf\\u0001\""
        );
        assert_eq!(json_str("Grüße"), "\"Grüße\"");
    }
}
