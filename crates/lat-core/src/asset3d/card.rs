//! A picture as a 3D thing: an upright card facing south (-y), as wide and
//! tall as the picture at `px_per_tile` pixels a tile, cropped to its
//! opaque part. [`card`] is one textured quad (the 5.12 cuts out the
//! transparent pixels and draws both sides, T3D-032); [`blocks`] builds the
//! picture from flat coloured strips instead, for clients that draw no
//! textures (4.20r8), as `pw3d` does for NPC pictures.

use super::{texture, Face, Material, Model, Up, Vertex};
use crate::preview::Rgba;

/// Pictures over this many pixels a side are made smaller before
/// [`blocks`], so the strips stay a few thousand faces.
const BLOCKS_MAX_SIDE: u32 = 128;

/// The part of `img` with opaque pixels (all of it if none are).
fn crop(img: &Rgba) -> Rgba {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..img.height {
        for x in 0..img.width {
            if img.pixel(x, y).is_some_and(|p| p[3] >= texture::ALPHA_CUT) {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    if x0 == u32::MAX || (x0 == 0 && y0 == 0 && x1 == img.width && y1 == img.height) {
        return img.clone();
    }
    let (w, h) = (x1 - x0, y1 - y0);
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in y0..y1 {
        let at = ((y * img.width + x0) * 4) as usize;
        pixels.extend_from_slice(&img.pixels[at..at + (w * 4) as usize]);
    }
    Rgba {
        width: w,
        height: h,
        pixels,
    }
}

/// One textured quad standing on z = 0, its middle at x = 0.
pub fn card(img: Rgba, px_per_tile: f32) -> Model {
    let img = crop(&img);
    let w = img.width as f32 / px_per_tile;
    let h = img.height as f32 / px_per_tile;
    let mut m = Model::new(Up::Z);
    m.materials.push(Material {
        name: "card".into(),
        colour: texture::average(&img),
        texture: Some(0),
    });
    m.textures.push(img);
    let v = |x: f32, z: f32, u: f32, t: f32| Vertex {
        p: [x, 0.0, z],
        uv: [u, t],
    };
    m.vertices = vec![
        v(-w / 2.0, 0.0, 0.0, 0.0),
        v(w / 2.0, 0.0, 1.0, 0.0),
        v(w / 2.0, h, 1.0, 1.0),
        v(-w / 2.0, h, 0.0, 1.0),
    ];
    m.faces = vec![Face { v: [0, 1, 2], m: 0 }, Face { v: [0, 2, 3], m: 0 }];
    m
}

/// The picture as runs of equal colour, one flat strip a run and row.
pub fn blocks(img: &Rgba, px_per_tile: f32, colour_bits: u8) -> Model {
    let mut img = crop(img);
    let mut px = px_per_tile;
    if img.width > BLOCKS_MAX_SIDE || img.height > BLOCKS_MAX_SIDE {
        let before = img.width;
        img = texture::safe(&img, BLOCKS_MAX_SIDE);
        px *= img.width as f32 / before as f32;
    }
    let mut m = Model::new(Up::Z);
    if img.width > 0 {
        m.notes.push(format!(
            "built from coloured strips ({} x {} pixels, {colour_bits} bits a channel)",
            img.width, img.height
        ));
    }
    let w = img.width as f32 / px;
    let h = img.height as f32 / px;
    for y in 0..img.height {
        let mut x = 0;
        while x < img.width {
            let colour = |x: u32| {
                img.pixel(x, y)
                    .filter(|p| p[3] >= texture::ALPHA_CUT)
                    .map(|p| texture::quantise([p[0], p[1], p[2]], colour_bits))
            };
            let Some(c) = colour(x) else {
                x += 1;
                continue;
            };
            let mut end = x + 1;
            while end < img.width && colour(end) == Some(c) {
                end += 1;
            }
            let mi = m.colour_material(c);
            let (x0, x1) = (x as f32 / px - w / 2.0, end as f32 / px - w / 2.0);
            let (z1, z0) = (h - y as f32 / px, h - (y + 1) as f32 / px);
            let base = m.vertices.len() as u32;
            for (px_, pz) in [(x0, z0), (x1, z0), (x1, z1), (x0, z1)] {
                m.vertices.push(Vertex {
                    p: [px_, 0.0, pz],
                    uv: [0.0, 0.0],
                });
            }
            m.faces.push(Face {
                v: [base, base + 1, base + 2],
                m: mi,
            });
            m.faces.push(Face {
                v: [base, base + 2, base + 3],
                m: mi,
            });
            x = end;
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4 x 2: a transparent column on the left, then red, red, blue.
    fn pic() -> Rgba {
        let mut p = Vec::new();
        for _ in 0..2 {
            p.extend([0, 0, 0, 0]);
            p.extend([255, 0, 0, 255]);
            p.extend([255, 0, 0, 255]);
            p.extend([0, 0, 255, 255]);
        }
        Rgba {
            width: 4,
            height: 2,
            pixels: p,
        }
    }

    #[test]
    fn a_card_is_cropped_and_stands_up() {
        let m = card(pic(), 1.0);
        assert_eq!(m.textures[0].width, 3);
        let (lo, hi) = m.bounds().unwrap();
        assert_eq!(lo, [-1.5, 0.0, 0.0]);
        assert_eq!(hi, [1.5, 0.0, 2.0]);
        assert_eq!(m.vertices[2].uv, [1.0, 1.0]);
    }

    #[test]
    fn blocks_merge_runs_of_one_colour() {
        let m = blocks(&pic(), 1.0, 8);
        // Two rows, two runs each, two triangles a run.
        assert_eq!(m.faces.len(), 8);
        assert_eq!(m.materials.len(), 2);
    }
}
