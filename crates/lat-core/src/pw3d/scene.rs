//! A world's 3D scene, extrapolated from what its tiles mean
//! ([`super::meaning`]):
//!
//! * **Elevation.** A 2D level draws a raised area in three-quarter view:
//!   the cliff face shows below the higher ground. So the ground is cut
//!   into regions at cliff and stair tiles, every vertical run of cliff
//!   tiles says the region above it stands that much higher than the one
//!   below (0.75 a tile of face), and the heights are solved outward from
//!   the largest region. Cliff tiles take the height of the ground above
//!   them, so their south edge is the face; stairs sit half way.
//! * **Ground.** Every tile is a flat top at its height in its own colour,
//!   with sides down to lower neighbours; water and lava are sunk, holes
//!   deep, walls and castle walls raised, hedges and fences a tile high.
//!   Under trees, buildings and furniture the ground takes the level's
//!   grass colour.
//! * **Things.** Trees (a crowd of tree tiles is split into trees by
//!   size) get a trunk where the 2D trunk stands, at the bottom of the
//!   sprite, and a round crown over it as tall as the sprite; bushes and
//!   rocks are rounded lumps; furniture is a box. A building (house, door
//!   and the walls around them) is a walled block over its footprint with
//!   a gabled roof in the roof's colour, its walls as tall as the 2D front
//!   wall is deep, and its door on the south face where the 2D door is.

use super::convert::Ground;
use super::meaning::Kind;
use super::shape::{Colour, Shape};
use std::collections::{HashMap, VecDeque};

/// One unit of cliff face a tile of 2D face.
const FACE: f32 = 0.75;

/// The tiles of a world, as the scene needs them.
pub struct Tiles<'a> {
    pub w: usize,
    pub h: usize,
    /// `None` in a gmap's holes.
    pub ground: &'a [Option<Ground>],
    pub kind: &'a [Kind],
    pub colour: &'a [Colour],
    /// Each tile's colour without its grass pixels (house walls).
    pub solid: &'a [Colour],
}

impl Tiles<'_> {
    fn ok(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h)
            .then(|| y as usize * self.w + x as usize)
            .filter(|&i| self.ground[i].is_some())
    }
}

/// What came out of a world, for the report.
#[derive(Debug, Default, Clone)]
pub struct Counts {
    pub trees: usize,
    pub buildings: usize,
    pub doors: usize,
    pub shrubs: usize,
    pub rocks: usize,
    pub furniture: usize,
    pub regions: usize,
    pub raised_regions: usize,
    pub highest: f32,
}

pub struct Scene {
    /// Top height of each tile (meaningless in holes).
    pub top: Vec<f32>,
    /// Top colour of each tile.
    pub colour: Vec<Colour>,
    /// Where a walker stands on each tile, `None` where it cannot.
    pub walk: Vec<Option<f32>>,
    /// Things, in world tile coordinates (Torque x, -y), with the tile they
    /// stand on, so each goes into its level's shape.
    pub things: Vec<((i32, i32), Shape)>,
    pub counts: Counts,
}

fn avg(cs: impl Iterator<Item = Colour>) -> Option<Colour> {
    let mut s = [0u32; 3];
    let mut n = 0;
    for c in cs {
        for (t, v) in s.iter_mut().zip(c.0) {
            *t += u32::from(v);
        }
        n += 1;
    }
    (n > 0).then(|| Colour(s.map(|v| (v / n) as u8)))
}

/// The most common colour among `cs`.
fn mode(cs: impl Iterator<Item = Colour>) -> Option<Colour> {
    let mut m: HashMap<Colour, usize> = HashMap::new();
    for c in cs {
        *m.entry(c).or_default() += 1;
    }
    m.into_iter().max_by_key(|(c, n)| (*n, *c)).map(|(c, _)| c)
}

/// Groups of tiles of the kinds `want`, 8-connected.
fn groups(t: &Tiles, want: &[Kind]) -> Vec<Vec<(i32, i32)>> {
    let mut seen = vec![false; t.w * t.h];
    let mut out = Vec::new();
    for y in 0..t.h as i32 {
        for x in 0..t.w as i32 {
            let Some(i) = t.ok(x, y) else { continue };
            if seen[i] || !want.contains(&t.kind[i]) {
                continue;
            }
            let mut g = Vec::new();
            let mut q = VecDeque::from([(x, y)]);
            seen[i] = true;
            while let Some((cx, cy)) = q.pop_front() {
                g.push((cx, cy));
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if let Some(j) = t.ok(cx + dx, cy + dy) {
                            if !seen[j] && want.contains(&t.kind[j]) {
                                seen[j] = true;
                                q.push_back((cx + dx, cy + dy));
                            }
                        }
                    }
                }
            }
            out.push(g);
        }
    }
    out
}

fn bbox(g: &[(i32, i32)]) -> (i32, i32, i32, i32) {
    let x0 = g.iter().map(|p| p.0).min().unwrap();
    let x1 = g.iter().map(|p| p.0).max().unwrap();
    let y0 = g.iter().map(|p| p.1).min().unwrap();
    let y1 = g.iter().map(|p| p.1).max().unwrap();
    (x0, y0, x1, y1)
}

/// Split a crowd of tree tiles into about one tree per `per` tiles
/// (k-means on the tile positions, seeded far apart).
fn split(g: &[(i32, i32)], per: usize) -> Vec<Vec<(i32, i32)>> {
    let k = (g.len() / per).max(1).min(g.len());
    if k == 1 {
        return vec![g.to_vec()];
    }
    let pts: Vec<(f32, f32)> = g.iter().map(|&(x, y)| (x as f32, y as f32)).collect();
    let mut cs = vec![pts[0]];
    while cs.len() < k {
        let far = pts
            .iter()
            .max_by(|a, b| {
                let d = |p: &&(f32, f32)| {
                    cs.iter()
                        .map(|c| (p.0 - c.0).powi(2) + (p.1 - c.1).powi(2))
                        .fold(f32::MAX, f32::min)
                };
                d(a).total_cmp(&d(b))
            })
            .unwrap();
        cs.push(*far);
    }
    let mut owner = vec![0usize; pts.len()];
    for _ in 0..12 {
        for (i, p) in pts.iter().enumerate() {
            owner[i] = (0..k)
                .min_by(|&a, &b| {
                    let d = |c: usize| (p.0 - cs[c].0).powi(2) + (p.1 - cs[c].1).powi(2);
                    d(a).total_cmp(&d(b))
                })
                .unwrap();
        }
        for (c, centre) in cs.iter_mut().enumerate() {
            let mine: Vec<&(f32, f32)> = pts
                .iter()
                .zip(&owner)
                .filter(|(_, o)| **o == c)
                .map(|(p, _)| p)
                .collect();
            if !mine.is_empty() {
                let n = mine.len() as f32;
                *centre = (
                    mine.iter().map(|p| p.0).sum::<f32>() / n,
                    mine.iter().map(|p| p.1).sum::<f32>() / n,
                );
            }
        }
    }
    (0..k)
        .map(|c| {
            g.iter()
                .zip(&owner)
                .filter(|(_, o)| **o == c)
                .map(|(p, _)| *p)
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty())
        .collect()
}

/// Region heights from cliff runs.
fn elevation(t: &Tiles) -> (Vec<f32>, usize, usize) {
    let barrier = |k: Kind| matches!(k, Kind::Cliff | Kind::Stairs);
    let mut region = vec![usize::MAX; t.w * t.h];
    let mut sizes = Vec::new();
    for y in 0..t.h as i32 {
        for x in 0..t.w as i32 {
            let Some(i) = t.ok(x, y) else { continue };
            if region[i] != usize::MAX || barrier(t.kind[i]) {
                continue;
            }
            let r = sizes.len();
            let mut n = 0;
            let mut q = VecDeque::from([(x, y)]);
            region[i] = r;
            while let Some((cx, cy)) = q.pop_front() {
                n += 1;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if let Some(j) = t.ok(cx + dx, cy + dy) {
                        if region[j] == usize::MAX && !barrier(t.kind[j]) {
                            region[j] = r;
                            q.push_back((cx + dx, cy + dy));
                        }
                    }
                }
            }
            sizes.push(n);
        }
    }
    let nr = sizes.len();
    // e[a] - e[b] votes for each pair, from vertical cliff runs.
    let mut votes: HashMap<(usize, usize), Vec<f32>> = HashMap::new();
    for x in 0..t.w as i32 {
        let mut y = 0i32;
        while y < t.h as i32 {
            let is_cliff = |y: i32| t.ok(x, y).is_some_and(|i| t.kind[i] == Kind::Cliff);
            if !is_cliff(y) {
                y += 1;
                continue;
            }
            let y0 = y;
            while is_cliff(y) {
                y += 1;
            }
            let (above, below) = (t.ok(x, y0 - 1), t.ok(x, y));
            if let (Some(a), Some(b)) = (above, below) {
                let (ra, rb) = (region[a], region[b]);
                if ra != usize::MAX && rb != usize::MAX && ra != rb {
                    let d = (y - y0) as f32 * FACE;
                    if ra < rb {
                        votes.entry((ra, rb)).or_default().push(d);
                    } else {
                        votes.entry((rb, ra)).or_default().push(-d);
                    }
                }
            }
        }
    }
    let mut edges: HashMap<usize, Vec<(usize, f32)>> = HashMap::new();
    for ((a, b), mut v) in votes {
        v.sort_by(f32::total_cmp);
        let d = v[v.len() / 2];
        edges.entry(a).or_default().push((b, -d));
        edges.entry(b).or_default().push((a, d));
    }
    // Solve outward from the largest region, the biggest regions first.
    let mut e = vec![f32::NAN; nr];
    let mut order: Vec<usize> = (0..nr).collect();
    order.sort_by_key(|&r| std::cmp::Reverse(sizes[r]));
    for &root in &order {
        if !e[root].is_nan() {
            continue;
        }
        e[root] = 0.0;
        let mut q = VecDeque::from([root]);
        while let Some(r) = q.pop_front() {
            for &(n, d) in edges.get(&r).map(|v| v.as_slice()).unwrap_or(&[]) {
                if e[n].is_nan() {
                    e[n] = e[r] + d;
                    q.push_back(n);
                }
            }
        }
    }
    let low = e.iter().copied().fold(0.0f32, f32::min);
    let raised = e.iter().filter(|&&v| v - low > 0.01).count();
    let mut tile = vec![0.0f32; t.w * t.h];
    for i in 0..tile.len() {
        if region[i] != usize::MAX {
            tile[i] = e[region[i]] - low;
        }
    }
    // Cliffs: the ground above their run; stairs: half way.
    for x in 0..t.w as i32 {
        for y in 0..t.h as i32 {
            let Some(i) = t.ok(x, y) else { continue };
            if !barrier(t.kind[i]) {
                continue;
            }
            let mut up = None;
            let mut yy = y - 1;
            while let Some(j) = t.ok(x, yy) {
                if !barrier(t.kind[j]) {
                    up = Some(tile[j]);
                    break;
                }
                yy -= 1;
            }
            let mut down = None;
            let mut yy = y + 1;
            while let Some(j) = t.ok(x, yy) {
                if !barrier(t.kind[j]) {
                    down = Some(tile[j]);
                    break;
                }
                yy += 1;
            }
            let side = [(1, 0), (-1, 0)]
                .iter()
                .filter_map(|(dx, dy)| t.ok(x + dx, y + dy))
                .filter(|&j| !barrier(t.kind[j]))
                .map(|j| tile[j])
                .fold(None, |m: Option<f32>, v| Some(m.map_or(v, |m| m.max(v))));
            let hi = [up, down, side]
                .into_iter()
                .flatten()
                .fold(0.0f32, f32::max);
            tile[i] = if t.kind[i] == Kind::Stairs {
                match (up, down) {
                    (Some(a), Some(b)) => (a + b) / 2.0,
                    _ => hi,
                }
            } else {
                up.unwrap_or(hi)
            };
        }
    }
    (tile, nr, raised)
}

/// Build the scene of a world.
fn near_water(t: &Tiles, i: usize) -> bool {
    let (x, y) = ((i % t.w) as i32, (i / t.w) as i32);
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| {
            t.ok(x + dx, y + dy)
                .is_some_and(|j| t.kind[j] == Kind::Water)
        })
    })
}

pub fn build(t: &Tiles, wall: f32) -> Scene {
    let n = t.w * t.h;
    let (base, regions, raised) = elevation(t);
    let grass = mode(
        (0..n)
            .filter(|&i| t.ground[i].is_some() && t.kind[i] == Kind::Ground)
            .map(|i| t.colour[i]),
    )
    .unwrap_or(Colour([70, 150, 60]));
    let mut top = vec![0.0f32; n];
    let mut colour = t.colour.to_vec();
    let mut walk = vec![None; n];
    for i in 0..n {
        let Some(g) = t.ground[i] else { continue };
        let e = base[i];
        let k = t.kind[i];
        top[i] = match k {
            Kind::Water => e + Ground::Water.height(wall),
            Kind::Lava => e + Ground::Lava.height(wall),
            Kind::Hole => e - 1.0,
            Kind::Bridge => e + 0.1,
            Kind::Fence => e + 1.0,
            Kind::CastleWall => e + wall * 2.0,
            // A rim round a pond is a low edge, not a wall.
            Kind::Wall if near_water(t, i) => e + 0.3,
            Kind::Wall => {
                e + if g == Ground::LowWall {
                    wall * 0.5
                } else {
                    wall
                }
            }
            _ => e,
        };
        if matches!(
            k,
            Kind::Tree | Kind::Shrub | Kind::Rock | Kind::Building | Kind::Door | Kind::Furniture
        ) {
            colour[i] = grass;
        }
        if g.walkable() {
            walk[i] = Some(top[i]);
        }
    }
    let mut things = Vec::new();
    let mut counts = Counts {
        regions,
        raised_regions: raised,
        highest: base.iter().copied().fold(0.0, f32::max),
        ..Default::default()
    };
    let at = |x: i32, y: i32| t.ok(x, y).map_or(0.0, |i| base[i]);
    let colour_of = |g: &[(i32, i32)]| {
        avg(g
            .iter()
            .filter_map(|&(x, y)| t.ok(x, y))
            .map(|i| t.colour[i]))
    };

    for g in groups(t, &[Kind::Tree]) {
        for tree in split(&g, 45) {
            let (x0, y0, x1, y1) = bbox(&tree);
            let (w, h) = ((x1 - x0 + 1) as f32, (y1 - y0 + 1) as f32);
            if tree.len() < 3 {
                continue;
            }
            let cx = (x0 + x1 + 1) as f32 / 2.0;
            // The trunk stands at the bottom of the 2D sprite.
            let ty = y1 as f32 + 0.5 - (h * 0.15).min(1.5);
            let e = at(cx as i32, ty as i32);
            let height = (h * 0.9).clamp(2.0, 9.0);
            let r = (w.max(h * 0.8) / 2.0).clamp(0.8, 4.5);
            let crown = colour_of(&tree).unwrap_or(Colour([40, 120, 40]));
            let trunk = Colour([95, 65, 35]);
            let mut s = Shape::default();
            let tw = (r * 0.18).clamp(0.2, 0.6);
            s.block(
                [cx - tw, -ty - tw, e],
                [cx + tw, -ty + tw, e + height * 0.55],
                trunk.shade(1.1),
                trunk,
            );
            s.blob(
                [cx, -ty, e + height - r * 0.75],
                [r, r, r * 0.75],
                crown,
                crown.shade(0.7),
            );
            things.push(((cx as i32, ty as i32), s));
            counts.trees += 1;
        }
    }
    for kind in [Kind::Shrub, Kind::Rock] {
        for g in groups(t, &[kind]) {
            let (x0, y0, x1, y1) = bbox(&g);
            let (w, h) = ((x1 - x0 + 1) as f32, (y1 - y0 + 1) as f32);
            let (cx, cy) = ((x0 + x1 + 1) as f32 / 2.0, (y0 + y1 + 1) as f32 / 2.0);
            let e = at(cx as i32, cy as i32);
            let c = colour_of(&g).unwrap_or(Colour([120, 120, 120]));
            let rz = if kind == Kind::Shrub {
                0.5
            } else {
                (w.min(h) * 0.3).clamp(0.3, 1.6)
            };
            let mut s = Shape::default();
            s.blob(
                [cx, -cy, e + rz * 0.8],
                [w / 2.0 * 0.95, h / 2.0 * 0.95, rz],
                c,
                c.shade(0.75),
            );
            things.push(((cx as i32, cy as i32), s));
            if kind == Kind::Shrub {
                counts.shrubs += 1;
            } else {
                counts.rocks += 1;
            }
        }
    }
    for g in groups(t, &[Kind::Furniture]) {
        let (x0, y0, x1, y1) = bbox(&g);
        let e = at(x0, y1);
        let c = colour_of(&g).unwrap_or(Colour([150, 110, 70]));
        let mut s = Shape::default();
        s.block(
            [x0 as f32 + 0.1, -(y1 as f32) - 0.9, e],
            [x1 as f32 + 0.9, -(y0 as f32) - 0.1, e + 0.8],
            c,
            c.shade(0.7),
        );
        things.push(((x0, y1), s));
        counts.furniture += 1;
    }
    // Buildings: house and door tiles, with the unnamed walls touching them.
    let mut kinds2 = t.kind.to_vec();
    for g in groups(t, &[Kind::Building, Kind::Door]) {
        let mut q: VecDeque<(i32, i32)> = g.iter().copied().collect();
        while let Some((x, y)) = q.pop_front() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                if let Some(j) = t.ok(x + dx, y + dy) {
                    if kinds2[j] == Kind::Wall {
                        kinds2[j] = Kind::Building;
                        q.push_back((x + dx, y + dy));
                    }
                }
            }
        }
    }
    let t2 = Tiles {
        w: t.w,
        h: t.h,
        ground: t.ground,
        kind: &kinds2,
        colour: t.colour,
        solid: t.solid,
    };
    for g in groups(&t2, &[Kind::Building, Kind::Door]) {
        if g.len() < 6 {
            continue;
        }
        let (x0, y0, x1, y1) = bbox(&g);
        let mut doors: Vec<(i32, i32)> = g
            .iter()
            .copied()
            .filter(|&(x, y)| t2.ok(x, y).is_some_and(|i| kinds2[i] == Kind::Door))
            .collect();
        // A doorstep: the bottom row standing out from a wider front. It
        // is the porch, not the house.
        let step: Vec<(i32, i32)> = g.iter().copied().filter(|p| p.1 == y1).collect();
        let porch = !step.is_empty() && (step.len() as i32) * 2 <= x1 - x0 + 1;
        if doors.is_empty() && porch {
            doors = step;
        }
        let by1 = if porch { y1 - 1 } else { y1 };
        let depth = by1 - y0 + 1;
        let front = doors
            .iter()
            .map(|d| d.1)
            .min()
            .map_or(3, |dy| by1 - dy + 1)
            .clamp(2, 4)
            .min(depth);
        // Ground under the building, and its walls and door.
        for &(x, y) in &g {
            if let Some(i) = t.ok(x, y) {
                top[i] = base[i];
                colour[i] = grass;
            }
        }
        let e = (y0..=by1)
            .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
            .filter_map(|(x, y)| t.ok(x, y))
            .map(|i| base[i])
            .fold(f32::MAX, f32::min);
        let e = if e == f32::MAX { 0.0 } else { e };
        // The front wall's colour: the house's own tiles in the front rows,
        // without the grass drawn round them.
        let greenish = |c: &Colour| {
            let [r, g, b] = c.0.map(i32::from);
            g > r + 12 && g > b + 12
        };
        let in_front = |p: &&(i32, i32)| p.1 > by1 - front && p.1 <= by1;
        let wall_c = avg(g
            .iter()
            .filter(in_front)
            .filter_map(|&(x, y)| t.ok(x, y))
            .map(|i| t.solid[i])
            .filter(|c| !greenish(c)))
        .unwrap_or(Colour([150, 110, 75]));
        let roof_c = avg(g
            .iter()
            .filter(|p| p.1 <= by1 - front)
            .filter_map(|&(x, y)| t.ok(x, y))
            .map(|i| t.colour[i]))
        .unwrap_or(wall_c.shade(0.6));
        let hgt = front as f32 * 0.9 + 0.4;
        let (fx0, fx1) = (x0 as f32, (x1 + 1) as f32);
        let (fy0, fy1) = (-((by1 + 1) as f32), -(y0 as f32));
        let mut s = Shape::default();
        s.block([fx0, fy0, e], [fx1, fy1, e + hgt], roof_c, wall_c);
        s.gable(
            [fx0 - 0.25, fy0 - 0.25, e + hgt],
            [fx1 + 0.25, fy1 + 0.25],
            roof_c,
            roof_c.shade(0.8),
        );
        if !doors.is_empty() {
            let dx0 = doors.iter().map(|d| d.0).min().unwrap() as f32;
            let dx1 = (doors.iter().map(|d| d.0).max().unwrap() + 1) as f32;
            let dh = (hgt - 0.3).min(2.0);
            let y = fy0 - 0.03;
            s.quad(
                [[dx0, y, e], [dx1, y, e], [dx1, y, e + dh], [dx0, y, e + dh]],
                Colour([55, 35, 20]),
            );
            counts.doors += 1;
        }
        things.push((((x0 + x1) / 2, y1), s));
        counts.buildings += 1;
    }
    Scene {
        top,
        colour,
        walk,
        things,
        counts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `w` x `h` world of floor and grass, with `f` setting kinds.
    fn world(
        w: usize,
        h: usize,
        f: impl Fn(usize, usize) -> Kind,
    ) -> (Vec<Option<Ground>>, Vec<Kind>, Vec<Colour>) {
        let kind: Vec<Kind> = (0..w * h).map(|i| f(i % w, i / w)).collect();
        let ground = kind
            .iter()
            .map(|k| {
                Some(match k {
                    Kind::Cliff | Kind::Building | Kind::Wall | Kind::Tree => Ground::Wall,
                    _ => Ground::Floor,
                })
            })
            .collect();
        (ground, kind, vec![Colour([70, 150, 60]); w * h])
    }

    #[test]
    fn a_cliff_raises_the_ground_above_it() {
        // Rows 8 to 11 are a cliff face: the ground north of it is higher.
        let (g, k, c) = world(20, 24, |_, y| {
            if (8..12).contains(&y) {
                Kind::Cliff
            } else {
                Kind::Ground
            }
        });
        let s = build(
            &Tiles {
                w: 20,
                h: 24,
                ground: &g,
                kind: &k,
                colour: &c,
                solid: &c,
            },
            1.5,
        );
        let (low, high) = (s.top[20 * 20 + 5], s.top[2 * 20 + 5]);
        assert_eq!(low, 0.0);
        assert!((high - 4.0 * FACE).abs() < 1e-4, "{high}");
        assert_eq!(s.walk[2 * 20 + 5], Some(high));
        assert_eq!(s.counts.raised_regions, 1);
    }

    #[test]
    fn house_tiles_become_a_house_with_a_door() {
        // A 7 x 5 house with a 2-tile doorstep below the middle of its front.
        let (g, k, c) = world(20, 20, |x, y| {
            let body = (4..11).contains(&x) && (4..9).contains(&y);
            let step = (6..8).contains(&x) && y == 9;
            if body || step {
                Kind::Building
            } else {
                Kind::Ground
            }
        });
        let s = build(
            &Tiles {
                w: 20,
                h: 20,
                ground: &g,
                kind: &k,
                colour: &c,
                solid: &c,
            },
            1.5,
        );
        assert_eq!((s.counts.buildings, s.counts.doors), (1, 1));
        // The ground under it stays where it was; the house is a thing.
        assert_eq!(s.top[5 * 20 + 5], 0.0);
        // Its walls are not the grass drawn round the house, and the
        // doorstep is the porch, outside its south wall.
        assert!(s.things[0].1.colours().contains(&Colour([150, 110, 75])));
        let south = s.things[0]
            .1
            .points
            .iter()
            .map(|p| p[1])
            .fold(0.0, f32::min);
        assert!(south >= -9.0 - 0.26, "{south}");
        let roof = s.things[0]
            .1
            .points
            .iter()
            .map(|p| p[2])
            .fold(0.0, f32::max);
        assert!(roof > 3.0, "{roof}");
    }

    #[test]
    fn trees_stand_where_tree_tiles_are() {
        // Two 4 x 4 crowns apart, and a wide wood that splits into several.
        let (g, k, c) = world(40, 20, |x, y| {
            let a = (2..6).contains(&x) && (2..6).contains(&y);
            let b = (10..14).contains(&x) && (2..6).contains(&y);
            let wood = (2..38).contains(&x) && (10..18).contains(&y);
            if a || b || wood {
                Kind::Tree
            } else {
                Kind::Ground
            }
        });
        let s = build(
            &Tiles {
                w: 40,
                h: 20,
                ground: &g,
                kind: &k,
                colour: &c,
                solid: &c,
            },
            1.5,
        );
        assert!(s.counts.trees >= 2 + 288 / 45, "{}", s.counts.trees);
        // The ground under the trees is grass, not wall.
        assert_eq!(s.top[3 * 40 + 3], 0.0);
    }
}
