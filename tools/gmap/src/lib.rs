//! `seihai-gmap make|render` (also `seihai-servertools gmap`): build a gmap
//! from levels and paint a whole gmap (`lat_core::worldmap`,
//! `lat_core::render`).

use lat_core::files::{self, CharacterPics};
use lat_core::gani::Gani;
use lat_core::render;
use lat_core::worldmap::{self, Built, Gmap};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Help for `seihai-gmap`.
pub const USAGE: &str = "\
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
    --dir (default: the gmap's folder and below), any letter case. Chests
    and signs as for level render. --npcs and --images as for level render
    (files default to --dir). --scale defaults to 4 (256 pixels a level).";

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

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Runs one command line: the words after `seihai-gmap` (or after
/// `seihai-servertools gmap`): `make` or `render`.
pub fn run(args: &[&str]) -> ExitCode {
    if matches!(args, ["--help" | "-h"]) {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (*c, r),
        None => return usage(USAGE),
    };
    let o = match parse(rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("gmap: {e}");
            return usage(USAGE);
        }
    };
    let result = match cmd {
        "make" => make(&o),
        "render" => match o.plain.as_slice() {
            [src, dst] => render_gmap(Path::new(src), Path::new(dst), &o),
            _ => return usage(USAGE),
        },
        _ => return usage(USAGE),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail("gmap", e),
    }
}

fn usage(text: &str) -> ExitCode {
    eprintln!("{text}");
    ExitCode::from(2)
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

fn make(o: &Opts) -> Result<(), String> {
    let Some((out, rest)) = o.plain.split_first() else {
        return Err("name the .gmap to write".into());
    };
    let built: Built = match (o.width, &o.names, o.links) {
        (Some(w), None, false) => worldmap::from_list(rest, w)?,
        (None, Some(dir), false) if rest.is_empty() => {
            let mut names: Vec<String> = files::level_index(dir)
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
            let index = files::level_index(Path::new(dir));
            worldmap::from_links(start, files::level_loader(&index))?
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
    let index = files::level_index(&dir);
    let sheet = files::sheet(o.sheet.as_deref())?;
    let images = image_index(o, &dir);
    let mut load = images.as_ref().map(files::image_loader);
    let mut ganis = images.as_ref().map(files::gani_loader);
    let pics = match images.as_ref() {
        Some(index) => CharacterPics::from_index(index),
        None => CharacterPics::default(),
    };
    let (img, report) = render::gmap_rgba(
        &gmap,
        sheet.sheet(),
        o.scale.unwrap_or(4),
        files::level_loader(&index),
        load.as_mut()
            .map(|f| f as &mut dyn FnMut(&str) -> Option<lat_core::preview::Rgba>),
        ganis
            .as_mut()
            .map(|f| f as &mut dyn FnMut(&str) -> Option<Gani>),
        pics.view(),
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
    report_missing(&report.missing_images);
    Ok(())
}
