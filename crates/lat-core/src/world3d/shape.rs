//! Reading `.3ds` shapes for the preview: the triangles of every mesh and
//! the colour of each face's material.
//!
//! The reader follows the published 3DS chunk layout, the same one
//! servertools' writers use (main 0x4D4D, editor 0x3D3D, materials 0xAFFF
//! with a name 0xA000 and a diffuse colour 0xA020, objects 0x4000, triangle
//! meshes 0x4100 with points 0x4110, faces 0x4120 and face groups 0x4130).
//! Points are read as stored: 3D Studio keeps them in world space, and the
//! mesh matrix (0x4160) is not applied. Textures are not read; a face whose
//! material has no colour is grey. `.dts` and `.dif` shapes are not read
//! here.

use std::collections::HashMap;

/// One triangle in shape space with its colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tri {
    pub p: [[f32; 3]; 3],
    pub rgb: [u8; 3],
}

const GREY: [u8; 3] = [170, 170, 170];

fn chunks(data: &[u8]) -> Vec<(u16, &[u8])> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 6 <= data.len() {
        let id = u16::from_le_bytes([data[at], data[at + 1]]);
        let len =
            u32::from_le_bytes([data[at + 2], data[at + 3], data[at + 4], data[at + 5]]) as usize;
        if len < 6 || at + len > data.len() {
            break;
        }
        out.push((id, &data[at + 6..at + len]));
        at += len;
    }
    out
}

/// A zero-ended string and what follows it.
fn cstr(data: &[u8]) -> (String, &[u8]) {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let name = String::from_utf8_lossy(&data[..end]).into_owned();
    (name, data.get(end + 1..).unwrap_or(&[]))
}

fn colour(body: &[u8]) -> Option<[u8; 3]> {
    for (id, c) in chunks(body) {
        match id {
            0x0011 | 0x0012 if c.len() >= 3 => return Some([c[0], c[1], c[2]]),
            0x0010 | 0x0013 if c.len() >= 12 => {
                let f = |i: usize| {
                    let v = f32::from_le_bytes([c[i], c[i + 1], c[i + 2], c[i + 3]]);
                    (v.clamp(0.0, 1.0) * 255.0).round() as u8
                };
                return Some([f(0), f(4), f(8)]);
            }
            _ => {}
        }
    }
    None
}

fn u16_at(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(i)?, *d.get(i + 1)?]))
}

fn f32_at(d: &[u8], i: usize) -> Option<f32> {
    Some(f32::from_le_bytes([
        *d.get(i)?,
        *d.get(i + 1)?,
        *d.get(i + 2)?,
        *d.get(i + 3)?,
    ]))
}

/// Every triangle of a `.3ds` file, or an error if it is not one.
pub fn read_3ds(data: &[u8]) -> Result<Vec<Tri>, String> {
    let top = chunks(data);
    let main = match top.first() {
        Some((0x4D4D, body)) => *body,
        _ => return Err("not a .3ds file (no 0x4D4D chunk)".into()),
    };
    let mut materials: HashMap<String, [u8; 3]> = HashMap::new();
    let mut tris = Vec::new();
    for (id, editor) in chunks(main) {
        if id != 0x3D3D {
            continue;
        }
        for (id, body) in chunks(editor) {
            if id == 0xAFFF {
                let mut name = String::new();
                let mut rgb = None;
                for (id, b) in chunks(body) {
                    match id {
                        0xA000 => name = cstr(b).0,
                        0xA020 => rgb = colour(b),
                        _ => {}
                    }
                }
                materials.insert(name, rgb.unwrap_or(GREY));
            }
        }
        for (id, body) in chunks(editor) {
            if id != 0x4000 {
                continue;
            }
            let (_, rest) = cstr(body);
            for (id, mesh) in chunks(rest) {
                if id == 0x4100 {
                    read_mesh(mesh, &materials, &mut tris);
                }
            }
        }
    }
    Ok(tris)
}

fn read_mesh(mesh: &[u8], materials: &HashMap<String, [u8; 3]>, tris: &mut Vec<Tri>) {
    let mut points: Vec<[f32; 3]> = Vec::new();
    let mut faces: Vec<[u16; 3]> = Vec::new();
    let mut colours: Vec<[u8; 3]> = Vec::new();
    for (id, b) in chunks(mesh) {
        match id {
            0x4110 => {
                let n = u16_at(b, 0).unwrap_or(0) as usize;
                for i in 0..n {
                    let at = 2 + i * 12;
                    match (f32_at(b, at), f32_at(b, at + 4), f32_at(b, at + 8)) {
                        (Some(x), Some(y), Some(z)) => points.push([x, y, z]),
                        _ => break,
                    }
                }
            }
            0x4120 => {
                let n = u16_at(b, 0).unwrap_or(0) as usize;
                for i in 0..n {
                    let at = 2 + i * 8;
                    match (u16_at(b, at), u16_at(b, at + 2), u16_at(b, at + 4)) {
                        (Some(a), Some(c), Some(d)) => faces.push([a, c, d]),
                        _ => break,
                    }
                }
                colours = vec![GREY; faces.len()];
                for (id, g) in chunks(b.get(2 + n * 8..).unwrap_or(&[])) {
                    if id != 0x4130 {
                        continue;
                    }
                    let (name, rest) = cstr(g);
                    let rgb = materials.get(&name).copied().unwrap_or(GREY);
                    let count = u16_at(rest, 0).unwrap_or(0) as usize;
                    for k in 0..count {
                        if let Some(f) = u16_at(rest, 2 + k * 2) {
                            if let Some(c) = colours.get_mut(f as usize) {
                                *c = rgb;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for (f, rgb) in faces.iter().zip(colours) {
        let p = |i: u16| points.get(i as usize).copied();
        if let (Some(a), Some(b), Some(c)) = (p(f[0]), p(f[1]), p(f[2])) {
            tris.push(Tri { p: [a, b, c], rgb });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_servertools_own_shapes() {
        // A generated 3D world's tree: two materials, bark and leaves.
        let o = crate::generate::TerrainOptions {
            world: "t".into(),
            width: 1,
            height: 1,
            seed: 7,
            base: 0,
            amplitude: 64.0,
            chaos: 0.5,
            even_borders: true,
        };
        let files = crate::terrain3d::terrain3d(&o, &Default::default()).unwrap();
        let tree = &files.iter().find(|(n, _)| n == "t_tree.3ds").unwrap().1;
        let tris = read_3ds(tree).unwrap();
        assert!(!tris.is_empty());
        let colours: std::collections::HashSet<[u8; 3]> = tris.iter().map(|t| t.rgb).collect();
        assert!(colours.contains(&[100, 70, 40]), "{colours:?}");
        assert!(colours.contains(&[30, 110, 40]), "{colours:?}");
        assert!(read_3ds(b"nope").is_err());
    }
}
