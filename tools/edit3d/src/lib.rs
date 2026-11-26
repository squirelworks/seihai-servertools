//! `seihai-edit3d`: a standalone 3D level editor for seihai-gserver worlds.
//! The world file, terrain, shapes and preview live in `lat_core::world3d`;
//! this crate reads the command line, saves files and runs the window.

#[cfg(feature = "window")]
mod window;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use lat_core::render::png_bytes;
use lat_core::world3d::doc::{fmt_num, Object, Turn};
use lat_core::world3d::view::{self, Camera};
use lat_core::world3d::{world_names, Brush, World};
use setup_core::config::backup;
use setup_core::stamp::Stamp;

pub const USAGE: &str = "\
seihai-edit3d FOLDER [--world NAME] [--renderer CMD]
    Open the 3D level editor on a seihai-gserver world folder (the one
    with config/). --world picks config/graal3dworlds/NAME.txt; the
    default is config/graal3dworld.txt. A world that does not exist yet
    opens empty and is created on save.
    In the window: objects listed on the left, the selected one's place
    and turn on the right, the preview in the middle. Right-drag turns
    the view, middle-drag (or shift+right-drag) moves it, the wheel
    zooms. Left-click picks an object, places the chosen shape (Place)
    or paints the terrain brush (Terrain). Save writes the world file
    (and the terrain, if it changed) with a dated backup of each.

seihai-edit3d worlds FOLDER
    The worlds in a folder: default, then config/graal3dworlds/.
seihai-edit3d show FOLDER [--world NAME]
    The world's objects, spawn, sun, sky, terrain and water, and what
    the server would warn about.
seihai-edit3d add FOLDER NAME SHAPE X Y Z [--turn DEG] [--ground] [--for ACCOUNTS]
seihai-edit3d move FOLDER NAME X Y Z [--turn DEG] [--ground]
seihai-edit3d remove FOLDER NAME
seihai-edit3d spawn FOLDER X Y Z [--ground]
    Change the world file (all take --world NAME). --ground makes Z a
    height above the terrain under X Y. --turn turns the object about
    +z, clockwise seen from above, as the clients do. --for makes an
    objectfor line, sent only to those accounts (comma-separated).
seihai-edit3d terrain FOLDER raise|lower|flatten|smooth|paint X Y RADIUS VALUE
    Change the terrain the world's terrain line names, around world X Y
    within RADIUS units: raise or lower by VALUE units at the centre,
    flatten to height VALUE, smooth with strength VALUE (0 to 1), or
    paint material number VALUE (a line of heights.info, from 0).
seihai-edit3d render FOLDER OUT.png [--world NAME] [--size WxH]
              [--eye X,Y,Z --yaw DEG --pitch DEG] [--fov DEG] [--renderer CMD]
    Draw the world to a PNG. Without --eye the view looks down at the
    spawn. --renderer runs another renderer in place of the built-in
    one (see README.md, \"Another renderer\").

Every change keeps the rest of the world file as written: comments,
order, unknown lines.";

pub fn run(args: &[&str]) -> ExitCode {
    if args.is_empty() || matches!(args, ["--help" | "-h" | "help"]) {
        println!("{USAGE}");
        return if args.is_empty() {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        };
    }
    match go(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("seihai-edit3d: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Positional words and `--flag value` / `--switch` options.
struct Args<'a> {
    pos: Vec<&'a str>,
    opts: Vec<(&'a str, Option<&'a str>)>,
}

const SWITCHES: [&str; 1] = ["--ground"];

impl<'a> Args<'a> {
    fn parse(words: &[&'a str]) -> Result<Self, String> {
        let mut pos = Vec::new();
        let mut opts = Vec::new();
        let mut i = 0;
        while i < words.len() {
            let w = words[i];
            // A negative number is a value, not a flag.
            if w.starts_with("--") {
                if SWITCHES.contains(&w) {
                    opts.push((w, None));
                } else {
                    let v = words
                        .get(i + 1)
                        .ok_or_else(|| format!("{w} needs a value"))?;
                    opts.push((w, Some(*v)));
                    i += 1;
                }
            } else {
                pos.push(w);
            }
            i += 1;
        }
        Ok(Args { pos, opts })
    }

    fn get(&self, name: &str) -> Option<&'a str> {
        self.opts
            .iter()
            .rev()
            .find(|(n, _)| *n == name)
            .and_then(|(_, v)| *v)
    }

    fn has(&self, name: &str) -> bool {
        self.opts.iter().any(|(n, _)| *n == name)
    }

    fn check(&self, allowed: &[&str]) -> Result<(), String> {
        for (n, _) in &self.opts {
            if !allowed.contains(n) {
                return Err(format!("{n} is not an option here (see --help)"));
            }
        }
        Ok(())
    }
}

fn num(s: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("`{s}` is not a number"))
}

fn open(a: &Args<'_>, folder: &str) -> Result<World, String> {
    World::open(Path::new(folder), a.get("--world").unwrap_or("default"))
}

/// `--ground` over no terrain keeps Z as given (the ground is z = 0), which
/// for a world whose floor is a shape (a seihai-pw3d level) puts the thing
/// inside that shape: say so rather than do it silently.
fn ground_note(w: &World, x: f64, y: f64) {
    if w.terrain.as_ref().and_then(|t| t.height_at(x, y)).is_none() {
        eprintln!("note: --ground: no terrain under {x} {y}, so Z stays as given (above z = 0)");
    }
}

/// The world file with a dated backup, and the terrain folder (backed up
/// as a dated copy) when `terrain` is set. Returns what was written.
pub fn save(w: &World, terrain: bool) -> Result<Vec<String>, String> {
    let stamp = Stamp::now();
    let mut said = Vec::new();
    let files = w.save_files(terrain)?;
    if terrain {
        if let Some(dir) = w.terrain_dir().filter(|d| d.is_dir()) {
            let name = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            // Not beside the folder: that is usually under the served
            // files, and a copy there would be served and listed too.
            let copy = w
                .root
                .join("backups")
                .join(format!("{name}.bak-{}", stamp.compact()));
            copy_dir(&dir, &copy).map_err(|e| format!("{}: {e}", copy.display()))?;
            said.push(format!("terrain backup {}", copy.display()));
        }
    }
    for (i, (path, bytes)) in files.iter().enumerate() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        if i == 0 {
            if let Some(b) =
                backup::save(path, bytes, stamp).map_err(|e| format!("{}: {e}", path.display()))?
            {
                said.push(format!("backup {}", b.display()));
            }
            said.push(format!("wrote {}", path.display()));
        } else {
            backup::write_atomic(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    if files.len() > 1 {
        said.push(format!("wrote {} terrain files", files.len() - 1));
    }
    Ok(said)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        if e.path().is_file() {
            fs::copy(e.path(), to.join(e.file_name()))?;
        }
    }
    Ok(())
}

fn print_saved(said: &[String]) {
    for s in said {
        println!("{s}");
    }
}

fn go(args: &[&str]) -> Result<(), String> {
    let (cmd, rest) = match args[0] {
        "worlds" | "show" | "add" | "move" | "remove" | "spawn" | "terrain" | "render" => {
            (args[0], &args[1..])
        }
        _ => ("open", args),
    };
    let a = Args::parse(rest)?;
    let folder = *a.pos.first().ok_or("name a world folder (see --help)")?;
    match cmd {
        "open" => {
            a.check(&["--world", "--renderer"])?;
            if a.pos.len() != 1 {
                return Err(format!("`{}` is not a command (see --help)", a.pos[0]));
            }
            let w = open(&a, folder)?;
            open_window(w, a.get("--renderer").map(str::to_string))
        }
        "worlds" => {
            a.check(&[])?;
            for n in world_names(Path::new(folder)) {
                println!("{n}");
            }
            Ok(())
        }
        "show" => {
            a.check(&["--world"])?;
            let w = open(&a, folder)?;
            print!("{}", describe(&w));
            Ok(())
        }
        "add" | "move" => {
            a.check(&["--world", "--turn", "--ground", "--for"])?;
            let need = if cmd == "add" { 6 } else { 5 };
            if a.pos.len() != need {
                return Err(format!("{cmd}: wrong number of words (see --help)"));
            }
            let mut w = open(&a, folder)?;
            let name = a.pos[1];
            let p = &a.pos[need - 3..];
            let (x, y, mut z) = (num(p[0])?, num(p[1])?, num(p[2])?);
            if a.has("--ground") {
                ground_note(&w, x, y);
                z += w.ground(x, y);
            }
            let turn = a
                .get("--turn")
                .map(num)
                .transpose()?
                .map(|d| Turn::about_z(d.to_radians()));
            if cmd == "add" {
                w.doc.add_object(&Object {
                    only_for: a.get("--for").map(str::to_string),
                    name: name.to_string(),
                    shape: a.pos[2].to_string(),
                    pos: [x, y, z],
                    turn,
                    rest: Vec::new(),
                })?;
            } else {
                let mut o = w
                    .doc
                    .object(name)
                    .ok_or_else(|| format!("no object named {name}"))?;
                o.pos = [x, y, z];
                if turn.is_some() {
                    o.turn = turn;
                }
                w.doc.set_object(name, &o)?;
            }
            print_saved(&save(&w, false)?);
            Ok(())
        }
        "remove" => {
            a.check(&["--world"])?;
            if a.pos.len() != 2 {
                return Err("remove: FOLDER NAME".into());
            }
            let mut w = open(&a, folder)?;
            w.doc.remove_object(a.pos[1])?;
            print_saved(&save(&w, false)?);
            Ok(())
        }
        "spawn" => {
            a.check(&["--world", "--ground"])?;
            if a.pos.len() != 4 {
                return Err("spawn: FOLDER X Y Z".into());
            }
            let mut w = open(&a, folder)?;
            let (x, y, mut z) = (num(a.pos[1])?, num(a.pos[2])?, num(a.pos[3])?);
            if a.has("--ground") {
                ground_note(&w, x, y);
                z += w.ground(x, y);
            }
            w.doc.set_spawn([x, y, z]);
            print_saved(&save(&w, false)?);
            Ok(())
        }
        "terrain" => {
            a.check(&["--world"])?;
            if a.pos.len() != 6 {
                return Err("terrain: FOLDER BRUSH X Y RADIUS VALUE".into());
            }
            let mut w = open(&a, folder)?;
            let v = num(a.pos[5])?;
            let brush = match a.pos[1] {
                "raise" => Brush::Raise(v),
                "lower" => Brush::Raise(-v),
                "flatten" => Brush::Flatten(v),
                "smooth" => Brush::Smooth(v),
                "paint" => {
                    let names = w.terrain.as_ref().map_or(0, |t| t.names.len());
                    if v < 0.0 || v.fract() != 0.0 || v as usize >= names.max(1) {
                        return Err(format!(
                            "paint: material {v} is not one of heights.info's {names}"
                        ));
                    }
                    Brush::Paint(v as u8)
                }
                b => {
                    return Err(format!(
                        "`{b}` is not a brush (raise, lower, flatten, smooth, paint)"
                    ))
                }
            };
            let (x, y, r) = (num(a.pos[2])?, num(a.pos[3])?, num(a.pos[4])?);
            let err = w.terrain_error.clone();
            let t = w.terrain.as_mut().ok_or_else(|| {
                err.unwrap_or_else(|| "the world has no terrain line".to_string())
            })?;
            let n = t.apply(brush, x, y, r);
            println!("{n} squares changed");
            // Nothing changed: no backups and no rewrite of 256 chunks.
            if n == 0 {
                println!("nothing to save");
                return Ok(());
            }
            print_saved(&save(&w, true)?);
            Ok(())
        }
        "render" => {
            a.check(&[
                "--world",
                "--size",
                "--eye",
                "--yaw",
                "--pitch",
                "--fov",
                "--renderer",
            ])?;
            if a.pos.len() != 2 {
                return Err("render: FOLDER OUT.png".into());
            }
            let mut w = open(&a, folder)?;
            let (width, height) = match a.get("--size") {
                Some(s) => {
                    let (x, y) = s.split_once('x').ok_or("--size is WxH, like 800x500")?;
                    let p = |v: &str| {
                        v.parse::<u32>()
                            .ok()
                            .filter(|n| (16..=4096).contains(n))
                            .ok_or("--size: 16 to 4096 a side")
                    };
                    (p(x)?, p(y)?)
                }
                None => (800, 500),
            };
            let mut cam = w.default_camera();
            if let Some(e) = a.get("--eye") {
                let v: Vec<f64> = e.split(',').map(num).collect::<Result<_, _>>()?;
                if v.len() != 3 {
                    return Err("--eye is X,Y,Z".into());
                }
                cam.eye = [v[0], v[1], v[2]];
            }
            if let Some(d) = a.get("--yaw") {
                cam.yaw = num(d)?.to_radians();
            }
            if let Some(d) = a.get("--pitch") {
                cam.pitch = num(d)?.to_radians();
            }
            if let Some(d) = a.get("--fov") {
                cam.fov = num(d)?.clamp(10.0, 150.0);
            }
            let out = PathBuf::from(a.pos[1]);
            let png = match a.get("--renderer") {
                Some(cmd) => external_render(cmd, &w, &cam, width, height)?,
                None => {
                    let (scene, notes) = w.scene(1);
                    for n in notes {
                        eprintln!("note: {n}");
                    }
                    png_bytes(&view::render(&scene, &cam, width, height).image)?
                }
            };
            fs::write(&out, png).map_err(|e| format!("{}: {e}", out.display()))?;
            println!("wrote {}", out.display());
            Ok(())
        }
        _ => unreachable!(),
    }
}

#[cfg(feature = "window")]
fn open_window(w: World, renderer: Option<String>) -> Result<(), String> {
    window::run(w, renderer).map_err(|e| format!("the window: {e}"))
}

#[cfg(not(feature = "window"))]
fn open_window(_: World, _: Option<String>) -> Result<(), String> {
    Err(
        "this build has no window (built without the `window` feature); \
         the edit and render commands work"
            .into(),
    )
}

/// Run another renderer: `CMD WORLD_FILE ROOT OUT.png EX EY EZ YAW PITCH
/// FOV W H` (angles in degrees, the conventions of `view::Camera`). It
/// writes OUT.png; its bytes are returned.
pub fn external_render(
    cmd: &str,
    w: &World,
    cam: &Camera,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let dir = std::env::temp_dir().join(format!("seihai-edit3d-{}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // The renderer draws what is being edited, saved or not.
    let world = dir.join("world.txt");
    fs::write(&world, w.doc.text()).map_err(|e| e.to_string())?;
    let out = dir.join("view.png");
    let _ = fs::remove_file(&out);
    let status = Command::new(cmd)
        .arg(&world)
        .arg(&w.root)
        .arg(&out)
        .args([
            fmt_num(cam.eye[0]),
            fmt_num(cam.eye[1]),
            fmt_num(cam.eye[2]),
            fmt_num(cam.yaw.to_degrees()),
            fmt_num(cam.pitch.to_degrees()),
            fmt_num(cam.fov),
            width.to_string(),
            height.to_string(),
        ])
        .status()
        .map_err(|e| format!("{cmd}: {e}"))?;
    if !status.success() {
        return Err(format!("{cmd} failed ({status})"));
    }
    fs::read(&out).map_err(|e| format!("{cmd} wrote no picture: {e}"))
}

/// What `show` prints.
pub fn describe(w: &World) -> String {
    let mut s = String::new();
    let d = &w.doc;
    s.push_str(&format!("world {} ({})\n", w.name, w.file.display()));
    match d.spawn() {
        Some(p) => s.push_str(&format!(
            "spawn {} {} {}\n",
            fmt_num(p[0]),
            fmt_num(p[1]),
            fmt_num(p[2])
        )),
        None => s.push_str("spawn: none (the server uses 0 0 0)\n"),
    }
    if let Some(sun) = d.sun() {
        s.push_str(&format!(
            "sun toward {} {} {}\n",
            fmt_num(sun.dir[0]),
            fmt_num(sun.dir[1]),
            fmt_num(sun.dir[2])
        ));
    }
    if let Some(sky) = d.sky() {
        s.push_str(&format!("sky {}\n", sky.textures.join(",")));
    }
    if let Some(t) = d.terrain() {
        match &w.terrain {
            Some(f) => s.push_str(&format!(
                "terrain {} ({} a square, {} materials, {} chunks missing)\n",
                t.folder,
                t.square,
                f.names.len(),
                f.missing
            )),
            None => s.push_str(&format!(
                "terrain {}: {}\n",
                t.folder,
                w.terrain_error.as_deref().unwrap_or("not loaded")
            )),
        }
    }
    for wa in d.waters() {
        s.push_str(&format!("water, surface at {}\n", fmt_num(wa.surface())));
    }
    let objects = d.objects();
    s.push_str(&format!("{} objects\n", objects.len()));
    for o in &objects {
        let found = if w.find(&o.shape).is_some() {
            ""
        } else {
            "  (shape not in the folder)"
        };
        s.push_str(&format!(
            "  {} {} at {} {} {}{}{}\n",
            o.name,
            o.shape,
            fmt_num(o.pos[0]),
            fmt_num(o.pos[1]),
            fmt_num(o.pos[2]),
            o.only_for
                .as_deref()
                .map(|a| format!(" for {a}"))
                .unwrap_or_default(),
            found
        ));
    }
    for p in d.problems() {
        s.push_str(&format!("problem: {p}\n"));
    }
    s
}
