//! NPCs as 3D things that look like their 2D picture.
//!
//! A level NPC shows an image (or the part `setimgpart` picks). Where the
//! image is found, it stands up as a cut-out: the picture's opaque pixels
//! in small squares, as wide and tall as the 2D image (16 pixels a tile),
//! a thin board facing south with the same picture on its back. A door
//! picture becomes a door standing in the house front, a lamp a lamp, a
//! guard a flat guard. An NPC shown as a gani, an image too big to be one
//! thing (a body sheet), or one not found becomes a figure: a body and a
//! head, in the picture's colours where there is a picture.

use super::shape::{Colour, Shape};
use crate::preview::Rgba;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Pixels a tile.
const PX: f32 = 16.0;
/// An image wider or taller than this is a sheet, not one thing.
const MAX_PX: u32 = 160;
/// Thickness of a cut-out, in tiles.
const DEPTH: f32 = 0.12;

/// The pictures NPCs can name: every file under the given folders, by
/// lower-cased file name, read when first asked for.
#[derive(Default)]
pub struct Images {
    paths: HashMap<String, PathBuf>,
    cache: HashMap<String, Option<Rgba>>,
}

impl Images {
    pub fn new(files: impl IntoIterator<Item = PathBuf>) -> Images {
        let mut paths = HashMap::new();
        for p in files {
            let Some(name) = p.file_name().map(|n| n.to_string_lossy().to_lowercase()) else {
                continue;
            };
            if matches!(
                Path::new(&name).extension().and_then(|e| e.to_str()),
                Some("png" | "gif" | "mng")
            ) {
                paths.entry(name).or_insert(p);
            }
        }
        Images {
            paths,
            cache: HashMap::new(),
        }
    }

    /// The picture called `name` (any folder, any case), if there is one.
    pub fn get(&mut self, name: &str) -> Option<&Rgba> {
        let key = name
            .trim()
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .to_lowercase();
        if !self.cache.contains_key(&key) {
            let img = self
                .paths
                .get(&key)
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| crate::render::decode_image(&b));
            self.cache.insert(key.clone(), img);
        }
        self.cache.get(&key).and_then(|o| o.as_ref())
    }
}

/// How an NPC came out, for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Made {
    /// A cut-out of its picture.
    Picture,
    /// A figure in its picture's colours (a sheet).
    Figure,
    /// A figure, its picture not found or it shows a gani.
    Guess,
}

/// The opaque part of `img` inside `part` (x, y, w, h in pixels), as
/// (x0, y0, x1, y1), or `None` if nothing there is opaque.
fn opaque_box(img: &Rgba, part: (u32, u32, u32, u32)) -> Option<(u32, u32, u32, u32)> {
    let (px, py, pw, ph) = part;
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for y in py..(py + ph).min(img.height) {
        for x in px..(px + pw).min(img.width) {
            if img.pixels[((y * img.width + x) * 4 + 3) as usize] >= 128 {
                b = Some(match b {
                    None => (x, y, x + 1, y + 1),
                    Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x + 1), e.max(y + 1)),
                });
            }
        }
    }
    b
}

/// The average colour of the opaque pixels of a cell, if at least half
/// of it is opaque.
fn cell(img: &Rgba, x0: u32, y0: u32, x1: u32, y1: u32) -> Option<[u32; 3]> {
    let (mut s, mut n, mut all) = ([0u32; 3], 0, 0);
    for y in y0..y1.min(img.height) {
        for x in x0..x1.min(img.width) {
            all += 1;
            let i = ((y * img.width + x) * 4) as usize;
            if img.pixels[i + 3] >= 128 {
                for (k, v) in s.iter_mut().enumerate() {
                    *v += u32::from(img.pixels[i + k]);
                }
                n += 1;
            }
        }
    }
    (n > 0 && n * 2 >= all).then(|| s.map(|v| v / n))
}

/// The average colour of the opaque pixels in rows `y0..y1` of a box.
fn band(img: &Rgba, b: (u32, u32, u32, u32), y0: u32, y1: u32) -> Option<[u32; 3]> {
    let (mut s, mut n) = ([0u64; 3], 0u64);
    for y in y0.max(b.1)..y1.min(b.3) {
        for x in b.0..b.2 {
            let i = ((y * img.width + x) * 4) as usize;
            if img.pixels[i + 3] >= 128 {
                for (k, v) in s.iter_mut().enumerate() {
                    *v += u64::from(img.pixels[i + k]);
                }
                n += 1;
            }
        }
    }
    (n > 0).then(|| s.map(|v| (v / n) as u32))
}

/// A person-sized figure centred on (`cx`, `cy`) (Torque x, y) on `z`.
pub fn figure(cx: f32, cy: f32, z: f32, body: Colour, head: Colour) -> Shape {
    let mut s = Shape::default();
    s.block(
        [cx - 0.35, cy - 0.25, z],
        [cx + 0.35, cy + 0.25, z + 1.1],
        body,
        body.shade(0.75),
    );
    s.block(
        [cx - 0.28, cy - 0.28, z + 1.1],
        [cx + 0.28, cy + 0.28, z + 1.6],
        head,
        head.shade(0.8),
    );
    s
}

/// The 3D thing for an NPC whose image's top left corner is at tile
/// (`x`, `y`), on ground `z`. `quantise` maps a colour to a material.
pub fn npc_shape(
    img: Option<&Rgba>,
    part: Option<(i32, i32, i32, i32)>,
    x: f32,
    y: f32,
    z: f32,
    quantise: &dyn Fn([u32; 3]) -> Colour,
) -> (Shape, Made) {
    const SKIN: Colour = Colour([230, 190, 150]);
    // Not the player's blue (convert.rs), so an NPC is not taken for it.
    const CLOTH: Colour = Colour([150, 120, 80]);
    let Some(img) = img else {
        return (figure(x + 1.0, -y - 1.5, z, CLOTH, SKIN), Made::Guess);
    };
    let whole = (0, 0, img.width, img.height);
    let part = part
        .and_then(|(px, py, pw, ph)| {
            (px >= 0 && py >= 0 && pw > 0 && ph > 0)
                .then_some((px as u32, py as u32, pw as u32, ph as u32))
        })
        .unwrap_or(whole);
    let Some(b) = opaque_box(img, part) else {
        return (Shape::default(), Made::Picture);
    };
    let (bw, bh) = (b.2 - b.0, b.3 - b.1);
    if bw > MAX_PX || bh > MAX_PX {
        // A sheet of many poses: a figure in its colours.
        let head = band(img, b, b.1, b.1 + bh / 3).map_or(SKIN, quantise);
        let body = band(img, b, b.1 + bh / 3, b.3).map_or(CLOTH, quantise);
        return (figure(x + 1.0, -y - 1.5, z, body, head), Made::Figure);
    }
    // The cut-out stands on the bottom edge of the picture's footprint.
    // At most about 16 squares along the longer side, so a big picture
    // (a 128 x 160 tree) stays a small shape.
    let step = bw.max(bh).div_ceil(16).max(2);
    let base_y = -(y + b.3 as f32 / PX) + 0.5;
    let (yf, yb) = (base_y - DEPTH / 2.0, base_y + DEPTH / 2.0);
    let top = z + bh as f32 / PX;
    let mut s = Shape::default();
    let mut cy = b.1;
    while cy < b.3 {
        // Runs of equal colour along a row of cells.
        let mut run: Option<(u32, Colour)> = None;
        let mut open_left = true;
        let mut cx = b.0;
        let z1 = top - (cy - b.1) as f32 / PX;
        let z0 = top - ((cy + step).min(b.3) - b.1) as f32 / PX;
        loop {
            let c = (cx < b.2)
                .then(|| cell(img, cx, cy, cx + step, cy + step))
                .flatten()
                .map(quantise);
            if run.map(|r| r.1) != c {
                if let Some((start, col)) = run {
                    let (x0, x1) = (x + start as f32 / PX, x + cx.min(b.2) as f32 / PX);
                    // Front (south) and back.
                    s.quad(
                        [[x0, yf, z0], [x1, yf, z0], [x1, yf, z1], [x0, yf, z1]],
                        col,
                    );
                    s.quad(
                        [[x1, yb, z0], [x0, yb, z0], [x0, yb, z1], [x1, yb, z1]],
                        col.shade(0.8),
                    );
                    // Edges only where the picture ends, so a board of many
                    // colours stays two faces a run.
                    if open_left {
                        s.quad(
                            [[x0, yb, z0], [x0, yf, z0], [x0, yf, z1], [x0, yb, z1]],
                            col.shade(0.7),
                        );
                    }
                    if c.is_none() {
                        s.quad(
                            [[x1, yf, z0], [x1, yb, z0], [x1, yb, z1], [x1, yf, z1]],
                            col.shade(0.7),
                        );
                    }
                }
                open_left = run.is_none();
                run = c.map(|col| (cx, col));
            }
            if cx >= b.2 {
                break;
            }
            cx += step;
        }
        cy += step;
    }
    // A big picture (a tree, 3 tiles wide and over 6 tall) is also a
    // board facing east, the same
    // picture turned a quarter round its middle, so it does not vanish
    // when seen from the side.
    if bh > 96 && bw >= 48 {
        let cx = x + (b.0 + b.2) as f32 / 2.0 / PX;
        let mut turned = s.clone();
        for p in &mut turned.points {
            let (dx, dy) = (p[0] - cx, p[1] - base_y);
            p[0] = cx + dy;
            p[1] = base_y - dx;
        }
        s.extend(&turned);
    }
    (s, Made::Picture)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(w: u32, h: u32, f: impl Fn(u32, u32) -> Option<[u8; 3]>) -> Rgba {
        let mut pixels = Vec::new();
        for y in 0..h {
            for x in 0..w {
                match f(x, y) {
                    Some(c) => pixels.extend_from_slice(&[c[0], c[1], c[2], 255]),
                    None => pixels.extend_from_slice(&[0, 0, 0, 0]),
                }
            }
        }
        Rgba {
            width: w,
            height: h,
            pixels,
        }
    }

    fn q(c: [u32; 3]) -> Colour {
        Colour(c.map(|v| v as u8))
    }

    #[test]
    fn a_picture_stands_up_as_tall_and_wide_as_in_2d() {
        // A 32 x 48 door: brown with a dark middle, transparent round it.
        let img = picture(40, 56, |x, y| {
            ((4..36).contains(&x) && (4..52).contains(&y)).then_some(
                if (12..28).contains(&x) && y > 20 {
                    [40, 20, 10]
                } else {
                    [150, 100, 50]
                },
            )
        });
        let (s, made) = npc_shape(Some(&img), None, 10.0, 20.0, 1.0, &q);
        assert_eq!(made, Made::Picture);
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in &s.points {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        assert!((hi[0] - lo[0] - 2.0).abs() < 1e-4, "{lo:?} {hi:?}");
        assert!((hi[2] - lo[2] - 3.0).abs() < 1e-4 && (lo[2] - 1.0).abs() < 1e-4);
        assert!(hi[1] - lo[1] < 0.2, "a thin board");
        let cs = s.colours();
        assert!(cs.contains(&Colour([40, 20, 10])) && cs.contains(&Colour([150, 100, 50])));
        assert!(!cs.contains(&Colour([0, 0, 0])), "no transparent cells");
    }

    #[test]
    fn imgpart_and_sheets_and_missing_pictures() {
        // Only the part's left half is opaque: the cut-out is 1 tile wide.
        let img = picture(64, 16, |x, _| (x < 16).then_some([200, 0, 0]));
        let (s, _) = npc_shape(Some(&img), Some((0, 0, 32, 16)), 0.0, 0.0, 0.0, &q);
        let w = s.points.iter().map(|p| p[0]).fold(0.0, f32::max);
        assert!((w - 1.0).abs() < 1e-4, "{w}");
        let sheet = picture(200, 200, |_, y| {
            Some(if y < 60 { [240, 200, 160] } else { [0, 0, 200] })
        });
        assert_eq!(
            npc_shape(Some(&sheet), None, 0.0, 0.0, 0.0, &q).1,
            Made::Figure
        );
        assert_eq!(npc_shape(None, None, 0.0, 0.0, 0.0, &q).1, Made::Guess);
        // A 128 x 160 picture stays at most 16 x 16 squares (5 quads each).
        let tree = picture(128, 160, |x, y| Some([(x * 2) as u8, y as u8, 90]));
        let (s, made) = npc_shape(Some(&tree), None, 0.0, 0.0, 0.0, &q);
        assert_eq!(made, Made::Picture);
        assert!(s.faces.len() <= 2 * 16 * 16 * 10, "{}", s.faces.len());
        // A tall picture is a cross of two boards, so it has depth both ways.
        let span = |k: usize| {
            let v = s.points.iter().map(|p| p[k]);
            v.clone().fold(f32::MIN, f32::max) - v.fold(f32::MAX, f32::min)
        };
        assert!(span(0) > 7.0 && span(1) > 7.0, "{} {}", span(0), span(1));
    }

    #[test]
    fn images_are_found_by_name_in_any_case_and_folder() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("a/b")).unwrap();
        let img = picture(2, 2, |_, _| Some([1, 2, 3]));
        let png = crate::render::png_bytes(&img).unwrap();
        std::fs::write(d.path().join("a/b/Door.PNG"), png).unwrap();
        let mut im = Images::new([d.path().join("a/b/Door.PNG"), d.path().join("x.txt")]);
        assert!(im.get("door.png").is_some());
        assert!(im.get("images/DOOR.png").is_some());
        assert!(im.get("nope.gif").is_none());
    }
}
