//! `seihai-check WORLD` (also `seihai-servertools check`): the world and
//! config checker. World findings come from `lat_core::check`, the setting
//! findings from `setup_core::folder`; this file only merges and prints.

use std::path::Path;
use std::process::ExitCode;

/// Help for `seihai-check`.
pub const USAGE: &str = "\
check [--json] [--client LABEL] [--client-reach] [--make-gif] WORLD
check --list-clients

check: reads every level, gmap and gani under WORLD and reports parse errors,
links or gmap cells that point at missing levels, and images or sounds a
level NPC or a gani names that are not in the folder. It also runs the setup
checks on every servers/<name>/config/serveroptions.txt (and the accounts/ of
public folders) and on seihai-listserver.toml: a setting `setup` would refuse
to save is an error. Exit code 1 on errors (a missing image or sound is a
warning). --json prints one JSON object instead, for CI: format (1), the
counts (levels, gmaps, ganis, errors, warnings) and findings, each with level
(error, warning, note), source (world or settings), file, key (or null) and
text. --client LABEL adds shared-profile level/gmap/gani/script warnings for
one build; check --list-clients lists the accepted labels. Unknown script
support remains unknown; package readability and exact per-build vocabulary
are not certified. Without --client the general inventory stays unchanged.
--client-reach also inventories level NPC image references (including
fixed GS1 script names) with PNG art but no GIF counterpart for pre-2.1 clients.
It flags parsed gani files for fallback review using shared profiles' gani
support, without claiming use or choosing a built-in animation, and lists
the profiles without gmap support or the level's format. It lists level NPC
and weapon (weapons/*.txt) scripts that are GS2 only or mix GS1 and GS2, with
the shared profiles that do not run GS2 (before 3.153), and levels with tiles
past column 116 of pics1.png, which the 1856-wide sheet of 1.28b2 to 2.05
lacks. It does not certify script, animation, tile or gmap compatibility.
--make-gif (opt-in) first writes a GIF copy next to each PNG that
--client-reach would report, keeping every pixel; it never replaces a file
and skips art with over 256 colours.";

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

/// Runs one command line: the words after `seihai-check` (or after
/// `seihai-servertools check`).
pub fn run(args: &[&str]) -> ExitCode {
    if matches!(args, ["--help" | "-h"]) {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args == ["--list-clients"] {
        for client in lat_core::check::CheckClient::all_versions() {
            println!("{}", client.label());
        }
        return ExitCode::SUCCESS;
    }
    let mut world = None;
    let mut json = false;
    let mut reach = false;
    let mut make_gif = false;
    let mut client = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match *arg {
            "--client" => {
                let selected = args
                    .next()
                    .and_then(|label| lat_core::check::CheckClient::from_label(label));
                if client.is_some() || selected.is_none() {
                    eprintln!(
                        "check: --client needs one known build label; use check --list-clients"
                    );
                    return ExitCode::from(2);
                }
                client = selected;
            }
            "--json" => json = true,
            "--client-reach" => reach = true,
            "--make-gif" => make_gif = true,
            value if !value.starts_with('-') && world.is_none() => world = Some(value),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(world) = world else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if make_gif && json {
        eprintln!("check: --make-gif prints what it wrote; run --json without it");
        return ExitCode::from(2);
    }
    if make_gif && Path::new(world).is_dir() {
        for c in lat_core::gifcopy::make_gifs(Path::new(world)) {
            match c.result {
                Ok(()) => println!("wrote {}", c.gif.display()),
                Err(e) => println!("skipped {}: {e}", c.png.display()),
            }
        }
    }
    check(Path::new(world), json, reach, client)
}

fn check(
    world: &Path,
    json: bool,
    reach: bool,
    client: Option<lat_core::check::CheckClient>,
) -> ExitCode {
    if !world.is_dir() {
        eprintln!("check: no folder {}", world.display());
        return ExitCode::from(2);
    }
    let mut report = match client {
        Some(client) => lat_core::check::check_world_for(world, client),
        None => lat_core::check::check_world(world),
    };
    if reach {
        report
            .findings
            .extend(lat_core::check::check_image_reach(world));
        report
            .findings
            .extend(lat_core::check::check_level_reach(world));
        report
            .findings
            .extend(lat_core::check::check_script_reach(world));
        report
            .findings
            .extend(lat_core::check::check_tile_reach(world));
    }
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
