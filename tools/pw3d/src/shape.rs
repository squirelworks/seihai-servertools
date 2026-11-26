//! Coloured triangle meshes and the `.3ds` files the 3D clients load.
//!
//! The writer follows the published 3DS chunk layout (main 0x4D4D, editor
//! 0x3D3D, materials 0xAFFF, objects 0x4000, triangle meshes 0x4100), as
//! `lat-core`'s terrain writer does. Every face has a material: a 5.12 or
//! 4.20r6 client does not draw a `.3ds` without one (seihai-gserver lab,
//! 2026-11-20). Faces are wound counter-clockwise seen from outside.
//!
//! A mesh is split into objects of at most 65535 points and faces, the 3DS
//! 16-bit counts.

/// A colour material, named after its colour so equal colours share one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Colour(pub [u8; 3]);

impl Colour {
    pub fn name(self) -> String {
        let [r, g, b] = self.0;
        format!("pw{r:02x}{g:02x}{b:02x}")
    }

    /// The same colour, darker: the sides of raised tiles.
    pub fn shade(self, f: f32) -> Colour {
        Colour(
            self.0
                .map(|c| (f32::from(c) * f).round().clamp(0.0, 255.0) as u8),
        )
    }
}

/// Points and faces under construction, any number of them.
#[derive(Debug, Default, Clone)]
pub struct Shape {
    pub points: Vec<[f32; 3]>,
    pub faces: Vec<([u32; 3], Colour)>,
}

impl Shape {
    /// A quad `a b c d`, counter-clockwise seen from outside.
    pub fn quad(&mut self, q: [[f32; 3]; 4], c: Colour) {
        let b = self.points.len() as u32;
        self.points.extend_from_slice(&q);
        self.faces.push(([b, b + 1, b + 2], c));
        self.faces.push(([b, b + 2, b + 3], c));
    }

    /// An axis-aligned box from `lo` to `hi`, top and sides in `top` and
    /// `side`, and its bottom left open (it sits on something).
    pub fn block(&mut self, lo: [f32; 3], hi: [f32; 3], top: Colour, side: Colour) {
        let [x0, y0, z0] = lo;
        let [x1, y1, z1] = hi;
        self.quad(
            [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
            top,
        );
        // -y, +x, +y, -x.
        self.quad(
            [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
            side,
        );
        self.quad(
            [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
            side,
        );
        self.quad(
            [[x1, y1, z0], [x0, y1, z0], [x0, y1, z1], [x1, y1, z1]],
            side,
        );
        self.quad(
            [[x0, y1, z0], [x0, y0, z0], [x0, y0, z1], [x0, y1, z1]],
            side,
        );
    }

    pub fn colours(&self) -> Vec<Colour> {
        let mut c: Vec<Colour> = self.faces.iter().map(|f| f.1).collect();
        c.sort();
        c.dedup();
        c
    }

    /// The whole shape as one `.3ds` file.
    pub fn to_3ds(&self, name: &str) -> Vec<u8> {
        write_3ds(name, self)
    }
}

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

fn colour_chunk(id: u16, rgb: [u8; 3]) -> Vec<u8> {
    chunk(id, &chunk(0x0011, &rgb))
}

/// Cut a shape into pieces whose point and face counts fit 16 bits. Faces
/// keep their points together (quads are never split across pieces, since
/// each face's points are its own or its quad's).
fn pieces(s: &Shape) -> Vec<Shape> {
    const MAX: usize = u16::MAX as usize;
    let mut out = Vec::new();
    let mut cur = Shape::default();
    let mut map: std::collections::HashMap<u32, u32> = Default::default();
    for (f, c) in &s.faces {
        let new_points = f.iter().filter(|p| !map.contains_key(p)).count();
        if cur.points.len() + new_points > MAX || cur.faces.len() + 1 > MAX {
            out.push(std::mem::take(&mut cur));
            map.clear();
        }
        let mut g = [0u32; 3];
        for (k, p) in f.iter().enumerate() {
            g[k] = *map.entry(*p).or_insert_with(|| {
                cur.points.push(s.points[*p as usize]);
                (cur.points.len() - 1) as u32
            });
        }
        cur.faces.push((g, *c));
    }
    if !cur.faces.is_empty() {
        out.push(cur);
    }
    out
}

fn write_3ds(name: &str, s: &Shape) -> Vec<u8> {
    let colours = s.colours();
    let mut editor = chunk(0x3D3E, &3u32.to_le_bytes());
    for c in &colours {
        let mut body = chunk(0xA000, &cstr(&c.name()));
        body.extend(colour_chunk(0xA010, c.0.map(|v| v / 2)));
        body.extend(colour_chunk(0xA020, c.0));
        body.extend(colour_chunk(0xA030, [25, 25, 25]));
        editor.extend(chunk(0xAFFF, &body));
    }
    for (n, piece) in pieces(s).iter().enumerate() {
        let mut points = (piece.points.len() as u16).to_le_bytes().to_vec();
        for p in &piece.points {
            for v in p {
                points.extend_from_slice(&v.to_le_bytes());
            }
        }
        let mut faces = (piece.faces.len() as u16).to_le_bytes().to_vec();
        for (f, _) in &piece.faces {
            for v in f {
                faces.extend_from_slice(&(*v as u16).to_le_bytes());
            }
            faces.extend_from_slice(&7u16.to_le_bytes()); // edge flags
        }
        for c in &colours {
            let used: Vec<u16> = piece
                .faces
                .iter()
                .enumerate()
                .filter(|(_, (_, fc))| fc == c)
                .map(|(i, _)| i as u16)
                .collect();
            if used.is_empty() {
                continue;
            }
            let mut group = cstr(&c.name());
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
        let mut object = cstr(&format!("{name}{n}"));
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

    fn normal(s: &Shape, f: [u32; 3]) -> [f32; 3] {
        let p = |i: u32| s.points[i as usize];
        let (a, b, c) = (p(f[0]), p(f[1]), p(f[2]));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    }

    #[test]
    fn block_faces_point_outwards() {
        let mut s = Shape::default();
        let c = Colour([200, 10, 10]);
        s.block([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], c, c.shade(0.5));
        let centre = [0.5f32, 0.5, 0.5];
        assert_eq!(s.faces.len(), 10);
        for (f, _) in &s.faces {
            let n = normal(&s, *f);
            let p = s.points[f[0] as usize];
            let out = [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]];
            let dot = n[0] * out[0] + n[1] * out[1] + n[2] * out[2];
            assert!(dot > 0.0, "face {f:?} faces inwards");
        }
    }

    #[test]
    fn big_shapes_split_into_16_bit_pieces() {
        let mut s = Shape::default();
        let c = Colour([1, 2, 3]);
        for i in 0..20_000 {
            let x = i as f32;
            s.quad(
                [
                    [x, 0.0, 0.0],
                    [x + 1.0, 0.0, 0.0],
                    [x + 1.0, 1.0, 0.0],
                    [x, 1.0, 0.0],
                ],
                c,
            );
        }
        let p = pieces(&s);
        assert!(p.len() >= 2);
        assert!(p
            .iter()
            .all(|p| p.points.len() <= 65535 && p.faces.len() <= 65535));
        assert_eq!(p.iter().map(|p| p.faces.len()).sum::<usize>(), 40_000);
        let file = s.to_3ds("g");
        assert_eq!(&file[0..2], &0x4D4Du16.to_le_bytes());
        assert_eq!(
            u32::from_le_bytes(file[2..6].try_into().unwrap()) as usize,
            file.len()
        );
    }
}
