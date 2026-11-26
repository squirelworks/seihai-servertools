//! `seihai-level convert|render|fill` (also `seihai-servertools level`): the
//! offline LAT jobs of the classic level converters, NW2PNG and LevelFiller
//! (`lat_core::level`, `lat_core::tmx`, `lat_core::render`,
//! `lat_core::levelfill`).

mod fill;

use lat_core::files::{self, CharacterPics};
use lat_core::gani::Gani;
use lat_core::render;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Help for `seihai-level` (`level fill --help` has the fill options).
pub const USAGE: &str = "\
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
    tile sheet (default: Seihai's built-in pics1.png). A chest is the 2x2
    closed or open tiles (magenta left out); one whose x or y is outside
    0 to 62 is skipped. A sign is a 2 by 1 mark with its first line, except
    a sign at 0,0, which is not drawn. --npcs also draws each visible level
    NPC: an image (.png, .gif or .mng, first frame), or a gani when the
    image ends in .gani or the script has a literal showcharacter,
    setcharani or setani (idle when showcharacter names nothing). That
    read is not a GS1 run: a command inside a quote or a comment is not
    drawn, and a name built at runtime is not drawn. Files are found by
    name in --images DIR (default: the level's
    folder and below), any letter case; body.png and head0.png fall back
    to Seihai's built-in pictures, and a stock gani such as idle.gani too.";

fn fail(cmd: &str, msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("{cmd}: {msg}");
    ExitCode::FAILURE
}

#[derive(Default)]
struct Opts {
    plain: Vec<String>,
    sheet: Option<PathBuf>,
    scale: Option<u32>,
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
            "-" => o.plain.push("-".into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => o.plain.push(s.to_string()),
        }
    }
    Ok(o)
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Runs one command line: the words after `seihai-level` (or after
/// `seihai-servertools level`): `convert`, `render` or `fill`.
pub fn run(args: &[&str]) -> ExitCode {
    match args {
        ["--help" | "-h"] => {
            println!("{USAGE}\n{}", fill::FILL_USAGE);
            return ExitCode::SUCCESS;
        }
        ["fill", "--help" | "-h"] => {
            println!("{}", fill::FILL_USAGE);
            return ExitCode::SUCCESS;
        }
        ["fill", rest @ ..] => return fill::run_fill(rest),
        _ => {}
    }
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (*c, r),
        None => return usage(USAGE),
    };
    let o = match parse(rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("level: {e}");
            return usage(USAGE);
        }
    };
    let result = match (cmd, o.plain.as_slice()) {
        ("convert", [src, dst]) => convert(Path::new(src), Path::new(dst), &o),
        ("render", [src, dst]) => render_level(Path::new(src), Path::new(dst), &o),
        _ => return usage(USAGE),
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

fn convert(src: &Path, dst: &Path, o: &Opts) -> Result<(), String> {
    use lat_core::level::BinaryLevel;
    let format = match files::extension(dst).as_str() {
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
    let level = if files::extension(src) == "tmx" {
        let bytes = std::fs::read(src).map_err(|e| format!("{}: {e}", src.display()))?;
        let (level, warnings) = lat_core::tmx::from_tmx(&String::from_utf8_lossy(&bytes))
            .map_err(|e| format!("{}: {e}", src.display()))?;
        for w in warnings {
            eprintln!("level: {}: {w}", src.display());
        }
        level
    } else {
        files::read_level(src)?
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
    let level = files::read_level(src)?;
    let sheet = files::sheet(o.sheet.as_deref())?;
    let folder = src
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let files = image_index(o, folder);
    let mut load = files.as_ref().map(files::image_loader);
    let mut ganis = files.as_ref().map(files::gani_loader);
    let pics = match files.as_ref() {
        Some(index) => CharacterPics::from_index(index),
        None => CharacterPics::default(),
    };
    let (img, missing) = render::level_rgba(
        &level,
        sheet.sheet(),
        o.scale.unwrap_or(1),
        load.as_mut()
            .map(|f| f as &mut dyn FnMut(&str) -> Option<lat_core::preview::Rgba>),
        ganis
            .as_mut()
            .map(|f| f as &mut dyn FnMut(&str) -> Option<Gani>),
        pics.view(),
    )?;
    write(dst, &render::png_bytes(&img)?)?;
    println!("{}: {} by {}", dst.display(), img.width, img.height);
    report_missing(&missing);
    Ok(())
}

/// NPC images for `--npcs`: every picture under `--images DIR`, or else
/// under `default`, looked up by file name in any letter case.
fn image_index(o: &Opts, default: &Path) -> Option<HashMap<String, PathBuf>> {
    o.npcs.then(|| {
        files::file_index(
            o.images.as_deref().unwrap_or(default),
            &["png", "gif", "mng", "gani"],
        )
    })
}

fn report_missing(names: &[String]) {
    for name in names {
        if name.to_ascii_lowercase().ends_with(".gani") {
            println!("  missing gani: {name}");
        } else {
            println!("  missing image: {name}");
        }
    }
}
