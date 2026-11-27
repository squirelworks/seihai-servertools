//! A 3D version of a [`generate::terrain`] world, for a Graal 3D server
//! (`graal3d = true` in seihai-gserver-v2 or seihai-gserver).
//!
//! The same height field as the 2D world (same options, same seed) becomes:
//!
//! * `<world>.3ds`: the ground, one mesh (samples 8 tiles apart, so one
//!   Torque unit per tile; one mesh per level when the world is too large for
//!   one), its faces coloured by height: water (flat) at and below the base,
//!   sand just above it, then grass and rock, as fractions of the highest point;
//! * `<world>_tree.3ds` and `<world>_rock.3ds`: two small coloured shapes;
//! * `<world>_graal3dworld.txt`: a world file (the format of
//!   seihai-gserver-v2's `bin/docs/graal3d.md`) with a sun, the ground at the
//!   origin, trees on grass and rocks on high ground, a spawn point on the
//!   flattest grass near the middle of the map, and the default camera.
//!
//! Every mesh has materials, since a 5.12 or 4.20r6 client does not draw a
//! `.3ds` without one (seihai-gserver-v2 lab, 2026-11-20). Faces are wound
//! counter-clockwise seen from outside, as in 3D Studio. The writer follows
//! the published 3DS chunk layout (main 0x4D4D, editor 0x3D3D, materials
//! 0xAFFF, objects 0x4000, triangle meshes 0x4100). Graal's own loader has not
//! been run on these files yet.
//!
//! With `stream`, the ground is also written as streamed terrain for a
//! 4.20r6 or 5.x client: `<world>_terrain/` holds the 256 chunk files,
//! `heights.info` and one 256 x 256 texture per material, and the world file
//! uses a `terrain` line in place of the mesh ground.
//!
//! Map tile (x, y) is Torque (x, -y): east stays +x and north +y, so the 2D
//! and 3D maps line up when seen from above.

use crate::generate::{self, DelphiRandom, File, TerrainOptions};
use crate::terrain_chunks;

/// Tiles between two height samples (the 2D generator's step).
const STEP: usize = 8;

/// A coloured material.
struct Material {
    name: &'static str,
    rgb: [u8; 3],
}

const WATER: Material = Material {
    name: "g3d_water",
    rgb: [40, 90, 200],
};
const SAND: Material = Material {
    name: "g3d_sand",
    rgb: [214, 196, 132],
};
const GRASS: Material = Material {
    name: "g3d_grass",
    rgb: [60, 150, 50],
};
const ROCK: Material = Material {
    name: "g3d_rock",
    rgb: [120, 110, 100],
};
const BARK: Material = Material {
    name: "g3d_bark",
    rgb: [100, 70, 40],
};
const LEAVES: Material = Material {
    name: "g3d_leaves",
    rgb: [30, 110, 40],
};

/// One triangle mesh: points, then faces with each face's material.
struct Mesh {
    name: String,
    points: Vec<[f32; 3]>,
    faces: Vec<([u16; 3], usize)>,
}

/// 3D options on top of the 2D world's.
#[derive(Debug, Clone)]
pub struct Terrain3dOptions {
    /// Torque units of height per height-field unit.
    pub vscale: f64,
    /// Trees to try to place per level (on grass only).
    pub trees_per_level: u32,
    /// Also write streamed terrain (see [`crate::terrain_chunks`]) and use it
    /// as the world's ground in place of the mesh.
    pub stream: bool,
}

impl Default for Terrain3dOptions {
    fn default() -> Self {
        Terrain3dOptions {
            vscale: 0.25,
            trees_per_level: 4,
            stream: false,
        }
    }
}

/// A height field ready to become a 3D world: `width` by `height` levels,
/// `heights` with `width * 8 + 1` columns and `height * 8 + 1` rows.
#[derive(Debug, Clone)]
pub struct Field3d {
    pub world: String,
    pub width: usize,
    pub height: usize,
    /// Heights below this are water.
    pub base: i32,
    /// The first split's reach; the bands use the highest point instead, and
    /// this only when the field is all water.
    pub amplitude: f64,
    /// Seeds the tree and rock placement.
    pub seed: u32,
    pub heights: Vec<Vec<i32>>,
    /// Where the field came from, for the world file's header.
    pub source: String,
}

/// The 3D files for the world `o` describes (see the module notes).
pub fn terrain3d(o: &TerrainOptions, o3: &Terrain3dOptions) -> Result<Vec<File>, String> {
    if o.width == 0 || o.height == 0 || o.width > 64 || o.height > 64 {
        return Err("terrain worlds are 1 to 64 levels a side".into());
    }
    let (pw, ph) = (o.width * STEP + 1, o.height * STEP + 1);
    let field = Field3d {
        world: o.world.clone(),
        width: o.width,
        height: o.height,
        base: o.base,
        amplitude: o.amplitude,
        seed: o.seed,
        heights: generate::height_field(
            pw,
            ph,
            o.seed,
            o.base,
            o.amplitude,
            o.chaos,
            o.even_borders,
        ),
        source: format!(
            "the gen terrain world {}.gmap (seed {}, base {}, height {}, chaos {}{})",
            o.world,
            o.seed,
            o.base,
            o.amplitude,
            o.chaos,
            if o.even_borders { ", islands" } else { "" }
        ),
    };
    world3d(&field, o3)
}

/// The height field of a gmap that carries a `HEIGHTMAP` block, `WIDTH` by
/// `HEIGHT` levels, in either form:
///
/// * `gen terrain`'s: `(WIDTH * 8 + 1)` by `(HEIGHT * 8 + 1)` heights, one per
///   tile corner, space-separated rows;
/// * the original TerrainGenerator's: `(WIDTH + 1) * (HEIGHT + 1)` heights,
///   one per level corner, comma-separated, followed by a `RANDOMSEEDS` block
///   (Graal_analysis `torque/terraingenerator-original-vs-servertools-2026-11-25.md`).
///   They are spread over the tile corners by bilinear interpolation, since
///   its levels carry no HEIGHTS of their own.
///
/// `GENBASE`,
/// `GENHEIGHT` and `GENSEED` set the water level, the bands and the
/// placement seed when present (else 0, 64 and 0).
pub fn field_from_gmap(world: &str, gmap: &str) -> Result<Field3d, String> {
    let mut width = 0usize;
    let mut height = 0usize;
    let (mut base, mut amplitude, mut seed) = (0i32, 64.0f64, 0u32);
    let mut rows: Vec<Vec<i32>> = Vec::new();
    let mut in_map = false;
    for line in gmap.lines() {
        let line = line.trim();
        if in_map {
            if line == "HEIGHTMAPEND" {
                in_map = false;
                continue;
            }
            // The next block (RANDOMSEEDS in the original's gmaps) also ends it.
            if line.starts_with(|c: char| c.is_ascii_alphabetic()) {
                in_map = false;
            } else {
                let row: Result<Vec<i32>, _> = line
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|w| !w.is_empty())
                    .map(|w| w.parse::<f64>().map(|v| v.round() as i32))
                    .collect();
                let row =
                    row.map_err(|_| format!("HEIGHTMAP row {} is not numbers", rows.len() + 1))?;
                if !row.is_empty() {
                    rows.push(row);
                }
                continue;
            }
        }
        let mut words = line.split_whitespace();
        let (key, value) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
        match key {
            "WIDTH" => width = value.parse().map_err(|_| "bad WIDTH")?,
            "HEIGHT" => height = value.parse().map_err(|_| "bad HEIGHT")?,
            "GENBASE" => base = value.parse().unwrap_or(0),
            "GENHEIGHT" => amplitude = value.parse().unwrap_or(64.0),
            "GENSEED" => seed = value.parse().unwrap_or(0),
            "HEIGHTMAP" => in_map = true,
            _ => {}
        }
    }
    if width == 0 || height == 0 || width > 64 || height > 64 {
        return Err("the gmap needs WIDTH and HEIGHT of 1 to 64 levels".into());
    }
    let (pw, ph) = (width * STEP + 1, height * STEP + 1);
    let all: Vec<i32> = rows.iter().flatten().copied().collect();
    if all.len() == (width + 1) * (height + 1) {
        // The original TerrainGenerator: one height per level corner.
        let corner = |x: usize, y: usize| f64::from(all[y * (width + 1) + x]);
        let heights = (0..ph)
            .map(|fy| {
                (0..pw)
                    .map(|fx| {
                        let (lx, ly) = ((fx / STEP).min(width - 1), (fy / STEP).min(height - 1));
                        let (u, v) = (
                            (fx - lx * STEP) as f64 / STEP as f64,
                            (fy - ly * STEP) as f64 / STEP as f64,
                        );
                        let top = corner(lx, ly) * (1.0 - u) + corner(lx + 1, ly) * u;
                        let bottom = corner(lx, ly + 1) * (1.0 - u) + corner(lx + 1, ly + 1) * u;
                        (top * (1.0 - v) + bottom * v).round() as i32
                    })
                    .collect()
            })
            .collect();
        return Ok(Field3d {
            world: world.to_string(),
            width,
            height,
            base,
            amplitude,
            seed,
            heights,
            source: format!(
                "the level-corner HEIGHTMAP of {world}.gmap (original TerrainGenerator form, interpolated; base {base})"
            ),
        });
    }
    if rows.len() < ph || rows.iter().take(ph).any(|r| r.len() < pw) {
        return Err(format!(
            "the gmap's HEIGHTMAP must hold {ph} rows of {pw} heights, or {} level-corner heights ({width} x {height} levels)",
            (width + 1) * (height + 1)
        ));
    }
    let heights = rows
        .into_iter()
        .take(ph)
        .map(|r| r[..pw].to_vec())
        .collect();
    Ok(Field3d {
        world: world.to_string(),
        width,
        height,
        base,
        amplitude,
        seed,
        heights,
        source: format!("the HEIGHTMAP of {world}.gmap (base {base}, height {amplitude})"),
    })
}

/// Which gserver a `--playerworld` folder is laid out for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Playerworld {
    /// seihai-gserver: `config/`, `levels/` and `files/`, served as it is
    /// with `seihai-gserver --world DIR` (lab, real 5.12, 2026-11-26: the
    /// GServer-v2 layout had to be copied into `files/` by hand).
    #[default]
    Seihai,
    /// seihai-gserver-v2 (GServer-v2): `config/` and `world/`.
    V2,
}

/// The files rearranged as a gserver folder for `server` (see
/// [`seihai_gserver_layout`] and [`playerworld_layout`]).
pub fn layout_for(server: Playerworld, world: &str, files: Vec<File>) -> Vec<File> {
    match server {
        Playerworld::Seihai => seihai_gserver_layout(world, files),
        Playerworld::V2 => playerworld_layout(world, files),
    }
}

/// The files rearranged as a seihai-gserver world folder that serves as it
/// is: the world file as `config/graal3dworld.txt`, the 2D levels and gmap in
/// `levels/`, every 3D file in `files/3d/` under its real, lower-cased name
/// (seihai-gserver serves everything under `files/`, at any depth, by bare
/// name), and a `config/server.txt` that turns 3D on, pushes the terrain's
/// files at login (else a 5.12's first visit draws the terrain white,
/// Graal_analysis `torque/terrain-512.md`) and starts new players in the
/// level under the 3D spawn.
pub fn seihai_gserver_layout(world: &str, files: Vec<File>) -> Vec<File> {
    let lower = world.to_lowercase();
    let world_txt = format!("{world}_graal3dworld.txt");
    let levels: Vec<String> = files
        .iter()
        .filter(|(n, _)| n.ends_with(".nw"))
        .map(|(n, _)| n.clone())
        .collect();
    let mut streamed = false;
    let mut spawn: Option<(f64, f64)> = None;
    let mut out: Vec<File> = files
        .into_iter()
        .map(|(name, bytes)| {
            if name == world_txt {
                let text = String::from_utf8_lossy(&bytes)
                    .replace(&format!(" {world}"), &format!(" {lower}"))
                    .replace(&format!("/{world}"), &format!("/{lower}"))
                    .replace("terrain world/", "terrain files/3d/")
                    .replace("into the server's world/ folder", "into the server's files/3d/ folder")
                    .replace("Copy it to config/graal3dworld.txt, and the .3ds files into the\n# server's file folders", "This is config/graal3dworld.txt of a seihai-gserver world; the shapes\n# and terrain are in files/3d/");
                streamed = text.lines().any(|l| l.starts_with("terrain "));
                spawn = text.lines().find_map(|l| {
                    let mut w = l.strip_prefix("spawn ")?.split_whitespace();
                    let x: f64 = w.next()?.parse().ok()?;
                    let y: f64 = w.next()?.parse().ok()?;
                    Some((x, -y))
                });
                ("config/graal3dworld.txt".to_string(), text.into_bytes())
            } else if name.ends_with(".nw") || name.ends_with(".gmap") {
                (format!("levels/{name}"), bytes)
            } else {
                let renamed = match name.strip_prefix(world) {
                    Some(rest) => format!("{lower}{rest}"),
                    None => name,
                };
                (format!("files/3d/{renamed}"), bytes)
            }
        })
        .collect();
    // The level under the spawn: gen terrain names them
    // <world><column><row>.nw, 64 tiles a side.
    let (tx, ty) = spawn.unwrap_or((32.0, 32.0));
    let (col, row) = ((tx / 64.0).max(0.0) as usize, (ty / 64.0).max(0.0) as usize);
    let want = format!("{world}{}{row:02}.nw", generate::column_name(col));
    let start = if levels.contains(&want) {
        Some((want, tx - (col * 64) as f64, ty - (row * 64) as f64))
    } else {
        levels.first().map(|l| (l.clone(), 30.0, 30.0))
    };
    let mut cfg = format!(
        "# seihai-gserver settings for the 3D world {world}, from seihai-servertools\n\
         # gen. Serve it with: seihai-gserver --world <this folder>\n\
         # A 5.x client links 3D only to a server whose name starts with `3 `.\n\
         # Other settings take their defaults (see seihai-gserver's world/config/server.txt).\n\n\
         name = 3 {world}\n\
         graal3d = true\n\
         # 4.x to 6.x clients log in to a lobby first; set the port they use.\n\
         #lobby_port = 14900\n"
    );
    if streamed {
        cfg.push_str(
            "# Send heights.info and the terrain textures at login, so a 5.12's\n\
             # first visit draws the terrain textured, not white.\n\
             graal3d_pushterrain = true\n",
        );
    }
    if let Some((level, x, y)) = start {
        cfg.push_str(&format!(
            "start_level = {level}\nstart_x = {:.1}\nstart_y = {:.1}\n",
            x.clamp(0.0, 63.0),
            y.clamp(0.0, 63.0)
        ));
    }
    out.push(("config/server.txt".to_string(), cfg.into_bytes()));
    out
}

/// The files rearranged as a gserver playerworld folder (seihai-gserver-v2's
/// `bin/docs/graal3d.md`, "Shipping a 3D world"): the world file as
/// `config/graal3dworld.txt`, 2D levels and the gmap in `world/`, and every 3D
/// file in `world/3d/` under its real, lower-cased name, which the world file
/// names (the 3D clients ask for lower-cased names).
pub fn playerworld_layout(world: &str, files: Vec<File>) -> Vec<File> {
    let lower = world.to_lowercase();
    let world_txt = format!("{world}_graal3dworld.txt");
    let mut out: Vec<File> = files
        .into_iter()
        .map(|(name, bytes)| {
            if name == world_txt {
                let text = String::from_utf8_lossy(&bytes)
                    .replace(&format!(" {world}"), &format!(" {lower}"))
                    .replace(&format!("/{world}"), &format!("/{lower}"))
                    .replace("terrain world/", "terrain world/3d/")
                    .replace("into the server's world/ folder", "into the server's world/3d/ folder")
                    .replace("Copy it to config/graal3dworld.txt, and the .3ds files into the\n# server's file folders", "This is config/graal3dworld.txt of a playerworld; the shapes and\n# terrain are in world/3d/, among the server's file folders");
                ("config/graal3dworld.txt".to_string(), text.into_bytes())
            } else if name.ends_with(".nw") || name.ends_with(".gmap") {
                (format!("world/{name}"), bytes)
            } else {
                let renamed = match name.strip_prefix(world) {
                    Some(rest) => format!("{lower}{rest}"),
                    None => name,
                };
                (format!("world/3d/{renamed}"), bytes)
            }
        })
        .collect();
    // A gserver file folder covers only its own files, so world/3d/ and each
    // folder under it (the streamed terrain) need a foldersconfig line (lab,
    // 2026-11-25: without them the world's files were not found).
    let mut folders: Vec<String> = out
        .iter()
        .filter_map(|(name, _)| name.strip_prefix("world/"))
        .filter(|rest| rest.starts_with("3d/"))
        .filter_map(|rest| rest.rsplit_once('/').map(|(dir, _)| dir.to_string()))
        .collect();
    folders.sort();
    folders.dedup();
    if !folders.is_empty() {
        let mut text = String::from(
            "# Add these lines to the server's config/foldersconfig.txt: a file\n\
             # folder covers only its own files, not the folders under it.\n",
        );
        for dir in folders {
            text.push_str(&format!("file    {dir}/*\n"));
        }
        out.push(("config/foldersconfig-3d.txt".to_string(), text.into_bytes()));
    }
    out
}

/// The 3D files for a height field (see the module notes).
pub fn world3d(f: &Field3d, o3: &Terrain3dOptions) -> Result<Vec<File>, String> {
    if !o3.vscale.is_finite() || o3.vscale <= 0.0 {
        return Err("the vertical scale must be a positive number".into());
    }
    let o = f;
    let (pw, ph) = (o.width * STEP + 1, o.height * STEP + 1);
    let field = &f.heights;
    let base = f64::from(o.base);
    // The bands follow the land as it came out, not the requested reach: sand
    // up to 8% of the highest point above the base, rock from 60% of it. (With
    // the reach, a world whose peaks overshoot it came out all sand on the
    // real 5.12, lab 2026-11-25.)
    let top = field
        .iter()
        .flatten()
        .map(|&h| f64::from(h))
        .fold(base, f64::max);
    let range = if top > base {
        top - base
    } else {
        o.amplitude.abs().max(1.0)
    };
    let sand_top = base + range * 0.08;
    let rock_from = base + range * 0.6;
    let z = |h: f64| ((h.max(base) - base) * o3.vscale) as f32;
    let height_at = |x: usize, y: usize| f64::from(field[y][x]);
    let corners_at = |x: usize, y: usize| {
        [
            height_at(x, y),
            height_at(x + 1, y),
            height_at(x, y + 1),
            height_at(x + 1, y + 1),
        ]
    };

    let materials = [&WATER, &SAND, &GRASS, &ROCK];
    // A cell is water only when all four corners are at or below the base, so
    // water is always flat; a cell rising out of it is sand.
    let pick = |corners: [f64; 4]| {
        let avg = corners.iter().sum::<f64>() / 4.0;
        if corners.iter().all(|&h| h <= base) {
            0
        } else if avg < sand_top {
            1
        } else if avg < rock_from {
            2
        } else {
            3
        }
    };

    // One mesh for the whole ground when its points and faces fit the 3DS
    // 16-bit counts: each mesh gets its own normals, so per-level meshes show
    // a shading seam where they meet (real 5.12, lab 2026-11-25). Larger
    // worlds fall back to one mesh per level.
    let whole =
        pw * ph <= usize::from(u16::MAX) && 2 * (pw - 1) * (ph - 1) <= usize::from(u16::MAX);
    let blocks: Vec<(usize, usize, usize, usize)> = if whole {
        vec![(0, 0, pw - 1, ph - 1)]
    } else {
        (0..o.height)
            .flat_map(|ly| (0..o.width).map(move |lx| (lx * STEP, ly * STEP, STEP, STEP)))
            .collect()
    };
    let mut meshes = Vec::new();
    for (n, &(x0, y0, w, h)) in blocks.iter().enumerate() {
        let mut mesh = Mesh {
            name: format!("g{n}"),
            points: Vec::new(),
            faces: Vec::new(),
        };
        for j in 0..=h {
            for i in 0..=w {
                let (fx, fy) = (x0 + i, y0 + j);
                mesh.points.push([
                    (fx * STEP) as f32,
                    -((fy * STEP) as f32),
                    z(height_at(fx, fy)),
                ]);
            }
        }
        let at = |i: usize, j: usize| (j * (w + 1) + i) as u16;
        for j in 0..h {
            for i in 0..w {
                let m = pick(corners_at(x0 + i, y0 + j));
                // Counter-clockwise from above (+z), with map y running to -y.
                mesh.faces.push(([at(i, j), at(i, j + 1), at(i + 1, j)], m));
                mesh.faces
                    .push(([at(i + 1, j), at(i, j + 1), at(i + 1, j + 1)], m));
            }
        }
        meshes.push(mesh);
    }

    let world = &o.world;
    let mut files = vec![(format!("{world}.3ds"), write_3ds(&materials, &meshes))];
    files.push((
        format!("{world}_tree.3ds"),
        write_3ds(&[&BARK, &LEAVES], &[tree()]),
    ));
    files.push((format!("{world}_rock.3ds"), write_3ds(&[&ROCK], &[rock()])));

    // The world file.
    let ground_z = |tx: f64, ty: f64| -> f64 {
        // Bilinear between the four samples around tile (tx, ty).
        let (sx, sy) = (tx / STEP as f64, ty / STEP as f64);
        let (x0, y0) = (
            (sx.floor() as usize).min(pw - 2),
            (sy.floor() as usize).min(ph - 2),
        );
        let (fx, fy) = (sx - x0 as f64, sy - y0 as f64);
        let h = |x: usize, y: usize| height_at(x, y).max(base);
        let top = h(x0, y0) * (1.0 - fx) + h(x0 + 1, y0) * fx;
        let bottom = h(x0, y0 + 1) * (1.0 - fx) + h(x0 + 1, y0 + 1) * fx;
        ((top * (1.0 - fy) + bottom * fy) - base) * o3.vscale
    };
    let surface = |tx: f64, ty: f64| -> usize {
        let (x, y) = (
            ((tx / STEP as f64) as usize).min(pw - 2),
            ((ty / STEP as f64) as usize).min(ph - 2),
        );
        pick(corners_at(x, y))
    };
    let n = |v: f64| format!("{:.2}", v);

    let mut text = String::new();
    text.push_str(&format!(
        "# {world}: the 3D version of {}, from seihai-servertools.\n\
         # Copy it to config/graal3dworld.txt, and the .3ds files into the\n\
         # server's file folders (the clients download them over TCP).\n\
         # One Torque unit is one tile; map tile (x, y) is (x, -y) here.\n\n",
        o.source
    ));
    text.push_str("sun 0.57 0.57 -0.57  0.85 0.85 0.8 1  0.35 0.35 0.4 1\n");
    if o3.stream {
        // The terrain is 256 squares a side, centred on the origin; the world
        // sits in its +x, -y quarter, so 128 squares cover its longer side.
        let side = 64 * o.width.max(o.height);
        let square = side.div_ceil(128).max(1);
        let half = (128 * square) as f64;
        let (w_tiles, h_tiles) = ((o.width * 64) as f64, (o.height * 64) as f64);
        let mut heights = vec![vec![0u16; terrain_chunks::SIZE]; terrain_chunks::SIZE];
        let mut mats = vec![vec![0u8; terrain_chunks::SIZE]; terrain_chunks::SIZE];
        for j in 0..terrain_chunks::SIZE {
            for i in 0..terrain_chunks::SIZE {
                let (x, y) = (-half + (i * square) as f64, -half + (j * square) as f64);
                let (tx, ty) = (x, -y);
                if (0.0..=w_tiles).contains(&tx) && (0.0..=h_tiles).contains(&ty) {
                    // Wire heights are 1/8 unit (Graal_analysis torque/terrain-512.md).
                    heights[j][i] = (ground_z(tx, ty) * 8.0).round().clamp(0.0, 16383.0) as u16;
                    mats[j][i] = surface(tx.min(w_tiles - 1.0), ty.min(h_tiles - 1.0)) as u8;
                }
            }
        }
        let folder = format!("{world}_terrain");
        for (name, bytes) in terrain_chunks::chunk_files(&heights, &mats)? {
            files.push((format!("{folder}/{name}"), bytes));
        }
        // One texture name a line (live, 2026-11-25), with its extension: the
        // Linux 4.206 client given bare names drew the terrain white, then saved
        // the textures extension-less and drew none (lab, 2026-11-25).
        let names: Vec<String> = materials
            .iter()
            .map(|m| format!("{}.png", m.name))
            .collect();
        files.push((
            format!("{folder}/heights.info"),
            (names.join("\n") + "\n").into_bytes(),
        ));
        for m in &materials {
            files.push((format!("{folder}/{}.png", m.name), texture(m.rgb)?));
        }
        text.push_str(&format!(
            "# Streamed terrain: copy {folder}/ into the server's world/ folder.\n\
             # The mesh ground is still written: swap the two lines to compare.\n\
             terrain world/{folder} {square}\n\
             # object Ground {world}.3ds 0 0 0\n\
             # Water over everything at the base: the surface is z + depth (live 5.12),\n\
             # so a 10 deep block at -9.75 has its surface at 0.25, just over the base (0):\n\
             # water {w0} {w0} -9.75 {ws} {ws} 10 g3d_water 0\n",
            // The client takes a water size only from 1024 a side.
            w0 = -(2.0 * half).max(2048.0) / 2.0,
            ws = (2.0 * half).max(2048.0)
        ));
    } else {
        text.push_str(&format!("object Ground {world}.3ds 0 0 0\n"));
    }

    let mut rng = DelphiRandom(o.seed ^ 0x3D3D_3D3D);
    let (mut trees, mut rocks) = (0, 0);
    for ly in 0..o.height {
        for lx in 0..o.width {
            for _ in 0..o3.trees_per_level {
                let tx = (lx * 64) as f64 + f64::from(rng.below(60)) + 2.0;
                let ty = (ly * 64) as f64 + f64::from(rng.below(60)) + 2.0;
                let turn = f64::from(rng.below(628)) / 100.0;
                match surface(tx, ty) {
                    2 => {
                        trees += 1;
                        text.push_str(&format!(
                            "object Tree{trees} {world}_tree.3ds {} {} {}  0 0 1 {}\n",
                            n(tx),
                            n(-ty),
                            n(ground_z(tx, ty)),
                            n(turn)
                        ));
                    }
                    3 => {
                        rocks += 1;
                        text.push_str(&format!(
                            "object Rock{rocks} {world}_rock.3ds {} {} {}  0 0 1 {}\n",
                            n(tx),
                            n(-ty),
                            n(ground_z(tx, ty)),
                            n(turn)
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
    // The spawn: the flattest grass (else any land) near the middle of the map,
    // so the default camera looks at ground, not water or a hillside.
    let (mut cx, mut cy) = ((o.width * 32) as f64, (o.height * 32) as f64);
    let mut best: Option<(f64, f64, f64)> = None;
    for want_grass in [true, false] {
        for y in 0..ph - 1 {
            for x in 0..pw - 1 {
                let c = corners_at(x, y);
                let m = pick(c);
                if m == 0 || (want_grass && m != 2) {
                    continue;
                }
                let (tx, ty) = (
                    ((x * STEP) + STEP / 2) as f64,
                    ((y * STEP) + STEP / 2) as f64,
                );
                let slope = c.iter().cloned().fold(f64::MIN, f64::max)
                    - c.iter().cloned().fold(f64::MAX, f64::min);
                let score = (tx - cx).hypot(ty - cy) + slope * 64.0 / range;
                if best.is_none_or(|b| score < b.0) {
                    best = Some((score, tx, ty));
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    if let Some((_, tx, ty)) = best {
        (cx, cy) = (tx, ty);
    }
    text.push_str(&format!(
        "\nspawn {} {} {}\n",
        n(cx),
        n(-cy),
        n(ground_z(cx, cy) + 1.0)
    ));
    text.push_str(
        "# The default camera sits 0 -5 2 from the spawn point and faces it.\n\
         # sky sky_fr,sky_rt,sky_bk,sky_lf,sky_up,sky_dn 500 0.6 0.7 0.9\n",
    );
    files.push((format!("{world}_graal3dworld.txt"), text.into_bytes()));
    Ok(files)
}

/// A 256 x 256 24-bit PNG of one colour with a little grain, as the client
/// wants for terrain materials.
pub fn texture_png(rgb: [u8; 3]) -> Result<Vec<u8>, String> {
    texture(rgb)
}

fn texture(rgb: [u8; 3]) -> Result<Vec<u8>, String> {
    let mut rng =
        DelphiRandom(u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2]));
    let mut pixels = Vec::with_capacity(256 * 256 * 3);
    for _ in 0..256 * 256 {
        let grain = rng.below(24) as i32 - 12;
        for c in rgb {
            pixels.push((i32::from(c) + grain).clamp(0, 255) as u8);
        }
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 256, 256);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&pixels).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// A tree 6 units tall: a square trunk and a four-sided crown.
fn tree() -> Mesh {
    let mut m = Mesh {
        name: "tree".into(),
        points: Vec::new(),
        faces: Vec::new(),
    };
    add_prism(&mut m, 0.3, 0.0, 2.0, 0);
    // Crown: a pyramid on a square base, 2.4 wide, from 1.6 to 6.
    let b = m.points.len() as u16;
    let r = 1.2;
    m.points.extend_from_slice(&[
        [-r, -r, 1.6],
        [r, -r, 1.6],
        [r, r, 1.6],
        [-r, r, 1.6],
        [0.0, 0.0, 6.0],
    ]);
    for k in 0..4u16 {
        m.faces.push(([b + k, b + (k + 1) % 4, b + 4], 1));
    }
    m.faces.push(([b, b + 3, b + 2], 1));
    m.faces.push(([b, b + 2, b + 1], 1));
    m
}

/// A rock: a squashed, uneven octahedron about 1.5 units across.
fn rock() -> Mesh {
    let mut m = Mesh {
        name: "rock".into(),
        points: vec![
            [0.9, 0.1, 0.4],
            [-0.1, 0.8, 0.35],
            [-0.8, -0.1, 0.45],
            [0.05, -0.7, 0.3],
            [0.1, 0.05, 1.1],
            [0.0, 0.0, -0.1],
        ],
        faces: Vec::new(),
    };
    for k in 0..4u16 {
        m.faces.push(([k, (k + 1) % 4, 4], 0));
        m.faces.push(([(k + 1) % 4, k, 5], 0));
    }
    m
}

/// A closed square prism (half width `r`) from `z0` to `z1`.
fn add_prism(m: &mut Mesh, r: f32, z0: f32, z1: f32, material: usize) {
    let b = m.points.len() as u16;
    for z in [z0, z1] {
        m.points
            .extend_from_slice(&[[-r, -r, z], [r, -r, z], [r, r, z], [-r, r, z]]);
    }
    for k in 0..4u16 {
        let (a, c) = (k, (k + 1) % 4);
        m.faces.push(([b + a, b + c, b + 4 + c], material));
        m.faces.push(([b + a, b + 4 + c, b + 4 + a], material));
    }
    m.faces.push(([b + 4, b + 5, b + 6], material));
    m.faces.push(([b + 4, b + 6, b + 7], material));
    m.faces.push(([b, b + 2, b + 1], material));
    m.faces.push(([b, b + 3, b + 2], material));
}

// ------------------------------------------------------------- 3DS writer

/// A chunk: id, then its length (6-byte header included), then the body.
fn chunk(id: u16, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 6);
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&((body.len() + 6) as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}

fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

fn colour(id: u16, rgb: [u8; 3]) -> Vec<u8> {
    chunk(id, &chunk(0x0011, &rgb))
}

/// A whole `.3ds` file: the materials, then one object per mesh. Each
/// mesh's face materials index `materials`.
fn write_3ds(materials: &[&Material], meshes: &[Mesh]) -> Vec<u8> {
    let mut editor = chunk(0x3D3E, &3u32.to_le_bytes());
    for m in materials {
        let mut body = chunk(0xA000, &cstr(m.name));
        let dim = m.rgb.map(|c| c / 2);
        body.extend(colour(0xA010, dim));
        body.extend(colour(0xA020, m.rgb));
        body.extend(colour(0xA030, [25, 25, 25]));
        editor.extend(chunk(0xAFFF, &body));
    }
    for mesh in meshes {
        let mut points = (mesh.points.len() as u16).to_le_bytes().to_vec();
        for p in &mesh.points {
            for v in p {
                points.extend_from_slice(&v.to_le_bytes());
            }
        }
        let mut faces = (mesh.faces.len() as u16).to_le_bytes().to_vec();
        for (f, _) in &mesh.faces {
            for v in f {
                faces.extend_from_slice(&v.to_le_bytes());
            }
            faces.extend_from_slice(&7u16.to_le_bytes()); // edge flags
        }
        for (mi, m) in materials.iter().enumerate() {
            let used: Vec<u16> = mesh
                .faces
                .iter()
                .enumerate()
                .filter(|(_, (_, fm))| *fm == mi)
                .map(|(i, _)| i as u16)
                .collect();
            if used.is_empty() {
                continue;
            }
            let mut group = cstr(m.name);
            group.extend_from_slice(&(used.len() as u16).to_le_bytes());
            for i in used {
                group.extend_from_slice(&i.to_le_bytes());
            }
            faces.extend(chunk(0x4130, &group));
        }
        let mut matrix = Vec::new();
        for v in [
            1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
        ] {
            matrix.extend_from_slice(&v.to_le_bytes());
        }
        let mut tri = chunk(0x4110, &points);
        tri.extend(chunk(0x4120, &faces));
        tri.extend(chunk(0x4160, &matrix));
        let mut object = cstr(&mesh.name);
        object.extend(chunk(0x4100, &tri));
        editor.extend(chunk(0x4000, &object));
    }
    let mut main = chunk(0x0002, &3u32.to_le_bytes());
    main.extend(chunk(0x3D3D, &editor));
    chunk(0x4D4D, &main)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> TerrainOptions {
        TerrainOptions {
            world: "isle".into(),
            width: 2,
            height: 1,
            seed: 7,
            base: 0,
            amplitude: 64.0,
            chaos: 0.5,
            even_borders: true,
        }
    }

    fn u16_at(b: &[u8], i: usize) -> u16 {
        u16::from_le_bytes([b[i], b[i + 1]])
    }
    fn u32_at(b: &[u8], i: usize) -> u32 {
        u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
    }

    /// The chunks directly inside `body`: (id, body).
    fn children(body: &[u8]) -> Vec<(u16, &[u8])> {
        let mut out = Vec::new();
        let mut i = 0;
        while i + 6 <= body.len() {
            let (id, len) = (u16_at(body, i), u32_at(body, i + 2) as usize);
            assert!(len >= 6 && i + len <= body.len(), "chunk {id:#x} overruns");
            out.push((id, &body[i + 6..i + len]));
            i += len;
        }
        assert_eq!(i, body.len(), "chunks fill their parent");
        out
    }

    fn find<'a>(chunks: &[(u16, &'a [u8])], id: u16) -> Vec<&'a [u8]> {
        chunks
            .iter()
            .filter(|(c, _)| *c == id)
            .map(|(_, b)| *b)
            .collect()
    }

    /// Every object's points and faces, read back from the file.
    #[allow(clippy::type_complexity)]
    fn read_meshes(
        file: &[u8],
    ) -> (
        Vec<String>,
        Vec<(Vec<[f32; 3]>, Vec<[u16; 3]>, Vec<String>)>,
    ) {
        let top = children(file);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].0, 0x4D4D);
        let main = children(top[0].1);
        let editor = children(find(&main, 0x3D3D)[0]);
        let mats: Vec<String> = find(&editor, 0xAFFF)
            .iter()
            .map(|m| {
                let name = find(&children(m), 0xA000)[0];
                String::from_utf8(name[..name.len() - 1].to_vec()).unwrap()
            })
            .collect();
        let mut meshes = Vec::new();
        for obj in find(&editor, 0x4000) {
            let nul = obj.iter().position(|&b| b == 0).unwrap();
            let tri = children(find(&children(&obj[nul + 1..]), 0x4100)[0]);
            let pts = find(&tri, 0x4110)[0];
            let np = u16_at(pts, 0) as usize;
            assert_eq!(pts.len(), 2 + np * 12);
            let points: Vec<[f32; 3]> = (0..np)
                .map(|k| {
                    let f = |c: usize| {
                        f32::from_le_bytes(
                            pts[2 + k * 12 + c * 4..6 + k * 12 + c * 4]
                                .try_into()
                                .unwrap(),
                        )
                    };
                    [f(0), f(1), f(2)]
                })
                .collect();
            let fa = find(&tri, 0x4120)[0];
            let nf = u16_at(fa, 0) as usize;
            let faces: Vec<[u16; 3]> = (0..nf)
                .map(|k| {
                    [
                        u16_at(fa, 2 + k * 8),
                        u16_at(fa, 4 + k * 8),
                        u16_at(fa, 6 + k * 8),
                    ]
                })
                .collect();
            let mut grouped = 0;
            let mut used = Vec::new();
            for g in find(&children(&fa[2 + nf * 8..]), 0x4130) {
                let nul = g.iter().position(|&b| b == 0).unwrap();
                used.push(String::from_utf8(g[..nul].to_vec()).unwrap());
                grouped += u16_at(g, nul + 1) as usize;
            }
            assert_eq!(grouped, nf, "every face has one material");
            meshes.push((points, faces, used));
        }
        (mats, meshes)
    }

    fn normal_z(p: &[[f32; 3]], f: [u16; 3]) -> f32 {
        let (a, b, c) = (p[f[0] as usize], p[f[1] as usize], p[f[2] as usize]);
        let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]]);
        u[0] * v[1] - u[1] * v[0]
    }

    #[test]
    fn ground_is_one_mesh_facing_up_with_materials() {
        let files = terrain3d(&opts(), &Terrain3dOptions::default()).unwrap();
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "isle.3ds",
                "isle_tree.3ds",
                "isle_rock.3ds",
                "isle_graal3dworld.txt"
            ]
        );
        let (mats, meshes) = read_meshes(&files[0].1);
        assert_eq!(mats, ["g3d_water", "g3d_sand", "g3d_grass", "g3d_rock"]);
        // One mesh, so no shading seam between the two levels.
        assert_eq!(meshes.len(), 1);
        let (points, faces, used) = &meshes[0];
        assert_eq!(points.len(), 17 * 9);
        assert_eq!(faces.len(), 256);
        assert!(!used.is_empty());
        for f in faces {
            assert!(normal_z(points, *f) > 0.0, "ground faces point up");
        }
        // The last sample is 128 units east.
        assert_eq!(points[16][0], 128.0);
        // Islands: the rim sits at the base, height 0.
        assert_eq!(points[0][2], 0.0);
    }

    #[test]
    fn shapes_are_closed_and_coloured() {
        for mesh in [tree(), rock()] {
            let bytes = write_3ds(&[&BARK, &LEAVES], std::slice::from_ref(&mesh));
            let (_, meshes) = read_meshes(&bytes);
            let faces = &meshes[0].1;
            // Closed: every edge is used once in each direction.
            let mut edges = std::collections::HashMap::new();
            for f in faces {
                for k in 0..3 {
                    *edges.entry((f[k], f[(k + 1) % 3])).or_insert(0) += 1;
                }
            }
            for (&(a, b), &c) in &edges {
                assert_eq!(c, 1, "{} edge {a}-{b}", mesh.name);
                assert_eq!(edges.get(&(b, a)), Some(&1), "{} edge {b}-{a}", mesh.name);
            }
        }
    }

    #[test]
    fn world_file_places_shapes_on_the_ground() {
        let o3 = Terrain3dOptions {
            vscale: 0.5,
            trees_per_level: 12,
            stream: false,
        };
        let mut placed_any = false;
        for seed in 1..40 {
            let mut o = opts();
            o.even_borders = false;
            o.width = 3;
            o.height = 3;
            o.seed = seed;
            let files = terrain3d(&o, &o3).unwrap();
            let text = String::from_utf8(files[3].1.clone()).unwrap();
            assert!(text.contains("object Ground isle.3ds 0 0 0\n"));
            assert!(text.contains("\nsun "));
            let spawn = text.lines().find(|l| l.starts_with("spawn ")).unwrap();
            let v: Vec<f64> = spawn[6..].split(' ').map(|s| s.parse().unwrap()).collect();
            assert!(
                (0.0..=192.0).contains(&v[0]) && (-192.0..=0.0).contains(&v[1]),
                "{spawn}"
            );
            assert!(v[2] >= 1.0);
            // On land whenever the map has any: not in the water.
            if text.contains("object Tree") || text.contains("object Rock") {
                assert!(v[2] > 1.0, "{spawn}");
            }
            for l in text.lines().filter(|l| l.starts_with("object ")) {
                let w: Vec<&str> = l.split_whitespace().collect();
                assert!(w.len() == 6 || w.len() == 10, "{l}");
                let x: f64 = w[3].parse().unwrap();
                let y: f64 = w[4].parse().unwrap();
                let z: f64 = w[5].parse().unwrap();
                assert!(
                    (0.0..=192.0).contains(&x) && (-192.0..=0.0).contains(&y) && z >= 0.0,
                    "{l}"
                );
                placed_any |= w[1] != "Ground";
            }
            // Same options, same files.
            assert_eq!(files, terrain3d(&o, &o3).unwrap());
        }
        assert!(placed_any, "some seed has land with trees or rocks");
    }

    #[test]
    fn bad_options_are_refused() {
        let mut o = opts();
        o.width = 0;
        assert!(terrain3d(&o, &Terrain3dOptions::default()).is_err());
        let o3 = Terrain3dOptions {
            vscale: 0.0,
            stream: false,
            ..Terrain3dOptions::default()
        };
        assert!(terrain3d(&opts(), &o3).is_err());
    }

    #[test]
    fn stream_writes_chunks_that_decode_to_the_ground() {
        let o = opts();
        let o3 = Terrain3dOptions {
            stream: true,
            ..Terrain3dOptions::default()
        };
        let files = terrain3d(&o, &o3).unwrap();
        let get = |n: &str| {
            files
                .iter()
                .find(|(name, _)| name == n)
                .map(|(_, b)| b.clone())
        };
        let chunks = files
            .iter()
            .filter(|(n, _)| n.starts_with("isle_terrain/heights") && n.ends_with(".dat"))
            .count();
        assert_eq!(chunks, 256);
        assert_eq!(
            get("isle_terrain/heights.info").unwrap(),
            b"g3d_water.png\ng3d_sand.png\ng3d_grass.png\ng3d_rock.png\n".to_vec()
        );
        let png = get("isle_terrain/g3d_grass.png").unwrap();
        assert_eq!(&png[16..24], &[0, 0, 1, 0, 0, 0, 1, 0]); // 256 x 256
        assert_eq!(png[25], 2); // RGB, 24-bit
        let text = String::from_utf8(get("isle_graal3dworld.txt").unwrap()).unwrap();
        assert!(text.contains("\nterrain world/isle_terrain 1\n"), "{text}");
        assert!(text.contains("# object Ground isle.3ds 0 0 0"));
        assert!(
            text.contains("# water -1024 -1024 -9.75 2048 2048 10 g3d_water 0"),
            "{text}"
        );

        // The world (2 x 2 levels, 128 tiles) sits in the +x, -y quarter:
        // square (128 + tx, 128 - ty) is tile (tx, ty). Outside it, flat 0.
        let plain = |cx: usize, cy: usize| {
            let z = get(&format!("isle_terrain/heights{cx}-{cy}.dat")).unwrap();
            let mut out = Vec::new();
            std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(&z[..]), &mut out)
                .unwrap();
            terrain_chunks::decode(&out)
        };
        let far = plain(0, 15);
        assert!(far.heights.iter().flatten().all(|&h| h == 0));
        assert!(far.materials.iter().flatten().all(|&m| m == 0));
        let inside = plain(10, 4); // squares 160..176 across, 64..80 up: tiles 32..48, 48..64
        assert!(inside.heights.iter().flatten().any(|&h| h > 0));
    }

    #[test]
    fn a_gmap_heightmap_gives_the_same_world() {
        let o = opts();
        let (pw, ph) = (o.width * STEP + 1, o.height * STEP + 1);
        let field =
            generate::height_field(pw, ph, o.seed, o.base, o.amplitude, o.chaos, o.even_borders);
        let mut gmap = format!(
            "GRMAP001\nWIDTH {}\nHEIGHT {}\nGENSEED {}\nGENBASE {}\nGENHEIGHT {}\nHEIGHTMAP\n",
            o.width, o.height, o.seed, o.base, o.amplitude
        );
        for row in &field {
            gmap += &row
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            gmap += "\n";
        }
        gmap += "HEIGHTMAPEND\n";
        let f = field_from_gmap(&o.world, &gmap).unwrap();
        assert_eq!(f.heights, field);
        let o3 = Terrain3dOptions::default();
        let a = terrain3d(&o, &o3).unwrap();
        let b = world3d(&f, &o3).unwrap();
        assert_eq!(a.len(), b.len());
        for ((na, ba), (nb, bb)) in a.iter().zip(&b) {
            assert_eq!(na, nb);
            if !na.ends_with(".txt") {
                assert_eq!(ba, bb, "{na}");
            }
        }
        // Too few heights for the levels is an error, not a panic.
        assert!(
            field_from_gmap("x", "WIDTH 2\nHEIGHT 2\nHEIGHTMAP\n1 2 3\nHEIGHTMAPEND\n").is_err()
        );
        assert!(field_from_gmap("x", "HEIGHTMAP\nHEIGHTMAPEND\n").is_err());
    }

    #[test]
    fn a_playerworld_has_real_lower_case_names_where_the_world_file_says() {
        let o = opts();
        let o3 = Terrain3dOptions {
            stream: true,
            ..Terrain3dOptions::default()
        };
        let mut world = o.clone();
        world.world = "Isle".into();
        let files = playerworld_layout("Isle", terrain3d(&world, &o3).unwrap());
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        for n in [
            "config/graal3dworld.txt",
            "world/3d/isle.3ds",
            "world/3d/isle_tree.3ds",
            "world/3d/isle_terrain/heights0-0.dat",
            "world/3d/isle_terrain/heights.info",
            "world/3d/isle_terrain/g3d_grass.png",
        ] {
            assert!(names.contains(&n), "{n} in {names:?}");
        }
        assert!(names.iter().all(|n| !n.contains("Isle")), "{names:?}");
        let folders = files
            .iter()
            .find(|(n, _)| n == "config/foldersconfig-3d.txt")
            .map(|(_, b)| String::from_utf8(b.clone()).unwrap())
            .expect("a foldersconfig snippet");
        assert!(folders.contains("file    3d/*\n"), "{folders}");
        assert!(folders.contains("file    3d/isle_terrain/*\n"), "{folders}");
        let text = String::from_utf8(
            files
                .iter()
                .find(|(n, _)| n == "config/graal3dworld.txt")
                .unwrap()
                .1
                .clone(),
        )
        .unwrap();
        assert!(text.contains("\nterrain world/3d/isle_terrain "), "{text}");
        assert!(text.contains("# object Ground isle.3ds 0 0 0"), "{text}");
        // Every shape the world file names is a file in world/3d/.
        for line in text.lines().filter(|l| l.starts_with("object ")) {
            let shape = line.split_whitespace().nth(2).unwrap();
            assert!(
                names.contains(&format!("world/3d/{shape}").as_str()),
                "{shape}"
            );
        }
    }

    #[test]
    fn a_seihai_gserver_world_serves_as_it_is() {
        let o3 = Terrain3dOptions {
            stream: true,
            ..Terrain3dOptions::default()
        };
        let mut world = opts();
        world.world = "Isle".into();
        let mut files = generate::terrain(&world).unwrap();
        files.extend(terrain3d(&world, &o3).unwrap());
        let files = layout_for(Playerworld::Seihai, "Isle", files);
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        for n in [
            "config/graal3dworld.txt",
            "config/server.txt",
            "levels/Isle.gmap",
            "files/3d/isle.3ds",
            "files/3d/isle_tree.3ds",
            "files/3d/isle_terrain/heights0-0.dat",
            "files/3d/isle_terrain/heights.info",
            "files/3d/isle_terrain/g3d_grass.png",
        ] {
            assert!(names.contains(&n), "{n} in {names:?}");
        }
        assert!(names.iter().all(|n| !n.starts_with("world/")), "{names:?}");
        // One heights.info, so the bare name the clients ask for is the terrain's.
        assert_eq!(
            names.iter().filter(|n| n.ends_with("heights.info")).count(),
            1
        );
        let text = |want: &str| {
            String::from_utf8(files.iter().find(|(n, _)| n == want).unwrap().1.clone()).unwrap()
        };
        let world_txt = text("config/graal3dworld.txt");
        assert!(
            world_txt.contains("\nterrain files/3d/isle_terrain "),
            "{world_txt}"
        );
        for line in world_txt.lines().filter(|l| l.starts_with("object ")) {
            let shape = line.split_whitespace().nth(2).unwrap();
            assert!(
                names.contains(&format!("files/3d/{shape}").as_str()),
                "{shape}"
            );
        }
        let cfg = text("config/server.txt");
        assert!(cfg.contains("\nname = 3 Isle\n"), "{cfg}");
        assert!(cfg.contains("\ngraal3d = true\n"), "{cfg}");
        assert!(cfg.contains("\ngraal3d_pushterrain = true\n"), "{cfg}");
        let start = cfg
            .lines()
            .find_map(|l| l.strip_prefix("start_level = "))
            .expect("a start level");
        assert!(
            names.contains(&format!("levels/{start}").as_str()),
            "{start}"
        );
    }

    #[test]
    fn bands_follow_the_land_flat_water_and_a_spawn_on_grass() {
        // A cone rising from water at the west edge to 100 at the east, with a
        // reach (amplitude) far above what came out.
        let (pw, ph) = (17, 9);
        let heights: Vec<Vec<i32>> = (0..ph)
            .map(|_| (0..pw).map(|x: i32| (x - 4) * 100 / 12).collect())
            .collect();
        let f = Field3d {
            world: "cone".into(),
            width: 2,
            height: 1,
            base: 0,
            amplitude: 4000.0,
            seed: 1,
            heights,
            source: "a test cone".into(),
        };
        let o3 = Terrain3dOptions {
            stream: true,
            trees_per_level: 0,
            ..Terrain3dOptions::default()
        };
        let files = world3d(&f, &o3).unwrap();
        let (_, meshes) = read_meshes(&files[0].1);
        // Rock shows on the high east side even though the reach is 4000.
        assert!(
            meshes[0].2.contains(&"g3d_rock".to_string()),
            "{:?}",
            meshes[0].2
        );
        let text = String::from_utf8(
            files
                .iter()
                .find(|(n, _)| n.ends_with("graal3dworld.txt"))
                .unwrap()
                .1
                .clone(),
        )
        .unwrap();
        let spawn = text.lines().find(|l| l.starts_with("spawn ")).unwrap();
        let v: Vec<f64> = spawn[6..].split(' ').map(|s| s.parse().unwrap()).collect();
        // On dry land (above the water's height 0 plus the 1 unit lift).
        assert!(v[2] > 1.0, "{spawn}");
    }

    #[test]
    fn an_original_terraingenerator_gmap_is_read_by_level_corner() {
        // The original's form: comma-separated level-corner heights, then RANDOMSEEDS.
        let gmap = "GRMAP001\nWIDTH 2\nHEIGHT 1\nHEIGHTMAP\n0,16,32,\n8,24,40,\nHEIGHTMAPEND\nRANDOMSEEDS\n11,22,\nRANDOMSEEDSEND\n";
        let f = field_from_gmap("orig", gmap).unwrap();
        assert_eq!((f.heights.len(), f.heights[0].len()), (9, 17));
        assert_eq!(f.heights[0][0], 0);
        assert_eq!(f.heights[0][8], 16);
        assert_eq!(f.heights[0][4], 8);
        assert_eq!(f.heights[8][16], 40);
        assert_eq!(f.heights[4][8], 20);
        assert!(world3d(&f, &Terrain3dOptions::default()).is_ok());
        // Neither form: refused with both sizes named.
        let bad = field_from_gmap("bad", "WIDTH 2\nHEIGHT 1\nHEIGHTMAP\n1,2\nHEIGHTMAPEND\n")
            .unwrap_err();
        assert!(bad.contains("6 level-corner"), "{bad}");
    }
}
