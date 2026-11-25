//! Streamed Graal 3D terrain: the chunk files a 4.20r6 or 5.x client takes
//! (Graal_analysis `torque/terrain-512.md`, a behaviour read of the real 5.12
//! client).
//!
//! The terrain is 256 by 256 squares in 16 by 16 chunks. Each chunk is a
//! small bit stream: a mode, the height of its first square, 256 signed
//! deltas against a two-level predictor, the materials it uses and, per
//! square, which one, then hole flags. The server sends it zlib-compressed in
//! a `TerrainChunkEvent`, and the client keeps the same bytes as
//! `heights<x>-<y>.dat` in its cache. seihai-gserver-v2 serves a folder of
//! these files with its `terrain <folder>` world line.
//!
//! The client re-encodes every chunk it receives and stamps that encoding
//! (live, 2026-11-25): mode 2, the material ids it uses in ascending order,
//! deltas from its own open-loop writer. [`chunk_files`] therefore writes
//! each chunk in exactly that form, so the stamp a server takes over the
//! file's bytes is the one the client sends back, and a returning client is
//! not sent chunks it already has. It first encodes closed-loop (each delta
//! against the heights the client will have decoded so far, so a clamped
//! step does not carry its error across the chunk), then re-encodes the
//! decoded result the client's way until that no longer changes it.

#![allow(clippy::needless_range_loop)] // grid maths reads clearer with indices

use std::io::Write;

/// Squares a side of the whole terrain and of one chunk.
pub const SIZE: usize = 256;
pub const CHUNK: usize = 16;

/// One chunk's decoded contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Wire heights (the client stores each shifted up 2 bits), `h[y][x]`.
    pub heights: [[u16; CHUNK]; CHUNK],
    /// Material ids (indices into heights.info) per square, 0 to 31.
    pub materials: [[u8; CHUNK]; CHUNK],
    pub holes: [[bool; CHUNK]; CHUNK],
}

struct Bits {
    out: Vec<u8>,
    n: usize,
}

impl Bits {
    fn new() -> Self {
        Bits {
            out: Vec::new(),
            n: 0,
        }
    }
    fn int(&mut self, v: u32, bits: u32) {
        for i in 0..bits {
            if self.n.is_multiple_of(8) {
                self.out.push(0);
            }
            if (v >> i) & 1 == 1 {
                *self.out.last_mut().unwrap() |= 1 << (self.n % 8);
            }
            self.n += 1;
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    n: usize,
}

impl Reader<'_> {
    fn int(&mut self, bits: u32) -> u32 {
        let mut v = 0;
        for i in 0..bits {
            if self
                .data
                .get(self.n / 8)
                .is_some_and(|b| (b >> (self.n % 8)) & 1 == 1)
            {
                v |= 1 << i;
            }
            self.n += 1;
        }
        v
    }
}

/// Bits for an index among `n` choices: `n` rounded up to a power of two,
/// then its base-2 logarithm.
fn index_bits(n: usize) -> u32 {
    n.next_power_of_two().trailing_zeros()
}

/// Truncating division by 2, as C's `/`.
fn half(v: i32) -> i32 {
    v / 2
}

/// The chunk's data, before zlib (mode 2: hole flags at the end).
pub fn encode(c: &Chunk) -> Vec<u8> {
    let mut w = Bits::new();
    w.int(2, 8);
    w.int(u32::from(c.heights[0][0]), 16);

    // Closed loop: dec holds what the client will have decoded, first the
    // first-pass deltas (dp), then the heights (h).
    let mut h = [[0i32; CHUNK]; CHUNK];
    let mut dp = [[0i32; CHUNK]; CHUNK];
    let mut wire = [[0i32; CHUNK]; CHUNK];
    h[0][0] = i32::from(c.heights[0][0]);
    for y in 0..CHUNK {
        for x in 0..CHUNK {
            if x == 0 && y == 0 {
                continue;
            }
            let pred = if y == 0 {
                h[0][x - 1]
            } else if x == 0 {
                h[y - 1][0]
            } else {
                (h[y - 1][x] + h[y][x - 1]) / 2
            };
            let want = (i32::from(c.heights[y][x]) - pred).clamp(-127, 127);
            let d = if x == 0 || y == 0 {
                want
            } else {
                let mean = half(dp[y - 1][x] + dp[y][x - 1]);
                (want - mean).clamp((-128 - mean).max(-128), (127 - mean).min(127))
            };
            wire[y][x] = d;
            dp[y][x] = if x == 0 || y == 0 {
                d
            } else {
                d + half(dp[y - 1][x] + dp[y][x - 1])
            };
            h[y][x] = (pred + dp[y][x]) & 0xFFFF;
        }
    }
    for row in &wire {
        for &d in row {
            w.int((d as u32) & 0xFF, 8);
        }
    }

    let ids = used_ids(c);
    w.int(ids.len() as u32, 6);
    for &m in &ids {
        w.int(u32::from(m & 31), 5);
    }
    if ids.len() > 1 {
        let bits = index_bits(ids.len());
        for row in &c.materials {
            for &m in row {
                w.int(0, 1); // no blend
                let i = ids.iter().position(|&x| x == m).unwrap();
                w.int(i as u32, bits);
            }
        }
    }
    for row in &c.holes {
        for &hole in row {
            w.int(u32::from(hole), 1);
        }
    }
    w.out
}

/// The material ids a chunk uses, ascending (as the client writes them).
fn used_ids(c: &Chunk) -> Vec<u8> {
    let mut ids: Vec<u8> = c.materials.iter().flatten().map(|m| m & 31).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// The chunk's data as the client's own writer makes it: first-pass deltas
/// against the true neighbours, each clamped, then a second pass from the
/// last square back to (1, 1) against the mean of the first-pass deltas above
/// and to the left.
pub fn encode_canonical(c: &Chunk) -> Vec<u8> {
    let h = |x: usize, y: usize| i32::from(c.heights[y][x]);
    let mut d = [[0i32; CHUNK]; CHUNK];
    for x in 1..CHUNK {
        d[0][x] = (h(x, 0) - h(x - 1, 0)).clamp(-127, 127);
    }
    for y in 1..CHUNK {
        d[y][0] = (h(0, y) - h(0, y - 1)).clamp(-127, 127);
        for x in 1..CHUNK {
            d[y][x] = (h(x, y) - (h(x, y - 1) + h(x - 1, y)) / 2).clamp(-127, 127);
        }
    }
    for y in (1..CHUNK).rev() {
        for x in (1..CHUNK).rev() {
            d[y][x] = (d[y][x] - half(d[y - 1][x] + d[y][x - 1])).clamp(-127, 127);
        }
    }
    let mut w = Bits::new();
    w.int(2, 8);
    w.int(u32::from(c.heights[0][0]), 16);
    for row in &d {
        for &v in row {
            w.int((v as u32) & 0xFF, 8);
        }
    }
    let ids = used_ids(c);
    w.int(ids.len() as u32, 6);
    for &m in &ids {
        w.int(u32::from(m), 5);
    }
    if ids.len() > 1 {
        let bits = index_bits(ids.len());
        for row in &c.materials {
            for &m in row {
                w.int(0, 1);
                w.int(ids.iter().position(|&x| x == m & 31).unwrap() as u32, bits);
            }
        }
    }
    for row in &c.holes {
        for &hole in row {
            w.int(u32::from(hole), 1);
        }
    }
    w.out
}

/// Chunk data the client re-encodes to the same bytes: the closed-loop
/// encoding, then the client's own encoding of what it decodes to, repeated
/// until it settles.
pub fn encode_settled(c: &Chunk) -> Vec<u8> {
    let mut plain = encode(c);
    for _ in 0..16 {
        let again = encode_canonical(&decode(&plain));
        if again == plain {
            break;
        }
        plain = again;
    }
    plain
}

/// Decodes chunk data as the client does (blends are read and dropped).
pub fn decode(data: &[u8]) -> Chunk {
    let mut r = Reader { data, n: 0 };
    let mode = r.int(8);
    let mut c = Chunk {
        heights: [[0; CHUNK]; CHUNK],
        materials: [[0; CHUNK]; CHUNK],
        holes: [[false; CHUNK]; CHUNK],
    };
    let base = r.int(16) as i32;
    let mut d = [[0i32; CHUNK]; CHUNK];
    for row in d.iter_mut() {
        for v in row.iter_mut() {
            *v = r.int(8) as u8 as i8 as i32;
        }
    }
    let mut h = [[0i32; CHUNK]; CHUNK];
    h[0][0] = base;
    for x in 1..CHUNK {
        h[0][x] = (h[0][x - 1] + d[0][x]) & 0xFFFF;
    }
    for y in 1..CHUNK {
        h[y][0] = (h[y - 1][0] + d[y][0]) & 0xFFFF;
        for x in 1..CHUNK {
            d[y][x] = (d[y][x] + half(d[y - 1][x] + d[y][x - 1])) as u8 as i8 as i32;
            h[y][x] = ((h[y - 1][x] + h[y][x - 1]) / 2 + d[y][x]) & 0xFFFF;
        }
    }
    for y in 0..CHUNK {
        for x in 0..CHUNK {
            c.heights[y][x] = h[y][x] as u16;
        }
    }
    let n = r.int(6) as usize;
    let ids: Vec<u8> = (0..n).map(|_| r.int(5) as u8).collect();
    if n > 1 {
        let bits = index_bits(n);
        for y in 0..CHUNK {
            for x in 0..CHUNK {
                let blend = r.int(1) == 1;
                let p = r.int(bits) as usize;
                if blend {
                    r.int(bits);
                    r.int(4);
                }
                c.materials[y][x] = ids.get(p).copied().unwrap_or(0);
            }
        }
    } else if let Some(&id) = ids.first() {
        c.materials = [[id; CHUNK]; CHUNK];
    }
    if mode == 2 {
        for y in 0..CHUNK {
            for x in 0..CHUNK {
                c.holes[y][x] = r.int(1) == 1;
            }
        }
    }
    c
}

/// zlib bytes (with the header), as `heights<x>-<y>.dat` and the event hold.
pub fn compress(plain: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    z.write_all(plain).expect("writing to memory");
    z.finish().expect("writing to memory")
}

/// Torque's calculateCRC (CRC-32, start 0xFFFFFFFF, no final inversion): the
/// stamp the client sends back for each chunk it holds.
pub fn stamp(plain: &[u8]) -> u32 {
    crate::code_cache::crc32(plain) ^ 0xFFFF_FFFF
}

/// The 256 chunk files for a whole terrain: `heights[y][x]` and
/// `materials[y][x]` are 256 by 256 (y is the terrain's +y). Each file is
/// `(name, zlib bytes)`; an error if a chunk compresses past the event's
/// 1023 bytes.
pub fn chunk_files(
    heights: &[Vec<u16>],
    materials: &[Vec<u8>],
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut files = Vec::new();
    for cy in 0..CHUNK {
        for cx in 0..CHUNK {
            let mut c = Chunk {
                heights: [[0; CHUNK]; CHUNK],
                materials: [[0; CHUNK]; CHUNK],
                holes: [[false; CHUNK]; CHUNK],
            };
            for y in 0..CHUNK {
                for x in 0..CHUNK {
                    c.heights[y][x] = heights[cy * CHUNK + y][cx * CHUNK + x];
                    c.materials[y][x] = materials[cy * CHUNK + y][cx * CHUNK + x];
                }
            }
            let z = compress(&encode_settled(&c));
            if z.len() > 1023 {
                return Err(format!(
                    "terrain chunk ({cx}, {cy}) compresses to {} bytes; an event holds 1023",
                    z.len()
                ));
            }
            files.push((format!("heights{cx}-{cy}.dat"), z));
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(h: u16, m: u8) -> Chunk {
        Chunk {
            heights: [[h; CHUNK]; CHUNK],
            materials: [[m; CHUNK]; CHUNK],
            holes: [[false; CHUNK]; CHUNK],
        }
    }

    #[test]
    fn a_flat_chunk_is_small_and_decodes_exactly() {
        let c = flat(800, 2);
        let plain = encode(&c);
        // 8 + 16 + 256 * 8 + 6 + 5 + 256 bits.
        assert_eq!(plain.len(), (8 + 16 + 2048 + 6 + 5 + 256usize).div_ceil(8));
        assert_eq!(decode(&plain), c);
        assert!(compress(&plain).len() < 40);
    }

    #[test]
    fn hills_and_materials_round_trip() {
        let mut c = flat(0, 0);
        for y in 0..CHUNK {
            for x in 0..CHUNK {
                let v = 600.0 + 200.0 * ((x as f64) / 3.0).sin() * ((y as f64) / 4.0).cos();
                c.heights[y][x] = v as u16;
                c.materials[y][x] = [1, 4, 9][(x + y) % 3];
            }
        }
        c.holes[3][5] = true;
        assert_eq!(decode(&encode(&c)), c);
    }

    #[test]
    fn a_cliff_too_steep_for_one_delta_is_caught_up_without_drift() {
        let mut c = flat(100, 0);
        for y in 0..CHUNK {
            for x in 8..CHUNK {
                c.heights[y][x] = 1100;
            }
        }
        let d = decode(&encode(&c));
        // A step of 1000 takes several squares, but the far side ends exact.
        for y in 0..CHUNK {
            assert_eq!(d.heights[y][0], 100);
            assert_eq!(d.heights[y][CHUNK - 1], 1100, "row {y}");
        }
    }

    #[test]
    fn written_chunks_are_what_the_client_re_encodes() {
        let mut c = flat(0, 0);
        for y in 0..CHUNK {
            for x in 0..CHUNK {
                let v = 900.0 + 400.0 * ((x as f64) / 2.0).sin() * ((y as f64) / 3.0).cos();
                c.heights[y][x] = v as u16;
                c.materials[y][x] = [7, 2, 5][(x * y) % 3];
            }
        }
        c.heights[9][9] = 3000; // a spike no delta reaches in one square
        let plain = encode_settled(&c);
        assert_eq!(encode_canonical(&decode(&plain)), plain);
        // Ids ascending.
        assert_eq!(decode(&plain).materials[0][0], 7);
        let mut r = Reader {
            data: &plain,
            n: 8 + 16 + 2048,
        };
        assert_eq!((r.int(6), r.int(5), r.int(5), r.int(5)), (3, 2, 5, 7));
    }

    #[test]
    fn stamp_is_crc32_without_the_final_inversion() {
        // CRC-32 of "123456789" is 0xCBF43926.
        assert_eq!(stamp(b"123456789"), 0xCBF4_3926 ^ 0xFFFF_FFFF);
    }

    #[test]
    fn a_whole_terrain_is_256_named_files() {
        let heights = vec![vec![500u16; SIZE]; SIZE];
        let materials = vec![vec![0u8; SIZE]; SIZE];
        let files = chunk_files(&heights, &materials).unwrap();
        assert_eq!(files.len(), 256);
        assert_eq!(files[0].0, "heights0-0.dat");
        assert_eq!(files[17].0, "heights1-1.dat");
        assert_eq!(files[255].0, "heights15-15.dat");
    }
}
