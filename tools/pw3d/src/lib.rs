//! `seihai-pw3d`: a 2D Graal playerworld as 3D worlds for seihai-gserver.
//! The conversion lives in `lat_core::pw3d`; this crate reads the command
//! line and prints the result.

use lat_core::pw3d::{convert, write_files};

use std::path::PathBuf;
use std::process::ExitCode;

pub const USAGE: &str = "\
seihai-pw3d INPUT OUT [--start LEVEL] [--at X,Y] [--tileset PICS1.PNG]
            [--tiledefs FILE] [--objects DIR] [--images DIR]... [--meaning-maps]
            [--wall-height F] [--colour-bits N] [--separate-interiors] [--force]
    Turn the 2D playerworld in INPUT (levels and gmaps, searched in
    subfolders too) into 3D worlds in OUT, a seihai-gserver world folder:
    config/graal3dworld.txt for the start level, config/graal3dworlds/ for
    every other level and gmap, files/ (one coloured .3ds a level, the
    player shape and sky), weapons/pw3dwalk.gs2 (walks the player, stops
    it at walls, takes links, shows signs), the 2D levels in levels/, and
    PW3D-REPORT.md (what came across, what was approximated, what was
    dropped). Add server.txt and accounts as for any world.
    --start names the start level (default onlinestartlocal, start, else
    the first by name), --at the start tile in it (default its middle, or
    the nearest floor). Tile colours come from Seihai's built-in tileset
    unless --tileset names a pics1.png-layout sheet; walls and water from
    Seihai's built-in tiledefs unless --tiledefs names one (text or
    arrays.dat). --objects names a folder of object sets (GOBJSET01 files
    or objects.dat, also in its tileobjects/): their stamp names say what
    each tile shows (tree, house, door, cliff, path, ...), and the 3D
    world is built from that: hills from cliffs, trees, roofed houses,
    shrubs, rocks. Without it only the tiledefs types are used.
    NPCs stand up as cut-outs of their picture (a door as a door, a lamp
    as a lamp), found by name in INPUT and any --images folder; an NPC
    shown as a gani, or whose picture is a sheet or missing, becomes a
    figure. --meaning-maps also writes PW3D-MAPS/<level>.png, each tile coloured
    by what it was read as. --wall-height is in tiles (default 1.5); --colour-bits per
    channel (1 to 8, default 3) sets how many materials there are.
    Interiors (levels a link from the start level or a gmap leads to,
    and the levels linked from them) are placed in that world, far from
    everything else, so a door is a walk with no world switch and the
    house is not drawn from outside; serve with graal3d_scope = on so it
    is not sent either. --separate-interiors keeps each a world of its own.
    Without --force, files already in OUT are not replaced.
    OUT holds copies of the playerworld's levels: keep it local.";

#[derive(Debug)]
struct Args {
    opts: convert::Options,
    out: PathBuf,
    force: bool,
}

fn parse(args: &[&str]) -> Result<Args, String> {
    let mut plain = Vec::new();
    let mut o = convert::Options::default();
    let mut force = false;
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        let mut value = || -> Result<&str, String> {
            i += 1;
            args.get(i).copied().ok_or(format!("{a} needs a value"))
        };
        match a {
            "--start" => o.start = Some(value()?.to_string()),
            "--at" => {
                let v = value()?;
                let (x, y) = v.split_once(',').ok_or(format!("--at {v}: use X,Y"))?;
                let n = |s: &str| {
                    s.trim()
                        .parse::<f32>()
                        .map_err(|_| format!("--at {v}: use X,Y"))
                };
                o.start_pos = Some((n(x)?, n(y)?));
            }
            "--tileset" => o.tileset = Some(PathBuf::from(value()?)),
            "--tiledefs" => o.tiledefs = Some(PathBuf::from(value()?)),
            "--objects" => o.objects = Some(PathBuf::from(value()?)),
            "--meaning-maps" => o.meaning_maps = true,
            "--images" => o.images.push(PathBuf::from(value()?)),
            "--wall-height" => {
                let v = value()?;
                o.wall_height = v
                    .parse()
                    .map_err(|_| format!("--wall-height {v}: a number"))?;
            }
            "--colour-bits" | "--color-bits" => {
                let v = value()?;
                o.colour_bits = v
                    .parse()
                    .ok()
                    .filter(|b| (1..=8).contains(b))
                    .ok_or(format!("--colour-bits {v}: 1 to 8"))?;
            }
            "--separate-interiors" => o.separate_interiors = true,
            "--force" => force = true,
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s => plain.push(s),
        }
        i += 1;
    }
    let [input, out] = plain[..] else {
        return Err("give INPUT and OUT".into());
    };
    o.input = PathBuf::from(input);
    Ok(Args {
        opts: o,
        out: PathBuf::from(out),
        force,
    })
}

/// The command line: `seihai-pw3d ...` or `seihai-servertools pw3d ...`.
pub fn run(args: &[&str]) -> ExitCode {
    if args.is_empty() || matches!(args, ["--help" | "-h" | "help", ..]) {
        println!("{USAGE}");
        return if args.is_empty() {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }
    let a = match parse(args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("seihai-pw3d: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let c = match convert::convert(&a.opts) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("seihai-pw3d: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = write_files(&a.out, &c.files, a.force) {
        eprintln!("seihai-pw3d: {e}");
        return ExitCode::FAILURE;
    }
    for w in &c.warnings {
        eprintln!("warning: {w}");
    }
    let links: usize = c.worlds.iter().map(|w| w.links.len()).sum();
    println!(
        "{} levels in {} worlds ({} links) written to {}; see PW3D-REPORT.md.",
        c.levels.len(),
        c.worlds.len(),
        links,
        a.out.display()
    );
    ExitCode::SUCCESS
}
