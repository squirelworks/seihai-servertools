//! A streamed terrain folder as one height field the editor can change.
//!
//! The folder is what a `terrain` world line names: `heights<x>-<y>.dat`
//! (x and y 0 to 15, zlib chunks, see [`crate::terrain_chunks`]) and
//! `heights.info` (one texture name a line, in material order). The field
//! is 256 x 256 squares, centred on the origin; square (i, j) is at
//! `x = (i - 128) * square`, `y = (j - 128) * square`, and a wire height is
//! 1/8 of a unit (Graal_analysis `torque/terrain-512.md`). Brushes work in
//! world units, so the editor can place them under the cursor.

use std::fs;
use std::io::Read;
use std::path::Path;

use crate::terrain_chunks::{self, CHUNK, SIZE};

/// Squares a side.
pub const SIZE_SQUARES: usize = SIZE;

/// Wire heights a unit.
pub const PER_UNIT: f64 = 8.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Wire heights, `heights[j][i]`.
    pub heights: Vec<Vec<u16>>,
    /// Material ids (indices into `names`), `materials[j][i]`.
    pub materials: Vec<Vec<u8>>,
    /// `heights.info`, in order.
    pub names: Vec<String>,
    /// Units a square, from the world line.
    pub square: u32,
    /// Chunks that were missing or did not inflate when loaded.
    pub missing: usize,
}

/// A brush: what it does to the squares within `radius` units of a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Brush {
    /// Raise (or, negative, lower) by up to this many units at the centre,
    /// falling off smoothly to nothing at the edge.
    Raise(f64),
    /// Pull toward this height (units), fully at the centre.
    Flatten(f64),
    /// Average with the neighbours, this strong (0 to 1) at the centre.
    Smooth(f64),
    /// Set this material id.
    Paint(u8),
}

fn inflate(z: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(z)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

impl Field {
    /// A flat field at height 0 with one material.
    pub fn flat(square: u32, material: &str) -> Self {
        Field {
            heights: vec![vec![0; SIZE]; SIZE],
            materials: vec![vec![0; SIZE]; SIZE],
            names: vec![material.to_string()],
            square,
            missing: 0,
        }
    }

    /// Read a terrain folder. Missing chunks stay flat at 0 and are counted.
    pub fn load(dir: &Path, square: u32) -> Result<Self, String> {
        if !dir.is_dir() {
            return Err(format!("{}: no such terrain folder", dir.display()));
        }
        let mut f = Field::flat(square, "");
        f.names = fs::read_to_string(dir.join("heights.info"))
            .map(|t| {
                t.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        for cy in 0..CHUNK {
            for cx in 0..CHUNK {
                let chunk = fs::read(dir.join(format!("heights{cx}-{cy}.dat")))
                    .ok()
                    .and_then(|z| inflate(&z))
                    .map(|plain| terrain_chunks::decode(&plain));
                let Some(c) = chunk else {
                    f.missing += 1;
                    continue;
                };
                for y in 0..CHUNK {
                    for x in 0..CHUNK {
                        f.heights[cy * CHUNK + y][cx * CHUNK + x] = c.heights[y][x];
                        f.materials[cy * CHUNK + y][cx * CHUNK + x] = c.materials[y][x];
                    }
                }
            }
        }
        Ok(f)
    }

    /// The 256 chunk files and `heights.info`, as `(name, bytes)`, in the
    /// client's own encoding (so its stamps match, see
    /// [`terrain_chunks::chunk_files`]). Holes are not kept.
    pub fn files(&self) -> Result<Vec<(String, Vec<u8>)>, String> {
        let mut out = terrain_chunks::chunk_files(&self.heights, &self.materials)?;
        if !self.names.is_empty() {
            out.push((
                "heights.info".into(),
                (self.names.join("\n") + "\n").into_bytes(),
            ));
        }
        Ok(out)
    }

    /// World x, y of square (i, j)'s corner.
    pub fn square_pos(&self, i: usize, j: usize) -> (f64, f64) {
        let s = f64::from(self.square);
        ((i as f64 - 128.0) * s, (j as f64 - 128.0) * s)
    }

    /// Height in units at square (i, j).
    pub fn height(&self, i: usize, j: usize) -> f64 {
        f64::from(self.heights[j][i]) / PER_UNIT
    }

    /// The ground's height under world (x, y), bilinear between squares;
    /// `None` off the terrain.
    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        let s = f64::from(self.square);
        let (u, v) = (x / s + 128.0, y / s + 128.0);
        if !(0.0..=255.0).contains(&u) || !(0.0..=255.0).contains(&v) {
            return None;
        }
        let (i, j) = ((u.floor() as usize).min(254), (v.floor() as usize).min(254));
        let (fu, fv) = (u - i as f64, v - j as f64);
        let a = self.height(i, j) * (1.0 - fu) + self.height(i + 1, j) * fu;
        let b = self.height(i, j + 1) * (1.0 - fu) + self.height(i + 1, j + 1) * fu;
        Some(a * (1.0 - fv) + b * fv)
    }

    /// Apply a brush around world (x, y). Returns how many squares changed.
    pub fn apply(&mut self, brush: Brush, x: f64, y: f64, radius: f64) -> usize {
        let s = f64::from(self.square);
        let (cu, cv) = (x / s + 128.0, y / s + 128.0);
        let r = (radius / s).max(0.5);
        let lo = |c: f64| ((c - r).floor().max(0.0)) as usize;
        let hi = |c: f64| ((c + r).ceil().min(255.0)) as usize;
        let before = self.heights.clone();
        let mut changed = 0;
        for j in lo(cv)..=hi(cv) {
            for i in lo(cu)..=hi(cu) {
                let d = (i as f64 - cu).hypot(j as f64 - cv) / r;
                if d > 1.0 {
                    continue;
                }
                // Smooth fall-off: 1 at the centre, 0 at the edge.
                let w = 0.5 + 0.5 * (std::f64::consts::PI * d).cos();
                let h0 = f64::from(before[j][i]);
                let h = match brush {
                    Brush::Raise(units) => h0 + units * PER_UNIT * w,
                    Brush::Flatten(units) => h0 + (units * PER_UNIT - h0) * w,
                    Brush::Smooth(strength) => {
                        let mut sum = 0.0;
                        let mut n = 0.0;
                        for (di, dj) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                            let (ni, nj) = (i as i32 + di, j as i32 + dj);
                            if (0..SIZE as i32).contains(&ni) && (0..SIZE as i32).contains(&nj) {
                                sum += f64::from(before[nj as usize][ni as usize]);
                                n += 1.0;
                            }
                        }
                        h0 + (sum / n - h0) * strength.clamp(0.0, 1.0) * w
                    }
                    Brush::Paint(m) => {
                        if self.materials[j][i] != m {
                            self.materials[j][i] = m;
                            changed += 1;
                        }
                        continue;
                    }
                };
                let h = h.round().clamp(0.0, 16383.0) as u16;
                if h != self.heights[j][i] {
                    self.heights[j][i] = h;
                    changed += 1;
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brushes_and_round_trip() {
        let mut f = Field::flat(8, "g3d_grass.png");
        f.names.push("g3d_rock.png".into());
        assert!(f.apply(Brush::Raise(10.0), 0.0, 0.0, 40.0) > 0);
        let top = f.height_at(0.0, 0.0).unwrap();
        assert!((top - 10.0).abs() < 0.2, "{top}");
        assert_eq!(f.height_at(100.0, 100.0), Some(0.0));
        assert_eq!(f.height_at(5000.0, 0.0), None);
        f.apply(Brush::Smooth(1.0), 0.0, 0.0, 16.0);
        assert!(f.height_at(0.0, 0.0).unwrap() < top);
        f.apply(Brush::Flatten(2.0), 0.0, 0.0, 8.0);
        assert!((f.height(128, 128) - 2.0).abs() < 0.2);
        assert_eq!(f.apply(Brush::Paint(1), 0.0, 0.0, 8.0), 5);

        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in f.files().unwrap() {
            fs::write(dir.path().join(name), bytes).unwrap();
        }
        let back = Field::load(dir.path(), 8).unwrap();
        assert_eq!(back.missing, 0);
        assert_eq!(back.names, f.names);
        assert_eq!(back.materials, f.materials);
        // The client's encoding clamps steep steps; this hill is gentle.
        assert_eq!(back.heights, f.heights);
    }

    #[test]
    fn missing_chunks_are_counted() {
        let dir = tempfile::tempdir().unwrap();
        let f = Field::load(dir.path(), 8).unwrap();
        assert_eq!(f.missing, 256);
        assert!(Field::load(&dir.path().join("nope"), 8).is_err());
    }
}
