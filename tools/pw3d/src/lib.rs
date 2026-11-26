//! `seihai-pw3d`: a 2D Graal playerworld as 3D worlds that seihai-gserver
//! serves to the 4.20r6, 4.20r8 and 5.x 3D clients.
//!
//! A 3D client draws none of a level's tiles, signs or NPCs
//! (Graal_analysis `torque/3d-win10-sky-prefs-scripting-512-2026-11-26.md`,
//! T3D-138), so everything has to become 3D content the client does draw:
//! `.3ds` GameObjects with colour materials, a Sky and a Sun. Each level
//! becomes one coloured `.3ds` ground (floor flat, water and lava sunk,
//! walls raised, NPCs, chests, signs and baddies as boxes), each gmap one
//! world with its levels side by side and every other level a world of its
//! own, and the `PW3DWalk` weapon ([`walker`]) walks the player, stops it at
//! walls, follows links between worlds and shows signs.
//!
//! The output is a seihai-gserver world folder: `config/graal3dworld.txt`
//! (the start level), `config/graal3dworlds/<name>.txt` (each other world,
//! named after its level or gmap so a TCP warp to that level takes a 3D
//! client along), `files/` (shapes and sky), `weapons/pw3dwalk.gs2` and the
//! 2D levels unchanged in `levels/`, plus `PW3D-REPORT.md`.
//! Everything generated is our own; the playerworld's levels are copied, so
//! the folder stays local.

pub mod convert;
pub mod shape;
pub mod walker;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub const USAGE: &str = "\
seihai-pw3d INPUT OUT [--start LEVEL] [--at X,Y] [--tileset PICS1.PNG]
            [--tiledefs FILE] [--wall-height F] [--colour-bits N] [--force]
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
    arrays.dat). --wall-height is in tiles (default 1.5); --colour-bits per
    channel (1 to 8, default 3) sets how many materials there are.
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

/// Write `files` under `out`; without `force`, refuse if any exists.
pub fn write_files(out: &Path, files: &[convert::File], force: bool) -> Result<(), String> {
    if !force {
        if let Some((name, _)) = files.iter().find(|(n, _)| out.join(n).exists()) {
            return Err(format!(
                "{} already exists (use --force to replace)",
                out.join(name).display()
            ));
        }
    }
    for (name, bytes) in files {
        let p = out.join(name);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        std::fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(())
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
