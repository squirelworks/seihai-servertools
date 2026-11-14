//! `seihai-servertools gen terrain|image|dungeon`: world generators
//! (`lat_core::generate`).

use lat_core::generate::{self, DungeonTiles, File, TerrainOptions};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub const USAGE: &str = "\
gen terrain DIR --world NAME [--size WxH] [--seed N] [--base N] [--height N]
            [--chaos F] [--islands] [--force]
    A height-field world (TerrainGenerator): WxH levels (default 2x2) named
    <NAME><column><row>.nw with a 9x9 HEIGHTS block each, edge links, and
    NAME.gmap with the generator keys. --height is how far the first split
    may move a point (default 64), --chaos the factor after each split
    (default 0.5), --islands keeps the rim at --base.
gen image PICTURE DIR --world NAME
    Levels from a .png or .gif drawn in GraalLevelGen's twelve colours
    (64 pixels a level, sampled every 6): NAME_a-01.nw ... and NAME.gmap.
    Levels already in DIR are painted over; their links and NPCs stay.
gen dungeon DIR --start NAME (--layout FILE | --maze WxH) [--seed N]
            [--path TILE] [--wall TILE] [--force]
    A dungeon from an X (wall) / . (path) layout file or a maze of WxH
    cells, each cell 8 tiles wide and 11 tall, cut into levels
    NAME<column><row>.nw with edge links and NAME.gmap. Tiles default to
    path 0 and wall 1.

Without --force, terrain and dungeon refuse to replace files in DIR.";

#[derive(Default)]
struct Opts {
    plain: Vec<String>,
    world: Option<String>,
    size: Option<(usize, usize)>,
    seed: u32,
    base: i32,
    height: f64,
    chaos: f64,
    islands: bool,
    force: bool,
    layout: Option<PathBuf>,
    maze: Option<(usize, usize)>,
    path: u16,
    wall: u16,
}

fn size(v: &str) -> Result<(usize, usize), String> {
    let (w, h) = v
        .split_once(['x', 'X'])
        .ok_or(format!("{v}: use WxH, like 3x2"))?;
    let n = |s: &str| {
        s.parse::<usize>()
            .map_err(|_| format!("{v}: use WxH, like 3x2"))
    };
    Ok((n(w)?, n(h)?))
}

fn parse(args: &[&str]) -> Result<Opts, String> {
    let mut o = Opts {
        height: 64.0,
        chaos: 0.5,
        wall: 1,
        ..Opts::default()
    };
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        let mut value = || it.next().copied().ok_or(format!("{a} needs a value"));
        let num = |v: &str| {
            v.parse::<f64>()
                .map_err(|_| format!("{a} {v}: not a number"))
        };
        match a {
            "--world" | "--start" => o.world = Some(value()?.to_string()),
            "--size" => o.size = Some(size(value()?)?),
            "--maze" => o.maze = Some(size(value()?)?),
            "--layout" => o.layout = Some(value()?.into()),
            "--seed" => o.seed = num(value()?)? as u32,
            "--base" => o.base = num(value()?)? as i32,
            "--height" => o.height = num(value()?)?,
            "--chaos" => o.chaos = num(value()?)?,
            "--path" => o.path = num(value()?)?.clamp(0.0, 4095.0) as u16,
            "--wall" => o.wall = num(value()?)?.clamp(0.0, 4095.0) as u16,
            "--islands" => o.islands = true,
            "--force" => o.force = true,
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => o.plain.push(s.to_string()),
        }
    }
    Ok(o)
}

pub fn run(args: &[&str]) -> ExitCode {
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (*c, r),
        None => return usage(),
    };
    let o = match parse(rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("gen: {e}");
            return usage();
        }
    };
    let result = match (cmd, o.plain.as_slice()) {
        ("terrain", [dir]) => terrain(Path::new(dir), &o),
        ("image", [pic, dir]) => image(Path::new(pic), Path::new(dir), &o),
        ("dungeon", [dir]) => dungeon(Path::new(dir), &o),
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gen: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

fn world(o: &Opts, flag: &str) -> Result<String, String> {
    let w = o.world.clone().ok_or(format!("{flag} NAME is needed"))?;
    if w.is_empty() || w.contains(['/', '\\']) {
        return Err(format!("{flag} {w}: a plain name, without folders"));
    }
    Ok(w)
}

fn save(dir: &Path, files: &[File], force: bool) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if !force {
        if let Some((name, _)) = files.iter().find(|(n, _)| dir.join(n).exists()) {
            return Err(format!(
                "{} already exists (use --force to replace)",
                dir.join(name).display()
            ));
        }
    }
    for (name, bytes) in files {
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let levels = files.iter().filter(|(n, _)| n.ends_with(".nw")).count();
    println!("{}: {levels} levels and a gmap", dir.display());
    Ok(())
}

fn terrain(dir: &Path, o: &Opts) -> Result<(), String> {
    let (width, height) = o.size.unwrap_or((2, 2));
    let files = generate::terrain(&TerrainOptions {
        world: world(o, "--world")?,
        width,
        height,
        seed: o.seed,
        base: o.base,
        amplitude: o.height,
        chaos: o.chaos,
        even_borders: o.islands,
    })?;
    save(dir, &files, o.force)
}

fn image(pic: &Path, dir: &Path, o: &Opts) -> Result<(), String> {
    let bytes = std::fs::read(pic).map_err(|e| format!("{}: {e}", pic.display()))?;
    let (w, h, rgba) = if seihai_core_gif(&bytes) {
        lat_core::render::read_gif_rgba(&bytes)
    } else {
        lat_core::render::read_png_rgba(&bytes)
    }
    .map_err(|e| format!("{}: {e}", pic.display()))?;
    let rgb: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let files = generate::from_image(&world(o, "--world")?, &rgb, w, h, |name| {
        let bytes = std::fs::read(dir.join(name)).ok()?;
        lat_core::level::open(&bytes).ok()
    })?;
    save(dir, &files, true)
}

fn seihai_core_gif(bytes: &[u8]) -> bool {
    bytes.starts_with(b"GIF8")
}

fn dungeon(dir: &Path, o: &Opts) -> Result<(), String> {
    let layout = match (&o.layout, o.maze) {
        (Some(p), None) => {
            let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            generate::parse_layout(&text)?
        }
        (None, Some((w, h))) => generate::maze_layout(w, h, o.seed),
        _ => return Err("use --layout FILE or --maze WxH".into()),
    };
    let files = generate::dungeon(
        &world(o, "--start")?,
        &layout,
        DungeonTiles {
            path: o.path,
            wall: o.wall,
        },
    )?;
    save(dir, &files, o.force)
}
