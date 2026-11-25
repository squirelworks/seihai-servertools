//! A 3D version of a [`generate::terrain`] world, for a Graal 3D server
//! (`graal3d = true` in seihai-gserver-v2 or seihai-gserver).
//!
//! The same height field as the 2D world (same options, same seed) becomes:
//!
//! * `<world>.3ds`: the ground, one mesh per level (9 by 9 samples, 8 tiles
//!   apart, so one Torque unit per tile), its faces coloured by height:
//!   water below the base, sand just above it, then grass and rock;
//! * `<world>_tree.3ds` and `<world>_rock.3ds`: two small coloured shapes;
//! * `<world>_graal3dworld.txt`: a world file (the format of
//!   seihai-gserver-v2's `bin/docs/graal3d.md`) with a sun, the ground at the
//!   origin, trees on grass and rocks on high ground, a spawn point on the
//!   ground at the middle of the map, and the default camera.
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

/// The 3D files for the world `o` describes (see the module notes).
pub fn terrain3d(o: &TerrainOptions, o3: &Terrain3dOptions) -> Result<Vec<File>, String> {
    if o.width == 0 || o.height == 0 || o.width > 64 || o.height > 64 {
        return Err("terrain worlds are 1 to 64 levels a side".into());
    }
    if !o3.vscale.is_finite() || o3.vscale <= 0.0 {
        return Err("the vertical scale must be a positive number".into());
    }
    let (pw, ph) = (o.width * STEP + 1, o.height * STEP + 1);
    let field =
        generate::height_field(pw, ph, o.seed, o.base, o.amplitude, o.chaos, o.even_borders);
    let base = f64::from(o.base);
    // Sand up to an eighth of the first split's reach above the base, rock
    // from half of it.
    let sand_top = base + o.amplitude.abs() / 8.0;
    let rock_from = base + o.amplitude.abs() / 2.0;
    let z = |h: f64| ((h.max(base) - base) * o3.vscale) as f32;
    let height_at = |x: usize, y: usize| f64::from(field[y][x]);

    let materials = [&WATER, &SAND, &GRASS, &ROCK];
    let pick = |avg: f64| {
        if avg < base {
            0
        } else if avg < sand_top {
            1
        } else if avg < rock_from {
            2
        } else {
            3
        }
    };

    let mut meshes = Vec::new();
    for ly in 0..o.height {
        for lx in 0..o.width {
            let mut mesh = Mesh {
                name: format!("g{}", ly * o.width + lx),
                points: Vec::new(),
                faces: Vec::new(),
            };
            for j in 0..=STEP {
                for i in 0..=STEP {
                    let (fx, fy) = (lx * STEP + i, ly * STEP + j);
                    mesh.points.push([
                        (fx * STEP) as f32,
                        -((fy * STEP) as f32),
                        z(height_at(fx, fy)),
                    ]);
                }
            }
            let at = |i: usize, j: usize| (j * (STEP + 1) + i) as u16;
            for j in 0..STEP {
                for i in 0..STEP {
                    let (fx, fy) = (lx * STEP + i, ly * STEP + j);
                    let avg = (height_at(fx, fy)
                        + height_at(fx + 1, fy)
                        + height_at(fx, fy + 1)
                        + height_at(fx + 1, fy + 1))
                        / 4.0;
                    let m = pick(avg);
                    // Counter-clockwise from above (+z), with map y running to -y.
                    mesh.faces.push(([at(i, j), at(i, j + 1), at(i + 1, j)], m));
                    mesh.faces
                        .push(([at(i + 1, j), at(i, j + 1), at(i + 1, j + 1)], m));
                }
            }
            meshes.push(mesh);
        }
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
        pick(
            (height_at(x, y) + height_at(x + 1, y) + height_at(x, y + 1) + height_at(x + 1, y + 1))
                / 4.0,
        )
    };
    let n = |v: f64| format!("{:.2}", v);

    let mut text = String::new();
    text.push_str(&format!(
        "# {world}: the 3D version of the gen terrain world {world}.gmap\n\
         # (seed {}, base {}, height {}, chaos {}{}), from seihai-servertools.\n\
         # Copy it to config/graal3dworld.txt, and the .3ds files into the\n\
         # server's file folders (the clients download them over TCP).\n\
         # One Torque unit is one tile; map tile (x, y) is (x, -y) here.\n\n",
        o.seed,
        o.base,
        o.amplitude,
        o.chaos,
        if o.even_borders { ", islands" } else { "" }
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
        let names: Vec<&str> = materials.iter().map(|m| m.name).collect();
        // One texture name a line (live, 2026-11-25); each is fetched bare.
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
             # object Ground {world}.3ds 0 0 0\n"
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
    let (cx, cy) = ((o.width * 32) as f64, (o.height * 32) as f64);
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
    fn ground_is_one_mesh_per_level_facing_up_with_materials() {
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
        assert_eq!(meshes.len(), 2);
        for (points, faces, used) in &meshes {
            assert_eq!(points.len(), 81);
            assert_eq!(faces.len(), 128);
            assert!(!used.is_empty());
            for f in faces {
                assert!(normal_z(points, *f) > 0.0, "ground faces point up");
            }
        }
        // The second level starts where the first ends: 64 units east.
        assert_eq!(meshes[1].0[0][0], 64.0);
        assert_eq!(meshes[0].0[8], meshes[1].0[0], "levels share their edge");
        // Islands: the rim sits at the base, height 0.
        assert_eq!(meshes[0].0[0][2], 0.0);
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
            assert_eq!((v[0], v[1]), (96.0, -96.0));
            assert!(v[2] >= 1.0);
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
            b"g3d_water\ng3d_sand\ng3d_grass\ng3d_rock\n".to_vec()
        );
        let png = get("isle_terrain/g3d_grass.png").unwrap();
        assert_eq!(&png[16..24], &[0, 0, 1, 0, 0, 0, 1, 0]); // 256 x 256
        assert_eq!(png[25], 2); // RGB, 24-bit
        let text = String::from_utf8(get("isle_graal3dworld.txt").unwrap()).unwrap();
        assert!(text.contains("\nterrain world/isle_terrain 1\n"), "{text}");
        assert!(text.contains("# object Ground isle.3ds 0 0 0"));

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
}
