//! World generators: the jobs of the classic TerrainGenerator, the NW
//! Dungeon Generator and GraalLevelGen, written from their behaviour notes
//! (Graal_analysis `tools/generators-222/README.md` and
//! `tools/classic-tools/graal-level-gen.md`), not from any original code.
//!
//! Each generator returns files (name and bytes): `GLEVNW01` levels with
//! edge links to their neighbours, and a `GRMAP001` gmap with a
//! `LEVELNAMES` grid, which is what clients and `seihai-core` read. None of
//! them reproduces an original program's output byte for byte; the
//! originals were not run.
//!
//! * [`terrain`]: a midpoint-displacement height field, one sample every 8
//!   tiles shared across level edges, stored as each level's 9 by 9
//!   `HEIGHTS` block. The board is tile 0. The gmap also carries the
//!   generator keys (`GENERATED`, `GENSEED`, ... `HEIGHTMAP`).
//! * [`from_image`]: a picture drawn in the twelve documented colours,
//!   sampled every 6 pixels; forest places the 6 by 5 tree stamp, other
//!   colours fill their 6 by 6 cell with one tile (the original's edge
//!   pieces are not described, so they are not drawn). A second run paints
//!   over existing levels and keeps their links and NPCs.
//! * [`dungeon`]: an `X` / `.` layout (or a maze from the seed), each cell
//!   8 tiles wide and 11 tall, cut into 64 by 64 levels.

use seihai_core::assets::{Level, Link};

use crate::level::encode;

/// A generated file.
pub type File = (String, Vec<u8>);

/// The Delphi random generator (`seed * 134775813 + 1`, 32 bits), the one
/// the original generators were built with.
#[derive(Debug, Clone)]
pub struct DelphiRandom(pub u32);

impl DelphiRandom {
    /// The next value in `[-1, 1)`.
    pub fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(134_775_813).wrapping_add(1);
        f64::from(self.0) / 4_294_967_296.0 * 2.0 - 1.0
    }

    /// The next value in `0..n` (`n` at least 1).
    pub fn below(&mut self, n: u32) -> u32 {
        self.0 = self.0.wrapping_mul(134_775_813).wrapping_add(1);
        ((u64::from(self.0) * u64::from(n.max(1))) >> 32) as u32
    }
}

/// Column letters: 0 `A`, 25 `Z`, 26 `AA`.
pub fn column_name(mut n: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'A' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

fn link(dest: &str, x: i32, y: i32, w: i32, h: i32, nx: Option<f32>, ny: Option<f32>) -> Link {
    Link {
        dest: dest.to_string(),
        x,
        y,
        width: w,
        height: h,
        new_x: nx,
        new_y: ny,
        x_offset: 0.0,
        y_offset: 0.0,
    }
}

/// One link on each side that has a neighbour. West arrives at x 61, east
/// at x 0, north at y 61, south at y 0; the other axis keeps the player's.
pub fn edge_links(names: &[String], width: usize, height: usize, x: usize, y: usize) -> Vec<Link> {
    let at = |cx: usize, cy: usize| names[cy * width + cx].as_str();
    let mut out = Vec::new();
    if x > 0 {
        out.push(link(at(x - 1, y), 0, 0, 1, 64, Some(61.0), None));
    }
    if x + 1 < width {
        out.push(link(at(x + 1, y), 63, 0, 1, 64, Some(0.0), None));
    }
    if y > 0 {
        out.push(link(at(x, y - 1), 0, 0, 64, 1, None, Some(61.0)));
    }
    if y + 1 < height {
        out.push(link(at(x, y + 1), 0, 63, 64, 1, None, Some(0.0)));
    }
    out
}

/// A level with a full 64 by 64 board on layer 0.
fn board_level(tiles: &[u16]) -> Level {
    Level::from_board(tiles)
}

/// The `LEVELNAMES` gmap for `names` (row by row), with extra key lines
/// before the grid.
fn gmap_text(width: usize, height: usize, names: &[String], keys: &[String]) -> Vec<u8> {
    let mut gmap = crate::worldmap::Gmap {
        width,
        height,
        names: names.to_vec(),
        ..Default::default()
    };
    gmap.load_at_start.clear();
    let text = crate::worldmap::write(&gmap);
    let mut lines: Vec<&str> = text.lines().collect();
    // Keys go after WIDTH and HEIGHT.
    let tail = lines.split_off(3);
    let mut out: Vec<String> = lines.into_iter().map(str::to_string).collect();
    out.extend(keys.iter().cloned());
    out.extend(tail.into_iter().map(str::to_string));
    let mut s = out.join("\n");
    s.push('\n');
    s.into_bytes()
}

// ---------------------------------------------------------------- terrain

/// TerrainGenerator's dialog fields.
#[derive(Debug, Clone)]
pub struct TerrainOptions {
    pub world: String,
    /// Levels across and down.
    pub width: usize,
    pub height: usize,
    pub seed: u32,
    pub base: i32,
    /// How far the first split may move a point.
    pub amplitude: f64,
    /// The factor on the move after each finer split (0 to 1 is usual).
    pub chaos: f64,
    /// Keep the outer ring at `base` (the Islands preset).
    pub even_borders: bool,
}

/// Samples per level side, minus the shared one: 9 by 9 per level.
const STEP: usize = 8;

/// Midpoint displacement (diamond and square steps) on the smallest
/// `2^k + 1` square that covers `w` by `h` points, then cut to size.
pub fn height_field(
    w: usize,
    h: usize,
    seed: u32,
    base: i32,
    amplitude: f64,
    chaos: f64,
    even_borders: bool,
) -> Vec<Vec<i32>> {
    let mut side = 2usize;
    while side + 1 < w.max(h) {
        side *= 2;
    }
    let n = side + 1;
    let mut rng = DelphiRandom(seed);
    let mut f = vec![vec![f64::from(base); n]; n];
    let rim = |x: usize, y: usize| x == 0 || y == 0 || x == n - 1 || y == n - 1;
    if !even_borders {
        for (x, y) in [(0, 0), (n - 1, 0), (0, n - 1), (n - 1, n - 1)] {
            f[y][x] = f64::from(base) + rng.unit() * amplitude;
        }
    }
    let mut step = side;
    let mut amp = amplitude;
    while step > 1 {
        let half = step / 2;
        // Diamond: the centre of each square.
        for y in (half..n).step_by(step) {
            for x in (half..n).step_by(step) {
                let avg = (f[y - half][x - half]
                    + f[y - half][x + half]
                    + f[y + half][x - half]
                    + f[y + half][x + half])
                    / 4.0;
                f[y][x] = avg + rng.unit() * amp;
            }
        }
        // Square: the edge midpoints.
        for y in (0..n).step_by(half) {
            let start = if (y / half).is_multiple_of(2) { half } else { 0 };
            for x in (start..n).step_by(step) {
                if even_borders && rim(x, y) {
                    f[y][x] = f64::from(base);
                    continue;
                }
                let mut sum = 0.0;
                let mut count = 0.0;
                if x >= half {
                    sum += f[y][x - half];
                    count += 1.0;
                }
                if x + half < n {
                    sum += f[y][x + half];
                    count += 1.0;
                }
                if y >= half {
                    sum += f[y - half][x];
                    count += 1.0;
                }
                if y + half < n {
                    sum += f[y + half][x];
                    count += 1.0;
                }
                f[y][x] = sum / count + rng.unit() * amp;
            }
        }
        amp *= chaos;
        step = half;
    }
    f.iter()
        .take(h)
        .map(|row| row.iter().take(w).map(|v| v.round() as i32).collect())
        .collect()
}

/// A terrain world: `world.gmap` and `width * height` levels named
/// `<world><column><row>.nw` (row from 00).
pub fn terrain(o: &TerrainOptions) -> Result<Vec<File>, String> {
    if o.width == 0 || o.height == 0 || o.width > 64 || o.height > 64 {
        return Err("terrain worlds are 1 to 64 levels a side".into());
    }
    let (pw, ph) = (o.width * STEP + 1, o.height * STEP + 1);
    let field = height_field(pw, ph, o.seed, o.base, o.amplitude, o.chaos, o.even_borders);
    let names: Vec<String> = (0..o.height)
        .flat_map(|y| (0..o.width).map(move |x| (x, y)))
        .map(|(x, y)| format!("{}{}{:02}.nw", o.world, column_name(x), y))
        .collect();
    let mut files = Vec::new();
    let blank = vec![0u16; 4096];
    for y in 0..o.height {
        for x in 0..o.width {
            let mut level = board_level(&blank);
            level.links = edge_links(&names, o.width, o.height, x, y);
            let mut bytes = encode(&level);
            bytes.extend_from_slice(b"HEIGHTS\n");
            for row in &field[y * STEP..=y * STEP + STEP] {
                let vals: Vec<String> = row[x * STEP..=x * STEP + STEP]
                    .iter()
                    .map(i32::to_string)
                    .collect();
                bytes.extend_from_slice(vals.join(" ").as_bytes());
                bytes.push(b'\n');
            }
            bytes.extend_from_slice(b"HEIGHTSEND\n");
            files.push((names[y * o.width + x].clone(), bytes));
        }
    }
    let num = |v: f64| {
        if v.fract() == 0.0 {
            format!("{}", v as i64)
        } else {
            v.to_string()
        }
    };
    let mut keys = vec![
        "GENERATED 1".to_string(),
        format!("GENSEED {}", o.seed),
        format!("GENBASE {}", o.base),
        format!("GENEVENBORDERS {}", u8::from(o.even_borders)),
        format!("GENHEIGHT {}", num(o.amplitude)),
        format!("GENCHAOS {}", num(o.chaos)),
        format!("LEVHEIGHT {}", num(o.amplitude)),
        format!("LEVCHAOS {}", num(o.chaos)),
        "HEIGHTMAP".to_string(),
    ];
    for row in &field {
        keys.push(row.iter().map(i32::to_string).collect::<Vec<_>>().join(" "));
    }
    keys.push("HEIGHTMAPEND".to_string());
    files.push((
        format!("{}.gmap", o.world),
        gmap_text(o.width, o.height, &names, &keys),
    ));
    Ok(files)
}

// ------------------------------------------------------------------ image

/// The twelve GraalLevelGen colours, in the image's order.
pub const COLOURS: [(&str, [u8; 3]); 12] = [
    ("grass", [24, 140, 24]),
    ("forest", [181, 239, 189]),
    ("water", [0, 0, 255]),
    ("mountains", [128, 0, 0]),
    ("flowers", [255, 0, 0]),
    ("sand", [150, 110, 0]),
    ("puddle", [0, 255, 255]),
    ("puddle stone", [0, 179, 179]),
    ("small sand stones", [113, 82, 0]),
    ("big sand stone", [83, 61, 0]),
    ("swamp", [0, 106, 0]),
    ("bushes", [0, 66, 0]),
];

/// The tree stamp forest places, 6 wide and 5 tall.
const FOREST: [[u16; 6]; 5] = [
    [25, 26, 27, 28, 29, 30],
    [41, 42, 43, 44, 45, 46],
    [57, 58, 59, 60, 61, 62],
    [73, 74, 75, 76, 77, 78],
    [89, 90, 91, 92, 93, 94],
];

/// The one tile each other colour fills its cell with (index 2 to 11).
const FILL: [u16; 12] = [0, 0, 12, 80, 4, 5, 6, 7, 8, 9, 10, 11];

/// The colour within 32 (sum of channel differences) of `rgb`, nearest
/// first.
pub fn match_colour(rgb: [u8; 3]) -> Option<usize> {
    COLOURS
        .iter()
        .enumerate()
        .map(|(i, (_, c))| {
            let d: i32 = (0..3)
                .map(|k| (i32::from(rgb[k]) - i32::from(c[k])).abs())
                .sum();
            (d, i)
        })
        .filter(|(d, _)| *d < 32)
        .min()
        .map(|(_, i)| i)
}

/// Level name GraalLevelGen writes: `world_a-01.nw` (column lower case,
/// row from 01).
pub fn image_level_name(world: &str, x: usize, y: usize) -> String {
    format!(
        "{world}_{}-{:02}.nw",
        column_name(x).to_ascii_lowercase(),
        y + 1
    )
}

/// Levels from an RGB picture (`rgb` is `width * height * 3`). `existing`
/// returns a level already on disk by name: its board is painted over and
/// its links and NPCs are kept.
pub fn from_image(
    world: &str,
    rgb: &[u8],
    width: u32,
    height: u32,
    mut existing: impl FnMut(&str) -> Option<Level>,
) -> Result<Vec<File>, String> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || rgb.len() != w * h * 3 {
        return Err("the picture is empty or its size does not match".into());
    }
    let (lx, ly) = (w.div_ceil(64), h.div_ceil(64));
    if lx > 64 || ly > 64 {
        return Err("the picture is over 64 levels a side".into());
    }
    let names: Vec<String> = (0..ly)
        .flat_map(|y| (0..lx).map(move |x| (x, y)))
        .map(|(x, y)| image_level_name(world, x, y))
        .collect();
    let mut old: Vec<Option<Level>> = names.iter().map(|n| existing(n)).collect();
    let mut boards: Vec<Vec<u16>> = old
        .iter()
        .map(|l| {
            l.as_ref()
                .map_or_else(|| vec![0; 4096], Level::layer0_board)
        })
        .collect();
    let mut put = |x: i64, y: i64, t: u16| {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        let (cx, cy) = (x / 64, y / 64);
        if cx < lx && cy < ly {
            boards[cy * lx + cx][(y % 64) * 64 + x % 64] = t;
        }
    };
    for cy in 0..h.div_ceil(6) {
        for cx in 0..w.div_ceil(6) {
            let (px, py) = (cx * 6, cy * 6);
            let p = (py * w + px) * 3;
            match match_colour([rgb[p], rgb[p + 1], rgb[p + 2]]) {
                None | Some(0) => {}
                Some(1) => {
                    let ox = px as i64 - if cy % 2 == 1 { 3 } else { 0 };
                    let oy = py as i64 + 1;
                    for (r, row) in FOREST.iter().enumerate() {
                        for (c, &t) in row.iter().enumerate() {
                            put(ox + c as i64, oy + r as i64, t);
                        }
                    }
                }
                Some(k) => {
                    for dy in 0..6 {
                        for dx in 0..6 {
                            put((px + dx) as i64, (py + dy) as i64, FILL[k]);
                        }
                    }
                }
            }
        }
    }
    let mut files = Vec::new();
    for y in 0..ly {
        for x in 0..lx {
            let i = y * lx + x;
            let mut level = board_level(&boards[i]);
            match old[i].take() {
                Some(prev) => {
                    level.links = prev.links;
                    level.npcs = prev.npcs;
                    level.signs = prev.signs;
                    level.chests = prev.chests;
                    level.baddies = prev.baddies;
                }
                None => level.links = edge_links(&names, lx, ly, x, y),
            }
            files.push((names[i].clone(), encode(&level)));
        }
    }
    files.push((format!("{world}.gmap"), gmap_text(lx, ly, &names, &[])));
    Ok(files)
}

// ---------------------------------------------------------------- dungeon

/// A dungeon cell is 8 tiles wide and 11 tall.
pub const CELL_W: usize = 8;
pub const CELL_H: usize = 11;

/// Read an `X` (wall) / `.` (path) layout, one row a line. Other
/// characters are walls; short rows are padded with walls.
pub fn parse_layout(text: &str) -> Result<Vec<Vec<bool>>, String> {
    let rows: Vec<&str> = text
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.trim().is_empty())
        .collect();
    let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    if w == 0 {
        return Err("the layout is empty".into());
    }
    Ok(rows
        .iter()
        .map(|r| {
            let mut v: Vec<bool> = r.chars().map(|c| c == '.').collect();
            v.resize(w, false);
            v
        })
        .collect())
}

/// A maze of `w` by `h` cells (odd sizes work best) from `seed`: walls
/// everywhere, then a depth-first carve from cell (1, 1).
pub fn maze_layout(w: usize, h: usize, seed: u32) -> Vec<Vec<bool>> {
    let mut grid = vec![vec![false; w]; h];
    if w < 3 || h < 3 {
        return grid;
    }
    let mut rng = DelphiRandom(seed);
    let mut stack = vec![(1usize, 1usize)];
    grid[1][1] = true;
    while let Some(&(x, y)) = stack.last() {
        let mut next = Vec::new();
        for (dx, dy) in [(2i64, 0i64), (-2, 0), (0, 2), (0, -2)] {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            if nx > 0
                && ny > 0
                && (nx as usize) < w - 1
                && (ny as usize) < h - 1
                && !grid[ny as usize][nx as usize]
            {
                next.push((nx as usize, ny as usize));
            }
        }
        if next.is_empty() {
            stack.pop();
            continue;
        }
        let (nx, ny) = next[rng.below(next.len() as u32) as usize];
        grid[(y + ny) / 2][(x + nx) / 2] = true;
        grid[ny][nx] = true;
        stack.push((nx, ny));
    }
    grid
}

/// Tiles for a dungeon: `path` fills open cells, `wall` the rest.
#[derive(Debug, Clone, Copy)]
pub struct DungeonTiles {
    pub path: u16,
    pub wall: u16,
}

/// Levels from a layout: each cell becomes an 8 by 11 block, the whole
/// picture is cut into 64 by 64 levels named `<start><column><row>.nw`
/// (row from 1, `cave_A1.nw`), with edge links and a gmap.
pub fn dungeon(
    start: &str,
    layout: &[Vec<bool>],
    tiles: DungeonTiles,
) -> Result<Vec<File>, String> {
    let ch = layout.len();
    let cw = layout.first().map_or(0, Vec::len);
    if cw == 0 || ch == 0 {
        return Err("the layout is empty".into());
    }
    let (tw, th) = (cw * CELL_W, ch * CELL_H);
    let (lx, ly) = (tw.div_ceil(64), th.div_ceil(64));
    if lx > 64 || ly > 64 {
        return Err("the dungeon is over 64 levels a side".into());
    }
    let names: Vec<String> = (0..ly)
        .flat_map(|y| (0..lx).map(move |x| (x, y)))
        .map(|(x, y)| format!("{start}{}{}.nw", column_name(x), y + 1))
        .collect();
    let mut files = Vec::new();
    for y in 0..ly {
        for x in 0..lx {
            let mut board = vec![tiles.wall; 4096];
            for ty in 0..64 {
                for tx in 0..64 {
                    let (gx, gy) = (x * 64 + tx, y * 64 + ty);
                    if gx < tw && gy < th && layout[gy / CELL_H][gx / CELL_W] {
                        board[ty * 64 + tx] = tiles.path;
                    }
                }
            }
            let mut level = board_level(&board);
            level.links = edge_links(&names, lx, ly, x, y);
            files.push((names[y * lx + x].clone(), encode(&level)));
        }
    }
    files.push((
        format!("{start}.gmap"),
        gmap_text(lx, ly, &names, &["GENERATED 1".into()]),
    ));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use seihai_core::gmap::Gmap;

    fn level(files: &[File], name: &str) -> Level {
        let (_, bytes) = files.iter().find(|(n, _)| n == name).expect(name);
        Level::from_bytes(bytes).unwrap()
    }

    fn gmap(files: &[File], name: &str) -> Gmap {
        let (_, bytes) = files.iter().find(|(n, _)| n == name).expect(name);
        Gmap::parse(&String::from_utf8_lossy(bytes)).unwrap()
    }

    #[test]
    fn delphi_random_and_columns() {
        let mut r = DelphiRandom(0);
        r.unit();
        assert_eq!(r.0, 1);
        r.unit();
        assert_eq!(r.0, 134_775_814);
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(27), "AB");
    }

    #[test]
    fn terrain_writes_linked_levels_with_shared_height_edges() {
        let o = TerrainOptions {
            world: "t".into(),
            width: 2,
            height: 2,
            seed: 7,
            base: 10,
            amplitude: 40.0,
            chaos: 0.5,
            even_borders: true,
        };
        let files = terrain(&o).unwrap();
        assert_eq!(files.len(), 5);
        let g = gmap(&files, "t.gmap");
        assert_eq!(g.names, vec!["tA00.nw", "tB00.nw", "tA01.nw", "tB01.nw"]);
        let a = level(&files, "tA00.nw");
        assert_eq!(a.links.len(), 2);
        assert_eq!(a.layer0_board(), vec![0; 4096]);
        let heights = |name: &str| -> Vec<Vec<i32>> {
            let text = String::from_utf8(files.iter().find(|(n, _)| n == name).unwrap().1.clone())
                .unwrap();
            let body = text.split("HEIGHTS\n").nth(1).unwrap();
            body.lines()
                .take(9)
                .map(|l| l.split(' ').map(|v| v.parse().unwrap()).collect())
                .collect()
        };
        let (ha, hb) = (heights("tA00.nw"), heights("tB00.nw"));
        for r in 0..9 {
            assert_eq!(ha[r][8], hb[r][0], "the shared column");
        }
        assert!(
            ha[0].iter().all(|&v| v == 10),
            "even borders keep the rim at base"
        );
        assert_eq!(terrain(&o).unwrap(), files, "the same seed repeats");
    }

    #[test]
    fn image_paints_forest_and_fills_and_keeps_old_links() {
        // 64 by 64: forest in cell (0,0), water in cell (2,0), grass elsewhere.
        let (w, h) = (64u32, 64u32);
        let mut rgb = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let c = match (x / 6, y / 6) {
                    (0, 0) => COLOURS[1].1,
                    (2, 0) => [2, 2, 250],
                    _ => COLOURS[0].1,
                };
                rgb.extend_from_slice(&c);
            }
        }
        let files = from_image("w", &rgb, w, h, |_| None).unwrap();
        let l = level(&files, "w_a-01.nw");
        assert_eq!(l.tile_id_at(0, 1), Some(25));
        assert_eq!(l.tile_id_at(5, 5), Some(94));
        assert_eq!(l.tile_id_at(12, 0), Some(12));
        assert_eq!(l.tile_id_at(17, 5), Some(12));
        assert_eq!(l.tile_id_at(30, 30), Some(0));
        assert!(l.links.is_empty());
        let mut prev = l.clone();
        prev.links
            .push(link("other.nw", 1, 1, 1, 1, Some(5.0), Some(5.0)));
        let again = from_image("w", &rgb, w, h, |_| Some(prev.clone())).unwrap();
        assert_eq!(level(&again, "w_a-01.nw").links.len(), 1);
        assert_eq!(match_colour([255, 255, 255]), None);
    }

    #[test]
    fn dungeon_from_a_layout_and_a_maze() {
        let layout = parse_layout("XXXXXXXXX\nX.......X\nXXXXXXXXX\n").unwrap();
        let files = dungeon("cave_", &layout, DungeonTiles { path: 0, wall: 1 }).unwrap();
        // 9 cells * 8 = 72 tiles wide (2 levels), 3 * 11 = 33 tall (1 level).
        let g = gmap(&files, "cave_.gmap");
        assert_eq!(g.names, vec!["cave_A1.nw", "cave_B1.nw"]);
        let a = level(&files, "cave_A1.nw");
        assert_eq!(a.tile_id_at(0, 0), Some(1));
        assert_eq!(a.tile_id_at(8, 11), Some(0));
        assert_eq!(a.tile_id_at(8, 22), Some(1));
        assert_eq!(a.links.len(), 1);
        let m = maze_layout(9, 9, 3);
        let open = m.iter().flatten().filter(|&&c| c).count();
        // A perfect maze on 4 by 4 rooms opens 16 rooms and 15 passages.
        assert_eq!(open, 31);
        assert_eq!(maze_layout(9, 9, 3), m);
    }
}
