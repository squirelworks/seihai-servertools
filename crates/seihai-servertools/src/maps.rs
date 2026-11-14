//! `seihai-servertools level convert|render` and `gmap make|render`: the
//! offline LAT jobs of the classic converters, map makers and NW2PNG
//! (`lat_core::level`, `lat_core::worldmap`, `lat_core::render`).

use lat_core::render::{self, OwnedSheet};
use lat_core::worldmap::{self, Built, Gmap};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub const LEVEL_USAGE: &str = "\
level convert IN OUT.nw|OUT.graal|OUT.zelda|OUT.tmx [--sheet PNG]
    Reads a .nw (GLEVNW01), .graal (GR-V1) or .zelda (Z3-V1) level, chosen
    by the file's first bytes, or a Tiled .tmx map, and writes the format
    OUT's extension names: GLEVNW01 text, GR-V1.03, Z3-V1.04 or a Tiled
    map (one 16-pixel tile layer per board layer on the tile sheet --sheet
    names, default pics1.png beside the map; links, signs, NPCs, baddies
    and chests as objects carrying their level lines in an nw property).
    A .tmx is read back from its CSV layers and those nw properties. Board, links, signs, baddies, NPCs
    and chests are kept; comments and a GR-V1.04 HEIGHTS block are not.
    A binary level has one layer, and a .zelda has no NPCs or chests and
    numbered link arrivals; what is dropped is printed as a warning.
level render IN OUT.png [--sheet PNG] [--scale N] [--npcs] [--images DIR]
    Paints the level's board (every layer) to a PNG, 1024 pixels a side at
    --scale 1 (default), or 1024/N with N = 2, 4, 8 or 16. --sheet is the
    tile sheet (default: Seihai's built-in pics1.png). --npcs also draws
    each visible level NPC's image (.png, .gif or .mng, first frame) at
    its place, found by name in --images DIR (default: the level's folder
    and below), any letter case. Signs, chests and NPCs drawn as a gani
    are not drawn.";

pub const GMAP_USAGE: &str = "\
gmap make OUT.gmap --width N LEVEL...
    Puts the levels row by row in a grid N wide (`-` is a hole).
gmap make OUT.gmap --names DIR
    Places every level in DIR by its name, <world><column><row>.nw
    (world-a01.nw, worldaa12.nw), as the generators name them.
gmap make OUT.gmap --links DIR START
    Walks the edge links from level START in DIR (a link along the left,
    right, top or bottom edge leads to the neighbour there) and puts the
    linked world on one grid.
    Each prints the grid size and what it could not place.
gmap render IN.gmap OUT.png [--dir DIR] [--sheet PNG] [--scale N]
           [--npcs] [--images DIR]
    Paints every level of the gmap into one PNG. Levels are looked up in
    --dir (default: the gmap's folder and below), any letter case. --npcs
    and --images as for level render (images default to --dir).
    --scale defaults to 4 (256 pixels a level).";

fn fail(cmd: &str, msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("{cmd}: {msg}");
    ExitCode::FAILURE
}

#[derive(Default)]
struct Opts {
    plain: Vec<String>,
    sheet: Option<PathBuf>,
    scale: Option<u32>,
    width: Option<usize>,
    names: Option<PathBuf>,
    links: bool,
    dir: Option<PathBuf>,
    npcs: bool,
    images: Option<PathBuf>,
}

fn parse(args: &[&str]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        let mut value = || it.next().copied().ok_or(format!("{a} needs a value"));
        match a {
            "--sheet" => o.sheet = Some(value()?.into()),
            "--dir" => o.dir = Some(value()?.into()),
            "--names" => o.names = Some(value()?.into()),
            "--links" => o.links = true,
            "--npcs" => o.npcs = true,
            "--images" => {
                o.images = Some(value()?.into());
                o.npcs = true;
            }
            "--scale" => {
                let v = value()?;
                o.scale = Some(
                    v.parse()
                        .map_err(|_| format!("--scale {v}: not a number"))?,
                );
            }
            "--width" => {
                let v = value()?;
                o.width = Some(
                    v.parse()
                        .map_err(|_| format!("--width {v}: not a number"))?,
                );
            }
            "-" => o.plain.push("-".into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => o.plain.push(s.to_string()),
        }
    }
    Ok(o)
}

fn sheet(o: &Opts) -> Result<OwnedSheet, String> {
    match &o.sheet {
        Some(p) => {
            let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
            OwnedSheet::from_png(&bytes)
        }
        None => OwnedSheet::builtin(),
    }
}

fn read_level(path: &Path) -> Result<lat_core::level::Level, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    lat_core::level::open(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn run_level(args: &[&str]) -> ExitCode {
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (*c, r),
        None => return usage(LEVEL_USAGE),
    };
    let o = match parse(rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("level: {e}");
            return usage(LEVEL_USAGE);
        }
    };
    let result = match (cmd, o.plain.as_slice()) {
        ("convert", [src, dst]) => convert(Path::new(src), Path::new(dst), &o),
        ("render", [src, dst]) => render_level(Path::new(src), Path::new(dst), &o),
        _ => return usage(LEVEL_USAGE),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail("level", e),
    }
}

fn usage(text: &str) -> ExitCode {
    eprintln!("{text}");
    ExitCode::from(2)
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn convert(src: &Path, dst: &Path, o: &Opts) -> Result<(), String> {
    use lat_core::level::BinaryLevel;
    let format = match extension(dst).as_str() {
        "nw" => None,
        "graal" => Some(Some(BinaryLevel::Graal)),
        "zelda" => Some(Some(BinaryLevel::Zelda)),
        "tmx" => Some(None),
        _ => {
            return Err(format!(
                "{}: the output must end in .nw, .graal, .zelda or .tmx",
                dst.display()
            ))
        }
    };
    let level = if extension(src) == "tmx" {
        let bytes = std::fs::read(src).map_err(|e| format!("{}: {e}", src.display()))?;
        let (level, warnings) = lat_core::tmx::from_tmx(&String::from_utf8_lossy(&bytes))
            .map_err(|e| format!("{}: {e}", src.display()))?;
        for w in warnings {
            eprintln!("level: {}: {w}", src.display());
        }
        level
    } else {
        read_level(src)?
    };
    let bytes = match format {
        None => lat_core::level::encode(&level),
        Some(None) => {
            let sheet = o.sheet.as_ref().map_or("pics1.png".to_string(), |p| {
                p.to_string_lossy().into_owned()
            });
            lat_core::tmx::to_tmx(&level, &sheet).into_bytes()
        }
        Some(Some(f)) => {
            let (bytes, warnings) = lat_core::level::encode_binary(&level, f);
            for w in warnings {
                eprintln!("level: {}: {w}", dst.display());
            }
            bytes
        }
    };
    write(dst, &bytes)?;
    println!(
        "{} -> {}: {} board runs, {} links, {} signs, {} baddies, {} NPCs, {} chests",
        src.display(),
        dst.display(),
        level.boards.len(),
        level.links.len(),
        level.signs.len(),
        level.baddies.len(),
        level.npcs.len(),
        level.chests.len()
    );
    Ok(())
}

fn render_level(src: &Path, dst: &Path, o: &Opts) -> Result<(), String> {
    let level = read_level(src)?;
    let sheet = sheet(o)?;
    let folder = src
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let images = image_index(o, folder);
    let mut load = images.as_ref().map(image_loader);
    let (img, missing) = render::level_rgba(
        &level,
        sheet.sheet(),
        o.scale.unwrap_or(1),
        load.as_mut()
            .map(|f| f as &mut dyn FnMut(&str) -> Option<lat_core::preview::Rgba>),
    )?;
    write(dst, &render::png_bytes(&img)?)?;
    println!("{}: {} by {}", dst.display(), img.width, img.height);
    for name in &missing {
        println!("  missing image: {name}");
    }
    Ok(())
}

pub fn run_gmap(args: &[&str]) -> ExitCode {
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (*c, r),
        None => return usage(GMAP_USAGE),
    };
    let o = match parse(rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("gmap: {e}");
            return usage(GMAP_USAGE);
        }
    };
    let result = match cmd {
        "make" => make(&o),
        "render" => match o.plain.as_slice() {
            [src, dst] => render_gmap(Path::new(src), Path::new(dst), &o),
            _ => return usage(GMAP_USAGE),
        },
        _ => return usage(GMAP_USAGE),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail("gmap", e),
    }
}

/// Every level file under `dir` (and below), by lower-case file name. The
/// first one found for a name wins.
fn level_index(dir: &Path) -> HashMap<String, PathBuf> {
    file_index(dir, &["nw", "graal", "zelda"])
}

/// Every file under `dir` (and below) whose extension is one of `exts`, by
/// lower-case file name. The first one found for a name wins.
pub(crate) fn file_index(dir: &Path, exts: &[&str]) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                stack.push(p);
            } else if exts.contains(&extension(&p).as_str()) {
                if let Some(name) = p.file_name() {
                    out.entry(name.to_string_lossy().to_ascii_lowercase())
                        .or_insert(p);
                }
            }
        }
    }
    out
}

fn loader(
    index: &HashMap<String, PathBuf>,
) -> impl FnMut(&str) -> Option<lat_core::level::Level> + '_ {
    |name| {
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
        index
            .get(&base.to_ascii_lowercase())
            .and_then(|p| read_level(p).ok())
    }
}

/// NPC images for `--npcs`: every picture under `--images DIR`, or else
/// under `default`, looked up by file name in any letter case.
fn image_index(o: &Opts, default: &Path) -> Option<HashMap<String, PathBuf>> {
    o.npcs.then(|| {
        file_index(
            o.images.as_deref().unwrap_or(default),
            &["png", "gif", "mng"],
        )
    })
}

pub(crate) fn image_loader(
    index: &HashMap<String, PathBuf>,
) -> impl FnMut(&str) -> Option<lat_core::preview::Rgba> + '_ {
    |name| {
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let bytes = std::fs::read(index.get(&base.to_ascii_lowercase())?).ok()?;
        render::decode_image(&bytes)
    }
}

fn make(o: &Opts) -> Result<(), String> {
    let Some((out, rest)) = o.plain.split_first() else {
        return Err("name the .gmap to write".into());
    };
    let built: Built = match (o.width, &o.names, o.links) {
        (Some(w), None, false) => worldmap::from_list(rest, w)?,
        (None, Some(dir), false) if rest.is_empty() => {
            let mut names: Vec<String> = level_index(dir)
                .into_values()
                .filter(|p| p.parent() == Some(dir.as_path()))
                .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .collect();
            names.sort();
            worldmap::from_names(&names)?
        }
        (None, None, true) => {
            let [dir, start] = rest else {
                return Err("--links needs DIR and START".into());
            };
            let index = level_index(Path::new(dir));
            worldmap::from_links(start, loader(&index))?
        }
        _ => return Err("use one of --width N LEVEL..., --names DIR, --links DIR START".into()),
    };
    write(Path::new(out), worldmap::write(&built.gmap).as_bytes())?;
    let filled = built.gmap.names.iter().filter(|n| !n.is_empty()).count();
    println!(
        "{out}: {} by {}, {filled} levels",
        built.gmap.width, built.gmap.height
    );
    for note in &built.notes {
        println!("  {note}");
    }
    Ok(())
}

fn render_gmap(src: &Path, dst: &Path, o: &Opts) -> Result<(), String> {
    let text = std::fs::read_to_string(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let gmap = Gmap::parse(&text).ok_or(format!("{}: not a GRMAP001 file", src.display()))?;
    let dir = o
        .dir
        .clone()
        .or_else(|| {
            src.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf)
        })
        .unwrap_or_else(|| PathBuf::from("."));
    let index = level_index(&dir);
    let sheet = sheet(o)?;
    let images = image_index(o, &dir);
    let mut load = images.as_ref().map(image_loader);
    let (img, report) = render::gmap_rgba(
        &gmap,
        sheet.sheet(),
        o.scale.unwrap_or(4),
        loader(&index),
        load.as_mut()
            .map(|f| f as &mut dyn FnMut(&str) -> Option<lat_core::preview::Rgba>),
    )?;
    write(dst, &render::png_bytes(&img)?)?;
    println!(
        "{}: {} by {}, {} levels painted",
        dst.display(),
        img.width,
        img.height,
        report.painted
    );
    for name in &report.missing {
        println!("  missing: {name}");
    }
    for name in &report.missing_images {
        println!("  missing image: {name}");
    }
    Ok(())
}
