//! Reading a 3D Studio `.3ds` for conversion: meshes with their texture
//! coordinates, material colours and texture maps. Points are taken as
//! stored (3D Studio keeps them in world space; the mesh matrix is not
//! applied), as in `world3d::shape`.

use super::{texture, Face, FileLoader, Material, Model, Up, Vertex};
use std::collections::HashMap;

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

fn cstr(data: &[u8]) -> (String, &[u8]) {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let name = String::from_utf8_lossy(&data[..end]).into_owned();
    (name, data.get(end + 1..).unwrap_or(&[]))
}

fn u16_at(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(i)?, *d.get(i + 1)?]))
}

fn f32_at(d: &[u8], i: usize) -> Option<f32> {
    Some(f32::from_le_bytes(d.get(i..i + 4)?.try_into().ok()?))
}

fn colour(body: &[u8]) -> Option<[u8; 3]> {
    for (id, c) in chunks(body) {
        match id {
            0x0011 | 0x0012 if c.len() >= 3 => return Some([c[0], c[1], c[2]]),
            0x0010 | 0x0013 if c.len() >= 12 => {
                let f =
                    |i: usize| (f32_at(c, i).unwrap_or(0.0).clamp(0.0, 1.0) * 255.0).round() as u8;
                return Some([f(0), f(4), f(8)]);
            }
            _ => {}
        }
    }
    None
}

/// Every mesh of a `.3ds` as one [`Model`]; texture maps are loaded from
/// beside the file.
pub fn read(data: &[u8], files: &FileLoader) -> Result<Model, String> {
    let top = chunks(data);
    let main = match top.first() {
        Some((0x4D4D, body)) => *body,
        _ => return Err("not a .3ds file (no 0x4D4D chunk)".into()),
    };
    let mut m = Model::new(Up::Z);
    let mut by_name: HashMap<String, usize> = HashMap::new();
    let mut loaded: HashMap<String, Option<usize>> = HashMap::new();
    for (id, editor) in chunks(main) {
        if id != 0x3D3D {
            continue;
        }
        for (id, body) in chunks(editor) {
            if id != 0xAFFF {
                continue;
            }
            let mut name = String::new();
            let mut rgb = None;
            let mut map = None;
            for (id, b) in chunks(body) {
                match id {
                    0xA000 => name = cstr(b).0,
                    0xA020 => rgb = colour(b),
                    0xA200 => {
                        for (id, t) in chunks(b) {
                            if id == 0xA300 {
                                map = Some(cstr(t).0);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let texture = match map.filter(|s| !s.trim().is_empty()) {
                Some(file) => *loaded.entry(file.clone()).or_insert_with(|| {
                    match files.load(&file).and_then(|b| texture::decode(&b)) {
                        Ok(img) => {
                            m.textures.push(img);
                            Some(m.textures.len() - 1)
                        }
                        Err(e) => {
                            m.notes
                                .push(format!("texture {e}; its material colour is used"));
                            None
                        }
                    }
                }),
                None => None,
            };
            by_name.insert(name.clone(), m.materials.len());
            m.materials.push(Material {
                name,
                colour: rgb.unwrap_or([170, 170, 170]),
                texture,
            });
        }
        for (id, body) in chunks(editor) {
            if id != 0x4000 {
                continue;
            }
            let (_, rest) = cstr(body);
            for (id, mesh) in chunks(rest) {
                if id == 0x4100 {
                    read_mesh(mesh, &by_name, &mut m);
                }
            }
        }
    }
    if m.faces.is_empty() {
        return Err("the .3ds has no faces".into());
    }
    Ok(m)
}

fn read_mesh(mesh: &[u8], by_name: &HashMap<String, usize>, m: &mut Model) {
    let base = m.vertices.len() as u32;
    let mut points: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut faces: Vec<[u32; 3]> = Vec::new();
    let mut mats: Vec<Option<usize>> = Vec::new();
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
            0x4140 => {
                let n = u16_at(b, 0).unwrap_or(0) as usize;
                for i in 0..n {
                    let at = 2 + i * 8;
                    match (f32_at(b, at), f32_at(b, at + 4)) {
                        (Some(u), Some(v)) => uvs.push([u, v]),
                        _ => break,
                    }
                }
            }
            0x4120 => {
                let n = u16_at(b, 0).unwrap_or(0) as usize;
                for i in 0..n {
                    let at = 2 + i * 8;
                    match (u16_at(b, at), u16_at(b, at + 2), u16_at(b, at + 4)) {
                        (Some(a), Some(c), Some(d)) => faces.push([a as u32, c as u32, d as u32]),
                        _ => break,
                    }
                }
                mats = vec![None; faces.len()];
                for (id, g) in chunks(b.get(2 + n * 8..).unwrap_or(&[])) {
                    if id != 0x4130 {
                        continue;
                    }
                    let (name, rest) = cstr(g);
                    let mi = by_name.get(&name).copied();
                    let count = u16_at(rest, 0).unwrap_or(0) as usize;
                    for k in 0..count {
                        if let Some(f) = u16_at(rest, 2 + k * 2) {
                            if let Some(slot) = mats.get_mut(f as usize) {
                                *slot = mi;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for (i, p) in points.iter().enumerate() {
        m.vertices.push(Vertex {
            p: *p,
            uv: uvs.get(i).copied().unwrap_or([0.0, 0.0]),
        });
    }
    let mut grey = None;
    for (f, mi) in faces.iter().zip(mats) {
        if f.iter().any(|&v| v as usize >= points.len()) {
            continue;
        }
        let mi = match mi {
            Some(i) => i,
            // A face with no material is not drawn by the 5.12; give it grey.
            None => *grey.get_or_insert_with(|| m.colour_material([170, 170, 170])),
        };
        m.faces.push(Face {
            v: f.map(|v| v + base),
            m: mi,
        });
    }
}

/// The texture file names the materials of a `.3ds` name, as written.
pub fn texture_names(data: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for (id, main) in chunks(data) {
        if id != 0x4D4D {
            continue;
        }
        for (id, editor) in chunks(main) {
            if id != 0x3D3D {
                continue;
            }
            for (id, mat) in chunks(editor) {
                if id != 0xAFFF {
                    continue;
                }
                for (id, map) in chunks(mat) {
                    if id == 0xA200 {
                        for (id, t) in chunks(map) {
                            if id == 0xA300 {
                                out.push(cstr(t).0);
                            }
                        }
                    }
                }
            }
        }
    }
    out
}
