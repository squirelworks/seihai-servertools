//! The `.3ds` writer for converted assets: the published 3DS chunk layout
//! (main 0x4D4D, editor 0x3D3D, materials 0xAFFF, objects 0x4000 with a
//! triangle mesh 0x4100), as the other servertools writers use, plus
//! texture coordinates (0x4140) and a texture map a material (0xA200 with
//! its file name 0xA300).
//!
//! The mesh is cut into chunks of at most 65 535 points and faces, the
//! 16-bit counts of the format; each chunk keeps only the points its faces
//! use. Points are written in shape space with an identity mesh matrix.

use super::{Face, Vertex};

/// A material as written: its name, colour and texture file (if any).
#[derive(Debug, Clone, PartialEq)]
pub struct OutMaterial {
    pub name: String,
    pub colour: [u8; 3],
    pub texture: Option<String>,
}

const MAX: usize = u16::MAX as usize;

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

/// One mesh chunk: its own points and faces (indices into its points).
pub struct Piece {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
}

/// Cut the faces into pieces whose point and face counts fit 16 bits.
pub fn pieces(vertices: &[Vertex], faces: &[Face]) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut cur = Piece {
        vertices: Vec::new(),
        faces: Vec::new(),
    };
    let mut map: std::collections::HashMap<u32, u32> = Default::default();
    for f in faces {
        let new = f.v.iter().filter(|v| !map.contains_key(v)).count();
        if cur.vertices.len() + new > MAX || cur.faces.len() + 1 > MAX {
            out.push(std::mem::replace(
                &mut cur,
                Piece {
                    vertices: Vec::new(),
                    faces: Vec::new(),
                },
            ));
            map.clear();
        }
        let mut g = [0u32; 3];
        for (k, v) in f.v.iter().enumerate() {
            g[k] = *map.entry(*v).or_insert_with(|| {
                cur.vertices.push(vertices[*v as usize]);
                (cur.vertices.len() - 1) as u32
            });
        }
        cur.faces.push(Face { v: g, m: f.m });
    }
    if !cur.faces.is_empty() {
        out.push(cur);
    }
    out
}

/// The whole `.3ds` file. Every face's `m` indexes `materials`.
pub fn write_3ds(materials: &[OutMaterial], vertices: &[Vertex], faces: &[Face]) -> Vec<u8> {
    let mut editor = chunk(0x3D3E, &3u32.to_le_bytes());
    for m in materials {
        let mut body = chunk(0xA000, &cstr(&m.name));
        body.extend(colour(0xA010, m.colour.map(|c| c / 2)));
        body.extend(colour(0xA020, m.colour));
        body.extend(colour(0xA030, [25, 25, 25]));
        if let Some(t) = &m.texture {
            let mut map = chunk(0x0030, &100u16.to_le_bytes());
            map.extend(chunk(0xA300, &cstr(t)));
            body.extend(chunk(0xA200, &map));
        }
        editor.extend(chunk(0xAFFF, &body));
    }
    let textured = materials.iter().any(|m| m.texture.is_some());
    for (n, piece) in pieces(vertices, faces).iter().enumerate() {
        let mut points = (piece.vertices.len() as u16).to_le_bytes().to_vec();
        for v in &piece.vertices {
            for c in v.p {
                points.extend_from_slice(&c.to_le_bytes());
            }
        }
        let mut uvs = (piece.vertices.len() as u16).to_le_bytes().to_vec();
        for v in &piece.vertices {
            for c in v.uv {
                uvs.extend_from_slice(&c.to_le_bytes());
            }
        }
        let mut fs = (piece.faces.len() as u16).to_le_bytes().to_vec();
        for f in &piece.faces {
            for v in f.v {
                fs.extend_from_slice(&(v as u16).to_le_bytes());
            }
            fs.extend_from_slice(&7u16.to_le_bytes()); // edge flags
        }
        for (mi, m) in materials.iter().enumerate() {
            let used: Vec<u16> = piece
                .faces
                .iter()
                .enumerate()
                .filter(|(_, f)| f.m == mi)
                .map(|(i, _)| i as u16)
                .collect();
            if used.is_empty() {
                continue;
            }
            let mut group = cstr(&m.name);
            group.extend_from_slice(&(used.len() as u16).to_le_bytes());
            for i in used {
                group.extend_from_slice(&i.to_le_bytes());
            }
            fs.extend(chunk(0x4130, &group));
        }
        let mut matrix = Vec::new();
        for v in [
            1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
        ] {
            matrix.extend_from_slice(&v.to_le_bytes());
        }
        let mut tri = chunk(0x4110, &points);
        if textured {
            tri.extend(chunk(0x4140, &uvs));
        }
        tri.extend(chunk(0x4120, &fs));
        tri.extend(chunk(0x4160, &matrix));
        // Plain names: Torque gives meaning to some object name prefixes
        // (collision hulls), so the asset's own name is not used here.
        let mut object = cstr(&format!("m{n}"));
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

    fn quad() -> (Vec<Vertex>, Vec<Face>) {
        let v = |x: f32, y: f32| Vertex {
            p: [x, y, 0.0],
            uv: [x, y],
        };
        (
            vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)],
            vec![Face { v: [0, 1, 2], m: 0 }, Face { v: [0, 2, 3], m: 0 }],
        )
    }

    #[test]
    fn a_textured_quad_reads_back_with_its_colour() {
        let (v, f) = quad();
        let m = OutMaterial {
            name: "t0".into(),
            colour: [10, 200, 30],
            texture: Some("crate_0.png".into()),
        };
        let file = write_3ds(&[m], &v, &f);
        let tris = crate::world3d::shape::read_3ds(&file).unwrap();
        assert_eq!(tris.len(), 2);
        assert!(tris.iter().all(|t| t.rgb == [10, 200, 30]));
        let back = super::super::tds::read(
            &file,
            &super::super::FileLoader {
                dir: std::env::temp_dir().join("no-such-asset3d-dir"),
            },
        );
        // The texture file is not there, so the reader keeps the colour.
        let back = back.unwrap();
        assert_eq!(back.faces.len(), 2);
        assert_eq!(back.vertices.len(), 4);
        assert_eq!(back.vertices[2].uv, [1.0, 1.0]);
    }

    #[test]
    fn big_meshes_are_cut_into_16_bit_pieces() {
        let n = 70_000u32;
        let vertices: Vec<Vertex> = (0..n * 3)
            .map(|i| Vertex {
                p: [i as f32, 0.0, 0.0],
                uv: [0.0, 0.0],
            })
            .collect();
        let faces: Vec<Face> = (0..n)
            .map(|i| Face {
                v: [i * 3, i * 3 + 1, i * 3 + 2],
                m: 0,
            })
            .collect();
        let p = pieces(&vertices, &faces);
        assert_eq!(p.len(), 4);
        assert!(p
            .iter()
            .all(|p| p.vertices.len() <= MAX && p.faces.len() <= MAX));
        assert_eq!(p.iter().map(|p| p.faces.len()).sum::<usize>(), n as usize);
    }
}
