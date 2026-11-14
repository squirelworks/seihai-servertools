//! `seihai-servertools gani movie show|render`: GraalShop's movie mode
//! (`GANI0002`), read through `lat_core::gani` and painted by
//! `lat_core::movie`.

use lat_core::gani::{Gani, GaniDoc};
use lat_core::movie::MovieFiles;
use lat_core::preview::Rgba;
use lat_core::render;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub const USAGE: &str = "\
gani movie show FILE [--frame N]
    Lists a GraalShop movie (GANI0002): its timeline length and each actor
    (CHAR, SPRITE, SOUND, TEXT) with its keyframes. --frame N prints what
    every actor is at timeline frame N instead (a key keeps its last value;
    dx and dy move in a straight line between keyframes).
gani movie render FILE OUT.png [--frame N] [--size WxH] [--images DIR]
    Paints timeline frame N (default 0) to a PNG, WxH pixels (default
    640x480). SPRITE actors are drawn from the gani's sprite table and CHAR
    actors as a player through their ani gani (default idle, facing dir,
    with the head, body, sword, shield and attr pictures their keys name),
    at dx, dy in pixels, lower layer first. Pictures and ganis are found by
    name in --images DIR (default: the gani's folder and below), any letter
    case. TEXT actors are drawn in the font file their font key names
    (Arial finds arial.ttf or .otf there), else Ubuntu Light, at fontsize
    pixels. A CHAR's colours and chat are not drawn yet.";

#[derive(Default)]
struct Opts {
    plain: Vec<String>,
    frame: Option<u32>,
    size: Option<(u32, u32)>,
    images: Option<PathBuf>,
}

fn parse(args: &[&str]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .copied()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match *a {
            "--frame" => {
                let v = value("--frame")?;
                o.frame = Some(v.parse().map_err(|_| format!("--frame {v}: not a frame"))?);
            }
            "--size" => {
                let v = value("--size")?;
                let size = v
                    .split_once(['x', 'X'])
                    .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
                    .filter(|&(w, h): &(u32, u32)| {
                        (1..=8192).contains(&w) && (1..=8192).contains(&h)
                    })
                    .ok_or_else(|| format!("--size {v}: want WxH, 1 to 8192 each"))?;
                o.size = Some(size);
            }
            "--images" => o.images = Some(PathBuf::from(value("--images")?)),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s => o.plain.push(s.to_string()),
        }
    }
    Ok(o)
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

pub fn run(args: &[&str]) -> ExitCode {
    let ["movie", cmd, rest @ ..] = args else {
        return usage();
    };
    let o = match parse(rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("gani: {e}");
            return usage();
        }
    };
    let result = match (*cmd, o.plain.as_slice()) {
        ("show", [file]) => show(Path::new(file), &o),
        ("render", [file, out]) => render_frame(Path::new(file), Path::new(out), &o),
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gani: {e}");
            ExitCode::FAILURE
        }
    }
}

fn open(path: &Path) -> Result<GaniDoc, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc = GaniDoc::open(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    if doc.movie().is_none() {
        return Err(format!(
            "{}: not a movie (no GANI0002 MOVIE block)",
            path.display()
        ));
    }
    Ok(doc)
}

fn pairs(values: &[(String, String)]) -> String {
    values
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn show(path: &Path, o: &Opts) -> Result<(), String> {
    let doc = open(path)?;
    let movie = doc.movie().expect("checked in open");
    println!("{} frames, {} actors", movie.frames, movie.actors.len());
    for a in &movie.actors {
        match o.frame {
            Some(t) => println!("{} {}: {}", a.kind.word(), a.name, pairs(&a.state_at(t))),
            None => {
                println!(
                    "{} {} ({} keyframes)",
                    a.kind.word(),
                    a.name,
                    a.keyframes.len()
                );
                for k in &a.keyframes {
                    println!("  {} {}", k.frame, pairs(&k.values));
                }
            }
        }
    }
    Ok(())
}

fn render_frame(path: &Path, out: &Path, o: &Opts) -> Result<(), String> {
    let doc = open(path)?;
    let movie = doc.movie().expect("checked in open");
    let t = o.frame.unwrap_or(0);
    if t >= movie.frames.max(1) {
        return Err(format!(
            "--frame {t}: the movie has {} frames",
            movie.frames
        ));
    }
    let folder = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let root = o.images.as_deref().unwrap_or(folder);
    let mut files = Folder {
        images: crate::maps::file_index(root, &["png", "gif", "mng"]),
        ganis: crate::maps::file_index(root, &["gani"]),
        fonts: crate::maps::file_index(root, &["ttf", "otf"]),
    };
    let frame = lat_core::movie::paint_frame(
        doc.gani(),
        movie,
        t,
        o.size.unwrap_or((640, 480)),
        &mut files,
    );
    let png = render::png_bytes(&frame.image)?;
    std::fs::write(out, png).map_err(|e| format!("{}: {e}", out.display()))?;
    for m in &frame.missing {
        eprintln!("warning: {m} not found");
    }
    for s in &frame.skipped {
        eprintln!("note: {s}");
    }
    println!("frame {t} of {}: wrote {}", movie.frames, out.display());
    Ok(())
}

/// A movie's files under one folder, by lower-case file name.
struct Folder {
    images: HashMap<String, PathBuf>,
    ganis: HashMap<String, PathBuf>,
    fonts: HashMap<String, PathBuf>,
}

fn base(name: &str) -> String {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase()
}

impl MovieFiles for Folder {
    fn image(&mut self, name: &str) -> Option<Rgba> {
        render::decode_image(&std::fs::read(self.images.get(&base(name))?).ok()?)
    }

    fn gani(&mut self, name: &str) -> Option<Gani> {
        let bytes = std::fs::read(self.ganis.get(&base(name))?).ok()?;
        GaniDoc::open(&bytes).ok().map(|d| d.gani().clone())
    }

    /// `Arial` finds `arial.ttf` or `arial.otf`; a name with its extension
    /// finds that file.
    fn font(&mut self, name: &str) -> Option<Vec<u8>> {
        let b = base(name);
        let path = [b.clone(), format!("{b}.ttf"), format!("{b}.otf")]
            .iter()
            .find_map(|n| self.fonts.get(n))?;
        std::fs::read(path).ok()
    }
}
