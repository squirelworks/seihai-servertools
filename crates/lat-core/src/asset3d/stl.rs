//! Reading STL, binary or text: triangles in one grey material. STL has
//! no colours or textures that are agreed on, and is usually +z up.

use super::{Face, Model, Up, Vertex};

/// Read an `.stl`'s bytes.
pub fn read(bytes: &[u8]) -> Result<Model, String> {
    let mut m = Model::new(Up::Z);
    let grey = m.colour_material([180, 180, 180]);
    let tri = |m: &mut Model, p: [[f32; 3]; 3]| {
        let base = m.vertices.len() as u32;
        for q in p {
            m.vertices.push(Vertex {
                p: q,
                uv: [0.0, 0.0],
            });
        }
        m.faces.push(Face {
            v: [base, base + 1, base + 2],
            m: grey,
        });
    };
    let binary_len = bytes
        .get(80..84)
        .map(|b| 84 + 50 * u32::from_le_bytes(b.try_into().unwrap()) as usize);
    if binary_len == Some(bytes.len()) {
        let f = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        for i in 0..(bytes.len() - 84) / 50 {
            let at = 84 + i * 50 + 12;
            let p = |k: usize| [f(at + k * 12), f(at + k * 12 + 4), f(at + k * 12 + 8)];
            tri(&mut m, [p(0), p(1), p(2)]);
        }
    } else {
        let text = String::from_utf8_lossy(bytes);
        if !text.trim_start().starts_with("solid") {
            return Err("not an STL file".into());
        }
        let mut pts: Vec<[f32; 3]> = Vec::new();
        for line in text.lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            match w.first().copied() {
                Some("vertex") => {
                    let f: Vec<f32> = w[1..].iter().filter_map(|s| s.parse().ok()).collect();
                    if f.len() == 3 {
                        pts.push([f[0], f[1], f[2]]);
                    }
                }
                Some("endloop") => {
                    if pts.len() >= 3 {
                        for k in 1..pts.len() - 1 {
                            tri(&mut m, [pts[0], pts[k], pts[k + 1]]);
                        }
                    }
                    pts.clear();
                }
                _ => {}
            }
        }
    }
    if m.faces.is_empty() {
        return Err("the STL has no triangles".into());
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_text_and_binary() {
        let text = "solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid x\n";
        let m = read(text.as_bytes()).unwrap();
        assert_eq!(m.faces.len(), 1);
        let mut bin = vec![0u8; 80];
        bin.extend(2u32.to_le_bytes());
        for _ in 0..2 {
            bin.extend([0u8; 12]);
            for v in [0f32, 0., 0., 2., 0., 0., 0., 2., 0.] {
                bin.extend(v.to_le_bytes());
            }
            bin.extend([0u8; 2]);
        }
        let m = read(&bin).unwrap();
        assert_eq!(m.faces.len(), 2);
        assert_eq!(m.vertices[1].p, [2.0, 0.0, 0.0]);
        assert!(read(b"hello").is_err());
    }
}
