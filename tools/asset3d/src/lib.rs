//! `seihai-asset3d`: 3D models and pictures as shapes the 5.12 draws, added
//! to a seihai-gserver world folder. The conversion lives in
//! `lat_core::asset3d`; this crate reads the command line and writes files.

use lat_core::asset3d::{self, manifest, Asset, Options, Origin, Up};

use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub const USAGE: &str = "\
seihai-asset3d add FOLDER INPUT... [--name NAME] [CONVERT OPTIONS] [--force]
    Convert each INPUT and add it to FOLDER, a seihai-gserver world folder:
    the shape and its textures go in FOLDER/files/ (where the server sends
    them from by name) and a line in FOLDER/config/assets3d.txt lists the
    asset with its size. Place it in a world with an object line naming the
    shape (`seihai-edit3d add FOLDER NAME SHAPE X Y Z --ground`, or the
    editor's Place). An asset of the same name is replaced; a file of
    another asset or of the world itself is not, unless --force.
seihai-asset3d convert INPUT OUT_DIR [--name NAME] [CONVERT OPTIONS] [--force]
    Convert INPUT into OUT_DIR only (the shape and textures, nothing else).
seihai-asset3d list FOLDER
seihai-asset3d remove FOLDER NAME
    The assets in a world folder; remove deletes one's files and line.
seihai-asset3d check FILE...
    Check a .3ds, .dts, .dif or picture against what a 5.12 draws.
seihai-asset3d formats
    What is read, what is written, and what is not supported and why.

Inputs: .glb .gltf .obj (with .mtl) .stl .3ds, pictures (.png .gif .mng
.jpg .bmp .tga) as upright cards, and .dts/.dif copied after the 5.12's
version checks. The asset is named after the file (lower-case letters and
digits) unless --name is given (one INPUT only).

CONVERT OPTIONS:
    --up y|z        the source's up axis (default: +y for glTF and OBJ, +z
                    for STL and 3DS). The front (+z of a +y-up model) faces
                    south (-y), as a 2D level's things face the player.
    --scale F       tiles a source unit (default 1: a 1.8 m glTF person is
                    1.8 tiles tall; a 2D player is about 3)
    --height F      scale it to F tiles tall instead
    --turn DEG      turn it about +z, counter-clockwise seen from above
    --origin ground|centre|keep
                    where its origin goes (default ground: the middle of its
                    footprint, standing on z = 0)
    --max-texture N largest texture side (default 1024, the most a real
                    5.12 drew; 2048 drew white)
    --colour-only   no textures: bake them into face colours (for 4.20r8,
                    which draws no textures); pictures become coloured strips
    --colour-bits N bits a channel for baked colours (1 to 8, default 4)
    --px-per-tile F pictures: pixels a tile (default 16, as in 2D)
    --solid         pictures: coloured strips instead of a textured card
    --force         allow a shape past the 5.12's 250 000 points, and
                    replace files that are not this asset's";

#[derive(Debug, Default)]
struct Common {
    opts: Options,
    name: Option<String>,
    force: bool,
    plain: Vec<String>,
}

fn parse(args: &[&str]) -> Result<Common, String> {
    let mut c = Common::default();
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        let mut value = || -> Result<&str, String> {
            i += 1;
            args.get(i).copied().ok_or(format!("{a} needs a value"))
        };
        let num = |v: &str| -> Result<f32, String> {
            v.parse::<f32>()
                .ok()
                .filter(|f| f.is_finite())
                .ok_or(format!("{a} {v}: a number"))
        };
        match a {
            "--name" => c.name = Some(value()?.to_string()),
            "--up" => {
                c.opts.up = Some(match value()? {
                    "y" | "Y" => Up::Y,
                    "z" | "Z" => Up::Z,
                    v => return Err(format!("--up {v}: y or z")),
                })
            }
            "--scale" => {
                let v = num(value()?)?;
                if v <= 0.0 {
                    return Err("--scale: above 0".into());
                }
                c.opts.scale = v;
            }
            "--height" => {
                let v = num(value()?)?;
                if v <= 0.0 {
                    return Err("--height: above 0".into());
                }
                c.opts.height = Some(v);
            }
            "--turn" => c.opts.turn = num(value()?)?,
            "--origin" => {
                c.opts.origin = match value()? {
                    "ground" => Origin::Ground,
                    "centre" | "center" => Origin::Centre,
                    "keep" => Origin::Keep,
                    v => return Err(format!("--origin {v}: ground, centre or keep")),
                }
            }
            "--max-texture" => {
                let v = value()?;
                c.opts.max_texture = v
                    .parse()
                    .ok()
                    .filter(|n| (1..=8192).contains(n))
                    .ok_or(format!("--max-texture {v}: 1 to 8192"))?;
            }
            "--colour-only" | "--color-only" => c.opts.colour_only = true,
            "--colour-bits" | "--color-bits" => {
                let v = value()?;
                c.opts.colour_bits = v
                    .parse()
                    .ok()
                    .filter(|b| (1..=8).contains(b))
                    .ok_or(format!("--colour-bits {v}: 1 to 8"))?;
            }
            "--px-per-tile" => {
                let v = num(value()?)?;
                if v <= 0.0 {
                    return Err("--px-per-tile: above 0".into());
                }
                c.opts.px_per_tile = v;
            }
            "--solid" => c.opts.solid = true,
            "--force" => {
                c.force = true;
                c.opts.force = true;
            }
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s => c.plain.push(s.to_string()),
        }
        i += 1;
    }
    Ok(c)
}

/// Write `bytes` to `path` through a temporary file and a rename.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

fn print_asset(a: &Asset) {
    let tex = if a.textures.is_empty() {
        "no textures".to_string()
    } else {
        format!("textures {}", a.textures.join(", "))
    };
    if a.points > 0 {
        println!(
            "{}: {} ({} x {} x {} tiles, {} points, {} faces, {tex})",
            a.name,
            a.shape,
            round(a.size[0]),
            round(a.size[1]),
            round(a.size[2]),
            a.points,
            a.faces
        );
    } else {
        println!("{}: {}", a.name, a.shape);
    }
    for n in &a.notes {
        println!("  note: {n}");
    }
}

fn round(v: f32) -> String {
    lat_core::world3d::doc::fmt_num(((v * 100.0).round() / 100.0) as f64)
}

fn source_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn convert_cmd(c: &Common) -> Result<(), String> {
    let [input, out] = &c.plain[..] else {
        return Err("convert: give INPUT and OUT_DIR".into());
    };
    let a = asset3d::convert(Path::new(input), c.name.as_deref(), &c.opts)?;
    let out = PathBuf::from(out);
    for (name, _) in &a.files {
        let p = out.join(name);
        if p.exists() && !c.force {
            return Err(format!(
                "{} exists (use --force to replace it)",
                p.display()
            ));
        }
    }
    for (name, bytes) in &a.files {
        write_file(&out.join(name), bytes)?;
    }
    print_asset(&a);
    Ok(())
}

fn read_manifest(folder: &Path) -> Result<manifest::Manifest, String> {
    let p = folder.join(manifest::PATH);
    match std::fs::read_to_string(&p) {
        Ok(t) => Ok(manifest::Manifest::parse(&t)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
        Err(e) => Err(format!("{}: {e}", p.display())),
    }
}

fn check_folder(folder: &Path) -> Result<(), String> {
    if !folder.join("config").is_dir() {
        return Err(format!(
            "{} has no config/ folder: give a seihai-gserver world folder",
            folder.display()
        ));
    }
    Ok(())
}

fn add_cmd(c: &Common) -> Result<(), String> {
    let [folder, inputs @ ..] = &c.plain[..] else {
        return Err("add: give FOLDER and INPUT".into());
    };
    if inputs.is_empty() {
        return Err("add: give at least one INPUT".into());
    }
    if c.name.is_some() && inputs.len() > 1 {
        return Err("--name names one INPUT only".into());
    }
    let folder = PathBuf::from(folder);
    check_folder(&folder)?;
    let mut man = read_manifest(&folder)?;
    let files = folder.join("files");
    // Convert everything first, so a failure leaves the folder untouched.
    let mut assets = Vec::new();
    for input in inputs {
        let p = Path::new(input);
        let a =
            asset3d::convert(p, c.name.as_deref(), &c.opts).map_err(|e| format!("{input}: {e}"))?;
        if assets
            .iter()
            .any(|(b, _): &(Asset, String)| b.name == a.name)
        {
            return Err(format!("two inputs make the asset {}; use --name", a.name));
        }
        assets.push((a, source_name(p)));
    }
    for (a, _) in &assets {
        let old: Vec<String> = man.get(&a.name).map(|e| e.files()).unwrap_or_default();
        for (name, _) in &a.files {
            let owned_by_other = man
                .entries()
                .iter()
                .any(|e| e.name != a.name && e.files().contains(name));
            let taken = owned_by_other || (files.join(name).exists() && !old.contains(name));
            if taken && !c.force {
                return Err(format!(
                    "files/{name} is already there and not the asset {}'s \
                     (use --name, or --force to replace it)",
                    a.name
                ));
            }
        }
    }
    for (a, source) in &assets {
        if let Some(old) = man.get(&a.name) {
            for f in old.files() {
                if !a.files.iter().any(|(n, _)| *n == f) {
                    let _ = std::fs::remove_file(files.join(&f));
                }
            }
        }
        for (name, bytes) in &a.files {
            write_file(&files.join(name), bytes)?;
        }
        man.set(&manifest::Entry {
            name: a.name.clone(),
            shape: a.shape.clone(),
            size: a.size,
            textures: a.textures.clone(),
            source: source.clone(),
        });
        print_asset(a);
    }
    write_file(&folder.join(manifest::PATH), man.text().as_bytes())?;
    println!(
        "Listed in {}. Place one with: seihai-edit3d add {} NAME SHAPE X Y Z --ground",
        manifest::PATH,
        folder.display()
    );
    Ok(())
}

fn list_cmd(c: &Common) -> Result<(), String> {
    let [folder] = &c.plain[..] else {
        return Err("list: give FOLDER".into());
    };
    let folder = PathBuf::from(folder);
    let man = read_manifest(&folder)?;
    let entries = man.entries();
    if entries.is_empty() {
        println!("No assets in {}.", folder.join(manifest::PATH).display());
    }
    for e in entries {
        let missing: Vec<String> = e
            .files()
            .into_iter()
            .filter(|f| !folder.join("files").join(f).exists())
            .collect();
        println!(
            "{} {} {} x {} x {} tiles, from {}{}",
            e.name,
            e.shape,
            round(e.size[0]),
            round(e.size[1]),
            round(e.size[2]),
            e.source,
            if missing.is_empty() {
                String::new()
            } else {
                format!(" (missing in files/: {})", missing.join(", "))
            }
        );
    }
    Ok(())
}

fn remove_cmd(c: &Common) -> Result<(), String> {
    let [folder, name] = &c.plain[..] else {
        return Err("remove: give FOLDER and NAME".into());
    };
    let folder = PathBuf::from(folder);
    let mut man = read_manifest(&folder)?;
    let e = man
        .remove(name)
        .ok_or(format!("no asset {name} in {}", manifest::PATH))?;
    for f in e.files() {
        let _ = std::fs::remove_file(folder.join("files").join(&f));
    }
    write_file(&folder.join(manifest::PATH), man.text().as_bytes())?;
    println!(
        "Removed {name}. World files that still place {} will log it as missing.",
        e.shape
    );
    Ok(())
}

fn check_cmd(c: &Common) -> Result<bool, String> {
    if c.plain.is_empty() {
        return Err("check: give FILE".into());
    }
    let mut ok = true;
    for f in &c.plain {
        match asset3d::check(Path::new(f)) {
            Ok(r) => {
                println!("{f}: {}", r.facts.join("; "));
                for p in &r.problems {
                    println!("  problem: {p}");
                }
                ok &= r.problems.is_empty();
            }
            Err(e) => {
                println!("{f}: {e}");
                ok = false;
            }
        }
    }
    Ok(ok)
}

/// The command line: `seihai-asset3d ...` or `seihai-servertools asset3d ...`.
pub fn run(args: &[&str]) -> ExitCode {
    let (cmd, rest) = match args {
        [] => {
            println!("{USAGE}");
            return ExitCode::from(2);
        }
        ["--help" | "-h" | "help", ..] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        [cmd, rest @ ..] => (*cmd, rest),
    };
    let c = match parse(rest) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("seihai-asset3d: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = match cmd {
        "add" => add_cmd(&c).map(|_| true),
        "convert" => convert_cmd(&c).map(|_| true),
        "list" => list_cmd(&c).map(|_| true),
        "remove" => remove_cmd(&c).map(|_| true),
        "check" => check_cmd(&c),
        "formats" => {
            println!("{}", asset3d::FORMATS);
            Ok(true)
        }
        other => {
            eprintln!("seihai-asset3d: unknown command {other}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("seihai-asset3d: {e}");
            ExitCode::FAILURE
        }
    }
}
