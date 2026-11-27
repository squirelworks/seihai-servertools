//! From a read [`Model`] to the files of an [`Asset`]: the clients' axes
//! (+z up, one unit a tile, the front facing south), scale, origin, the
//! 5.12's limits, safe textures (or colours baked from them) and the
//! `.3ds`.

use super::write::{pieces, write_3ds, OutMaterial};
use super::{texture, Asset, Face, Model, Options, Origin, Up, Vertex};
use std::collections::HashMap;

/// Points a shape may have: a real 5.12 drew a shape of about 265 000
/// points and drew nothing by about 397 000 (T3D-124).
pub const MAX_POINTS: usize = 250_000;

fn to_z_up(p: [f32; 3], up: Up) -> [f32; 3] {
    match up {
        // +y up, front +z (glTF): up becomes +z, the front -y (south).
        Up::Y => [p[0], -p[2], p[1]],
        Up::Z => p,
    }
}

/// Make the asset `name` from `model`.
pub fn finish(mut model: Model, name: &str, o: &Options) -> Result<Asset, String> {
    let up = o.up.unwrap_or(model.up);
    let (s, c) = o.turn.to_radians().sin_cos();
    for v in &mut model.vertices {
        let p = to_z_up(v.p, up);
        v.p = [p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2]];
    }
    // Faces with a point that is not a number, or no area, are dropped.
    let before = model.faces.len();
    model.faces.retain(|f| {
        let p = f.v.map(|i| model.vertices[i as usize].p);
        if p.iter().flatten().any(|c| !c.is_finite()) {
            return false;
        }
        let a = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
        let b = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
        let n = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        n.iter().map(|x| x * x).sum::<f32>() > 0.0
    });
    if model.faces.len() < before {
        model.notes.push(format!(
            "{} faces with no area dropped",
            before - model.faces.len()
        ));
    }
    let (lo, hi) = model.bounds().ok_or("nothing to draw: no faces")?;
    let tall = hi[2] - lo[2];
    let scale = match o.height {
        Some(h) if tall > 0.0 => h / tall,
        Some(_) => return Err("--height: the asset is flat (no height to scale)".into()),
        None => o.scale,
    };
    if !(scale.is_finite() && scale > 0.0) {
        return Err("the scale must be above 0".into());
    }
    let shift = match o.origin {
        Origin::Ground => [-(lo[0] + hi[0]) / 2.0, -(lo[1] + hi[1]) / 2.0, -lo[2]],
        Origin::Centre => [
            -(lo[0] + hi[0]) / 2.0,
            -(lo[1] + hi[1]) / 2.0,
            -(lo[2] + hi[2]) / 2.0,
        ],
        Origin::Keep => [0.0; 3],
    };
    for v in &mut model.vertices {
        for (c, d) in v.p.iter_mut().zip(shift) {
            *c = (*c + d) * scale;
        }
    }
    let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]].map(|d| d * scale);
    if size.iter().any(|d| *d > 4000.0) {
        model.notes.push(format!(
            "it is {:.0} tiles across: check the scale (--scale or --height)",
            size.iter().cloned().fold(0.0, f32::max)
        ));
    }

    let (materials, vertices, faces, files, textures) = if o.colour_only {
        bake(&mut model, o.colour_bits)?
    } else {
        textured(&mut model, name, o.max_texture)?
    };
    if faces.is_empty() {
        return Err("nothing left to draw (every face was cut out)".into());
    }
    let points: usize = pieces(&vertices, &faces)
        .iter()
        .map(|p| p.vertices.len())
        .sum();
    if points > MAX_POINTS && !o.force {
        return Err(format!(
            "{points} points: a real 5.12 draws nothing of a shape past about {MAX_POINTS} \
             (T3D-124). Make it simpler first (Blender: Decimate), or --force"
        ));
    }
    if points > MAX_POINTS {
        model
            .notes
            .push(format!("{points} points: past what a 5.12 draws (--force)"));
    }
    let shape = format!("{name}.3ds");
    let mut out_files = vec![(shape.clone(), write_3ds(&materials, &vertices, &faces))];
    out_files.extend(files);
    Ok(Asset {
        name: name.to_string(),
        shape,
        files: out_files,
        size,
        points,
        faces: faces.len(),
        textures,
        notes: model.notes,
    })
}

type Prepared = (
    Vec<OutMaterial>,
    Vec<Vertex>,
    Vec<Face>,
    Vec<(String, Vec<u8>)>,
    Vec<String>,
);

/// Keep textures: each used one made safe and written `<name>_<n>.png`.
fn textured(model: &mut Model, name: &str, max: u32) -> Result<Prepared, String> {
    let mut used: Vec<usize> = model.faces.iter().map(|f| f.m).collect();
    used.sort();
    used.dedup();
    let mut mat_map: HashMap<usize, usize> = HashMap::new();
    let mut tex_map: HashMap<usize, String> = HashMap::new();
    let mut files = Vec::new();
    let mut textures = Vec::new();
    let mut materials = Vec::new();
    let mut shrunk = false;
    for &mi in &used {
        let m = &model.materials[mi];
        let tex = match m.texture {
            Some(t) => Some(match tex_map.get(&t) {
                Some(n) => n.clone(),
                None => {
                    let src = &model.textures[t];
                    let safe = texture::safe(src, max);
                    shrunk |= safe.width != src.width || safe.height != src.height;
                    let base = format!("{name}_{}", tex_map.len());
                    let file = format!("{base}.png");
                    files.push((file.clone(), texture::png(&safe)?));
                    textures.push(base);
                    tex_map.insert(t, file.clone());
                    file
                }
            }),
            None => None,
        };
        mat_map.insert(mi, materials.len());
        materials.push(OutMaterial {
            name: format!("m{}", materials.len()),
            colour: m.colour,
            texture: tex,
        });
    }
    if shrunk {
        model.notes.push(format!(
            "textures over {max} pixels a side were made smaller"
        ));
    }
    let faces = model
        .faces
        .iter()
        .map(|f| Face {
            v: f.v,
            m: mat_map[&f.m],
        })
        .collect();
    Ok((materials, model.vertices.clone(), faces, files, textures))
}

/// Faces a baked shape may grow to by cutting textured faces finer.
const BAKE_FACES: usize = 60_000;

/// Texture pixels along a baked face's longest side before it is cut.
const BAKE_PX: f32 = 8.0;

/// Bake textures into face colours. A textured face is cut into a grid of
/// smaller triangles (up to 16 a side, about one per 8 texture pixels,
/// within [`BAKE_FACES`] for the shape) so the picture survives as
/// colours; each piece takes the texture's colour at its middle (a face
/// left whole, the average of its middle and corners), kept to `bits` bits
/// a channel. Pieces on transparent texture are dropped.
fn bake(model: &mut Model, bits: u8) -> Result<Prepared, String> {
    // How finely to cut each face.
    let mut cuts: Vec<usize> = model
        .faces
        .iter()
        .map(|f| match model.materials[f.m].texture {
            None => 1,
            Some(t) => {
                let img = &model.textures[t];
                let side = img.width.max(img.height) as f32;
                let uv = f.v.map(|i| model.vertices[i as usize].uv);
                let e = (0..3)
                    .map(|k| {
                        let (a, b) = (uv[k], uv[(k + 1) % 3]);
                        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() * side
                    })
                    .fold(0.0, f32::max);
                ((e / BAKE_PX).ceil() as usize).clamp(1, 16)
            }
        })
        .collect();
    let total: usize = cuts.iter().map(|n| n * n).sum();
    if total > BAKE_FACES.max(model.faces.len()) {
        let k = (BAKE_FACES.max(model.faces.len()) as f32 / total as f32).sqrt();
        for n in &mut cuts {
            *n = ((*n as f32 * k).floor() as usize).max(1);
        }
    }
    let mut colours: Vec<[u8; 3]> = Vec::new();
    let mut by_colour: HashMap<[u8; 3], usize> = HashMap::new();
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut faces = Vec::new();
    let mut cut = 0usize;
    let mut colour_of = |rgb: [u8; 3]| {
        *by_colour.entry(rgb).or_insert_with(|| {
            colours.push(rgb);
            colours.len() - 1
        })
    };
    let lerp3 = |a: [f32; 3], b: [f32; 3], c: [f32; 3], s: f32, t: f32| {
        [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * s + (c[k] - a[k]) * t)
    };
    let lerp2 = |a: [f32; 2], b: [f32; 2], c: [f32; 2], s: f32, t: f32| {
        [0, 1].map(|k| a[k] + (b[k] - a[k]) * s + (c[k] - a[k]) * t)
    };
    for (f, &n) in model.faces.iter().zip(&cuts) {
        let m = &model.materials[f.m];
        let corner = f.v.map(|i| model.vertices[i as usize]);
        let Some(t) = m.texture else {
            let mi = colour_of(texture::quantise(m.colour, bits));
            let b = vertices.len() as u32;
            vertices.extend(corner);
            faces.push(Face {
                v: [b, b + 1, b + 2],
                m: mi,
            });
            continue;
        };
        let img = &model.textures[t];
        let uv = corner.map(|v| v.uv);
        let p = corner.map(|v| v.p);
        let mut tris: Vec<([[f32; 3]; 3], [u8; 3])> = Vec::new();
        if n == 1 {
            let mid = [
                (uv[0][0] + uv[1][0] + uv[2][0]) / 3.0,
                (uv[0][1] + uv[1][1] + uv[2][1]) / 3.0,
            ];
            let mut sum = [0u32; 3];
            let mut k = 0;
            for q in [mid, uv[0], uv[1], uv[2]] {
                // Corners pulled toward the middle, so a face reads its
                // own part of the picture.
                let q = [q[0] * 0.8 + mid[0] * 0.2, q[1] * 0.8 + mid[1] * 0.2];
                if let Some(c) = texture::sample(img, q) {
                    for j in 0..3 {
                        sum[j] += c[j] as u32;
                    }
                    k += 1;
                }
            }
            if k > 0 {
                tris.push((p, texture::quantise(sum.map(|s| (s / k) as u8), bits)));
            }
        } else {
            let step = 1.0 / n as f32;
            let mut piece = |a: (usize, usize), b: (usize, usize), c: (usize, usize)| {
                let at = |q: (usize, usize)| (q.0 as f32 * step, q.1 as f32 * step);
                let (s, t) = {
                    let (a, b, c) = (at(a), at(b), at(c));
                    ((a.0 + b.0 + c.0) / 3.0, (a.1 + b.1 + c.1) / 3.0)
                };
                let pos = [a, b, c].map(|q| {
                    let (s, t) = at(q);
                    lerp3(p[0], p[1], p[2], s, t)
                });
                match texture::sample(img, lerp2(uv[0], uv[1], uv[2], s, t)) {
                    Some(c) => tris.push((pos, texture::quantise(c, bits))),
                    None => cut += 1,
                }
            };
            for i in 0..n {
                for j in 0..n - i {
                    piece((i, j), (i + 1, j), (i, j + 1));
                    if i + j + 2 <= n {
                        piece((i + 1, j), (i + 1, j + 1), (i, j + 1));
                    }
                }
            }
        }
        if tris.is_empty() && n == 1 {
            cut += 1;
        }
        for (pos, rgb) in tris {
            let mi = colour_of(rgb);
            let b = vertices.len() as u32;
            for q in pos {
                vertices.push(Vertex { p: q, uv: [0.0; 2] });
            }
            faces.push(Face {
                v: [b, b + 1, b + 2],
                m: mi,
            });
        }
    }
    // Points shared by faces of one colour are written once.
    let (vertices, faces) = weld(vertices, faces);
    if cut > 0 {
        model
            .notes
            .push(format!("{cut} pieces on transparent texture dropped"));
    }
    if !model.textures.is_empty() {
        model.notes.push(format!(
            "textures baked into {} face colours ({bits} bits a channel)",
            colours.len()
        ));
    }
    let materials = colours
        .iter()
        .enumerate()
        .map(|(i, c)| OutMaterial {
            name: format!("m{i}"),
            colour: *c,
            texture: None,
        })
        .collect();
    Ok((materials, vertices, faces, Vec::new(), Vec::new()))
}

/// Merge points at the same place (and texture place).
fn weld(vertices: Vec<Vertex>, faces: Vec<Face>) -> (Vec<Vertex>, Vec<Face>) {
    let mut map: HashMap<[u32; 5], u32> = HashMap::new();
    let mut out = Vec::new();
    let mut remap = Vec::with_capacity(vertices.len());
    for v in &vertices {
        let key = [
            v.p[0].to_bits(),
            v.p[1].to_bits(),
            v.p[2].to_bits(),
            v.uv[0].to_bits(),
            v.uv[1].to_bits(),
        ];
        let i = *map.entry(key).or_insert_with(|| {
            out.push(*v);
            (out.len() - 1) as u32
        });
        remap.push(i);
    }
    let faces = faces
        .into_iter()
        .map(|f| Face {
            v: f.v.map(|i| remap[i as usize]),
            m: f.m,
        })
        .collect();
    (out, faces)
}

#[cfg(test)]
mod tests {
    use super::super::{card, Material};
    use super::*;
    use crate::preview::Rgba;

    fn tri_model(up: Up) -> Model {
        let mut m = Model::new(up);
        let mi = m.colour_material([10, 20, 30]);
        for p in [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 4.0, 1.0]] {
            m.vertices.push(Vertex { p, uv: [0.0, 0.0] });
        }
        m.faces.push(Face {
            v: [0, 1, 2],
            m: mi,
        });
        m
    }

    #[test]
    fn y_up_models_stand_up_on_the_ground() {
        let a = finish(tri_model(Up::Y), "t", &Options::default()).unwrap();
        // y (4 tall) became z; z (1 deep) became -y.
        assert_eq!(a.size, [2.0, 1.0, 4.0]);
        let tris = crate::world3d::shape::read_3ds(&a.files[0].1).unwrap();
        let zmin = tris
            .iter()
            .flat_map(|t| t.p)
            .map(|p| p[2])
            .fold(f32::MAX, f32::min);
        assert_eq!(zmin, 0.0);
        let o = Options {
            height: Some(2.0),
            ..Options::default()
        };
        let a = finish(tri_model(Up::Y), "t", &o).unwrap();
        assert_eq!(a.size, [1.0, 0.5, 2.0]);
        assert_eq!(a.shape, "t.3ds");
        assert!(a.textures.is_empty());
    }

    #[test]
    fn textures_are_named_for_the_asset_and_kept_small() {
        let img = Rgba {
            width: 3000,
            height: 1500,
            pixels: [50u8, 60, 70, 255].repeat(3000 * 1500),
        };
        let a = finish(card::card(img, 16.0), "sign", &Options::default()).unwrap();
        assert_eq!(a.textures, vec!["sign_0".to_string()]);
        let (n, png) = &a.files[1];
        assert_eq!(n, "sign_0.png");
        let back = texture::decode(png).unwrap();
        assert_eq!((back.width, back.height), (1024, 512));
        // The shape names the texture file.
        let shape = &a.files[0].1;
        assert!(shape.windows(10).any(|w| w == b"sign_0.png"));
    }

    #[test]
    fn colour_only_bakes_and_cuts() {
        let mut img = Rgba {
            width: 2,
            height: 1,
            pixels: vec![255, 0, 0, 255, 0, 0, 0, 0],
        };
        img.pixels[4..8].copy_from_slice(&[0, 0, 0, 0]);
        let mut m = Model::new(Up::Z);
        m.textures.push(img);
        m.materials.push(Material {
            name: "x".into(),
            colour: [1, 1, 1],
            texture: Some(0),
        });
        let quad = |m: &mut Model, u: f32| {
            let b = m.vertices.len() as u32;
            for (x, z) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)] {
                m.vertices.push(Vertex {
                    p: [x + u * 4.0, 0.0, z],
                    uv: [u + x * 0.4, 0.5],
                });
            }
            m.faces.push(Face {
                v: [b, b + 1, b + 2],
                m: 0,
            });
        };
        quad(&mut m, 0.05); // the red half
        quad(&mut m, 0.55); // the clear half
        let o = Options {
            colour_only: true,
            colour_bits: 8,
            ..Options::default()
        };
        let a = finish(m, "c", &o).unwrap();
        assert_eq!(a.faces, 1);
        assert_eq!(a.files.len(), 1);
        let tris = crate::world3d::shape::read_3ds(&a.files[0].1).unwrap();
        assert_eq!(tris[0].rgb, [255, 0, 0]);
    }

    #[test]
    fn colour_only_cuts_big_textured_faces_to_keep_the_picture() {
        // One quad whose 64 px texture is red on the left, blue on the right.
        let mut px = Vec::new();
        for _ in 0..64 {
            for x in 0..64 {
                px.extend(if x < 32 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 255, 255]
                });
            }
        }
        let img = Rgba {
            width: 64,
            height: 64,
            pixels: px,
        };
        let o = Options {
            colour_only: true,
            colour_bits: 8,
            ..Options::default()
        };
        let a = finish(card::card(img, 16.0), "q", &o).unwrap();
        assert!(a.faces > 2, "{}", a.faces);
        let tris = crate::world3d::shape::read_3ds(&a.files[0].1).unwrap();
        for t in &tris {
            let x = (t.p[0][0] + t.p[1][0] + t.p[2][0]) / 3.0;
            let want = if x < 0.0 { [255, 0, 0] } else { [0, 0, 255] };
            assert_eq!(t.rgb, want, "piece at x {x}");
        }
    }

    #[test]
    fn too_many_points_is_refused_unless_forced() {
        let mut m = Model::new(Up::Z);
        let mi = m.colour_material([1, 2, 3]);
        for i in 0..(MAX_POINTS as u32 / 3 + 10) {
            for p in [
                [i as f32, 0.0, 0.0],
                [i as f32 + 1.0, 0.0, 0.0],
                [i as f32, 0.0, 1.0],
            ] {
                m.vertices.push(Vertex { p, uv: [0.0, 0.0] });
            }
            m.faces.push(Face {
                v: [i * 3, i * 3 + 1, i * 3 + 2],
                m: mi,
            });
        }
        assert!(finish(m.clone(), "big", &Options::default()).is_err());
        let o = Options {
            force: true,
            ..Options::default()
        };
        assert!(finish(m, "big", &o).is_ok());
    }
}
