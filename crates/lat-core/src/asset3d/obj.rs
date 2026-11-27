//! Reading Wavefront `.obj` with its `.mtl` material libraries: points
//! (`v`), texture places (`vt`), faces (`f`, polygons cut into fans),
//! `usemtl`, and the libraries' `Kd` colour and `map_Kd` texture. Normals,
//! groups, smoothing and the other maps are ignored.

use super::{texture, Face, FileLoader, Material, Model, Up, Vertex};
use std::collections::HashMap;

struct Mtl {
    colour: [u8; 3],
    map: Option<String>,
}

fn floats(words: &[&str]) -> Vec<f32> {
    words.iter().filter_map(|w| w.parse::<f32>().ok()).collect()
}

fn read_mtl(text: &str, out: &mut HashMap<String, Mtl>) {
    let mut cur: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first().copied() {
            Some("newmtl") => {
                let name = words[1..].join(" ");
                out.insert(
                    name.clone(),
                    Mtl {
                        colour: [200, 200, 200],
                        map: None,
                    },
                );
                cur = Some(name);
            }
            Some("Kd") => {
                let f = floats(&words[1..]);
                if let (Some(name), [r, g, b, ..]) = (&cur, f.as_slice()) {
                    if let Some(m) = out.get_mut(name) {
                        m.colour = [*r, *g, *b].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
                    }
                }
            }
            Some(k) if k.eq_ignore_ascii_case("map_Kd") => {
                // Options (`-s 1 1 1`, `-bm 1`, ...) come first; the file
                // name is the last word.
                if let (Some(name), Some(file)) = (&cur, words.last().filter(|_| words.len() > 1)) {
                    if let Some(m) = out.get_mut(name) {
                        m.map = Some(file.to_string());
                    }
                }
            }
            _ => {}
        }
    }
}

/// Read an `.obj`'s bytes; `files` finds its `.mtl` and textures.
pub fn read(bytes: &[u8], files: &FileLoader) -> Result<Model, String> {
    let text = String::from_utf8_lossy(bytes);
    let mut m = Model::new(Up::Y);
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut mtls: HashMap<String, Mtl> = HashMap::new();
    let mut used: HashMap<String, usize> = HashMap::new();
    let mut textures: HashMap<String, Option<usize>> = HashMap::new();
    let mut verts: HashMap<(usize, Option<usize>), u32> = HashMap::new();
    let mut current: Option<String> = None;
    let mut skipped_lines = 0usize;
    for line in text.lines() {
        let line = line.trim();
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first().copied() {
            Some("v") => {
                let f = floats(&words[1..]);
                if f.len() >= 3 {
                    positions.push([f[0], f[1], f[2]]);
                }
            }
            Some("vt") => {
                let f = floats(&words[1..]);
                uvs.push([
                    f.first().copied().unwrap_or(0.0),
                    f.get(1).copied().unwrap_or(0.0),
                ]);
            }
            Some("mtllib") => {
                let name = words[1..].join(" ");
                match files.load(&name) {
                    Ok(b) => read_mtl(&String::from_utf8_lossy(&b), &mut mtls),
                    Err(e) => m.notes.push(format!("material library {e}; grey is used")),
                }
            }
            Some("usemtl") => current = Some(words[1..].join(" ")),
            Some("f") => {
                let key = current.clone().unwrap_or_default();
                let mi = match used.get(&key) {
                    Some(i) => *i,
                    None => {
                        let (colour, map) = match mtls.get(&key) {
                            Some(t) => (t.colour, t.map.clone()),
                            None => ([200, 200, 200], None),
                        };
                        let texture = match map {
                            Some(file) => *textures.entry(file.clone()).or_insert_with(|| {
                                match files.load(&file).and_then(|b| texture::decode(&b)) {
                                    Ok(img) => {
                                        m.textures.push(img);
                                        Some(m.textures.len() - 1)
                                    }
                                    Err(e) => {
                                        m.notes.push(format!(
                                            "texture {e}; the material colour is used"
                                        ));
                                        None
                                    }
                                }
                            }),
                            None => None,
                        };
                        m.materials.push(Material {
                            name: if key.is_empty() {
                                "default".into()
                            } else {
                                key.clone()
                            },
                            colour,
                            texture,
                        });
                        used.insert(key, m.materials.len() - 1);
                        m.materials.len() - 1
                    }
                };
                let mut poly: Vec<u32> = Vec::new();
                let mut bad = false;
                for w in &words[1..] {
                    let mut parts = w.split('/');
                    let resolve = |s: Option<&str>, len: usize| -> Option<usize> {
                        let i: i64 = s?.parse().ok()?;
                        let i = if i < 0 { len as i64 + i } else { i - 1 };
                        (0..len as i64).contains(&i).then_some(i as usize)
                    };
                    let Some(pi) = resolve(parts.next(), positions.len()) else {
                        bad = true;
                        break;
                    };
                    let ti = resolve(parts.next().filter(|s| !s.is_empty()), uvs.len());
                    let idx = *verts.entry((pi, ti)).or_insert_with(|| {
                        m.vertices.push(Vertex {
                            p: positions[pi],
                            uv: ti.map(|t| uvs[t]).unwrap_or([0.0, 0.0]),
                        });
                        (m.vertices.len() - 1) as u32
                    });
                    poly.push(idx);
                }
                if bad || poly.len() < 3 {
                    skipped_lines += 1;
                    continue;
                }
                for k in 1..poly.len() - 1 {
                    m.faces.push(Face {
                        v: [poly[0], poly[k], poly[k + 1]],
                        m: mi,
                    });
                }
            }
            _ => {}
        }
    }
    if skipped_lines > 0 {
        m.notes.push(format!(
            "{skipped_lines} faces with bad point numbers skipped"
        ));
    }
    if m.faces.is_empty() {
        return Err("the .obj has no faces".into());
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_textured_quad_with_its_library() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("q.mtl"),
            "newmtl wood\nKd 1 0.5 0\nmap_Kd -s 1 1 1 Wood.PNG\nnewmtl red\nKd 1 0 0\n",
        )
        .unwrap();
        let img = crate::preview::Rgba {
            width: 2,
            height: 2,
            pixels: [9u8, 8, 7, 255].repeat(4),
        };
        std::fs::write(dir.path().join("wood.png"), texture::png(&img).unwrap()).unwrap();
        let obj =
            "mtllib q.mtl\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\n\
                   usemtl wood\nf 1/1 2/2 3/3 4/4\nusemtl red\nf -4 -3 -2\n";
        let m = read(
            obj.as_bytes(),
            &FileLoader {
                dir: dir.path().into(),
            },
        )
        .unwrap();
        assert_eq!(m.faces.len(), 3);
        assert_eq!(m.materials.len(), 2);
        assert_eq!(m.materials[0].colour, [255, 128, 0]);
        assert_eq!(m.materials[0].texture, Some(0));
        assert_eq!(m.materials[1].texture, None);
        assert_eq!(m.textures[0], img);
        assert_eq!(m.vertices[2].uv, [1.0, 1.0]);
        assert!(m.notes.is_empty(), "{:?}", m.notes);
    }
}
