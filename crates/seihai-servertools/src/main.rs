//! `seihai-servertools`: every server tool in one program. Each command is
//! its own crate and binary under `tools/` (`seihai-lab`, `seihai-check`,
//! ...); this file only hands the words after the command name to that
//! tool's `run`, so `seihai-servertools gpak pack ...` and `seihai-gpak pack
//! ...` are the same thing (see docs/ARCHITECTURE.md).

use std::process::ExitCode;

const USAGE: &str = "\
seihai-servertools: Seihai server tools

Usage:
  seihai-servertools guide [TOPIC]  how the servers fit together, every tool, setting up
                                    from scratch, RC, troubleshooting (`guide --help`)
  seihai-servertools setup new [DIR] [options]
                                    a new server folder from scratch: listservers, gservers,
                                    NPC-servers, staff accounts for RC, start/stop scripts
  seihai-servertools lab start [--world DIR] [--listserver PATH | --no-listserver]
                               [--gserver PATH] [--rust DIR]... [--rust-gserver PATH]
                               [SERVER...]
  seihai-servertools lab plan  (same options; prints what start would run)
  seihai-servertools check [--json] [--client LABEL] [--client-reach] [--make-gif] WORLD
  seihai-servertools check --list-clients
  seihai-servertools setup status|public|lab|undo [--world DIR] [SERVER]
                                    take a gserver folder public or back to the lab
                                    (`setup --help` for the options)
  seihai-servertools code show|crc|dump|pack FILE  (`.code` caches; `code --help`)
  seihai-servertools gpak pack FOLDER OUT [--server DIR] [--key HEX16] [--keys FILE]
  seihai-servertools gpak unpack PACKAGE DIR [--key HEX16 | --keys FILE | --server DIR]
                                    resource packages (.gpak, .gpake) for 4.20 and 6.x
                                    clients (`gpak --help`)
  seihai-servertools level convert IN OUT.nw | render IN OUT.png [--sheet PNG] [--scale N]
                                    convert .graal/.zelda/.nw/Tiled .tmx levels, paint a level (`level --help`)
  seihai-servertools level fill THEME.nwt LEVEL... (--out DIR | --in-place) [--seed N]
                                    LevelFiller: scatter a theme's objects (`level fill --help`)
  seihai-servertools theme make RECIPE.tm OUT.nwt [--seed N]
                                    LFThemeMaker: patch themes from a recipe (`theme --help`)
  seihai-servertools gmap make OUT.gmap (--width N LEVEL... | --names DIR | --links DIR START)
  seihai-servertools gmap render IN.gmap OUT.png [--dir DIR] [--sheet PNG] [--scale N]
                                    build a gmap from levels, paint a whole gmap (`gmap --help`)
  seihai-servertools gani movie show FILE [--frame N] | render FILE OUT.png [--frame N]
                                    GraalShop movies (GANI0002): list actors and keyframes,
                                    paint a timeline frame (`gani --help`)
  seihai-servertools gen terrain|3d|image|dungeon ...
                                    generate a world: height field, its 3D version from a
                                    gmap, from a picture, or a dungeon (`gen --help`)
  seihai-servertools stats ACCOUNTS OUTDIR [MODE...]
                                    leaderboard pages (kills, deaths, rupees, highap, lowap,
                                    maxpower, power, arrows, bombs, glovepower, swordpower,
                                    shieldpower; default all) from seihai-gserver accounts
  seihai-servertools gif2mng [-b] [--out DIR] [--force] GIF...
                                    each GIF as a .png (one frame) or .mng animation beside
                                    it or in DIR; -b makes palette index 2 transparent
  seihai-servertools pw3d INPUT OUT [...]
                                    a 2D playerworld as 3D worlds for seihai-gserver
                                    (`pw3d --help`)
  seihai-servertools edit3d FOLDER [--world NAME] | show|add|move|remove|spawn|terrain|render ...
                                    the 3D level editor: a world file and its terrain, with a
                                    live preview (`edit3d --help`)
  seihai-servertools gui [--world DIR] [--page lab|setup|levels|npcs|database|ganis|packages|check]
                                    open the window
  seihai-servertools --version | --help

Every command is also its own program, built and run on its own:
seihai-guide, seihai-lab, seihai-setup, seihai-check, seihai-code, seihai-gpak,
seihai-level, seihai-theme, seihai-gmap, seihai-gani, seihai-gen,
seihai-stats, seihai-gif2mng, seihai-pw3d, seihai-edit3d and seihai-servertools-gui take the same
words after the command name (`seihai-gpak pack ...`). `seihai-servertools
COMMAND --help` and `seihai-COMMAND --help` print a command's options.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        // No words: the guide's overview, so a first run explains itself.
        [] => {
            let code = seihai_guide::run(&[]);
            println!("\nEvery command: seihai-servertools --help");
            code
        }
        ["--help" | "-h"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["--version" | "-V"] => {
            println!("seihai-servertools {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [tool, rest @ ..] => match dispatch(tool, rest) {
            Some(code) => code,
            None => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        },
    }
}

/// Runs `tool` with the words after it, or `None` for no such command.
fn dispatch(tool: &str, rest: &[&str]) -> Option<ExitCode> {
    Some(match tool {
        "guide" => seihai_guide::run(rest),
        "lab" => seihai_lab::run(rest),
        "setup" => seihai_setup::run(rest),
        "check" => seihai_check::run(rest),
        "code" => seihai_code::run(rest),
        "gpak" => seihai_gpak::run(rest),
        "level" => seihai_level::run(rest),
        "theme" => seihai_theme::run(rest),
        "gmap" => seihai_gmap::run(rest),
        "gani" => seihai_gani::run(rest),
        "gen" => seihai_gen::run(rest),
        "regions" => seihai_regions::run(rest),
        "stats" => seihai_stats::run(rest),
        "gif2mng" => seihai_gif2mng::run(rest),
        "pw3d" => seihai_pw3d::run(rest),
        "edit3d" => seihai_edit3d::run(rest),
        "gui" => gui(rest),
        _ => return None,
    })
}

#[cfg(feature = "gui")]
fn gui(args: &[&str]) -> ExitCode {
    seihai_servertools_gui::run(args)
}

#[cfg(not(feature = "gui"))]
fn gui(_: &[&str]) -> ExitCode {
    eprintln!("gui: this build has no window (built without the `gui` feature)");
    ExitCode::from(2)
}
