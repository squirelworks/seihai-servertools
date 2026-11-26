//! A 2D playerworld as 3D worlds (see the crate notes in `lib.rs`).

use super::meaning::{read_object_sets, Kind, Meanings};
use super::npc::{npc_shape, Images};
use super::scene;
use super::shape::{Colour, Shape};
use super::walker;
use seihai_core::assets::{read_png_rgba, Level, TileTypes, EMPTY_UPPER_TILE};
use seihai_core::gmap::Gmap;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// One output file: its path under the output folder, and its bytes.
pub type File = (String, Vec<u8>);

/// What to convert and how.
#[derive(Debug, Clone)]
pub struct Options {
    /// The playerworld folder: levels (`.nw`, `.graal`, `.zelda`) and gmaps,
    /// searched recursively.
    pub input: PathBuf,
    /// The level the default world is made from (a file name such as
    /// `start.nw`). Without it: `onlinestartlocal`, `start`, else the first
    /// level by name.
    pub start: Option<String>,
    /// Where the player stands in the start level, in tiles.
    pub start_pos: Option<(f32, f32)>,
    /// A `pics1.png`-layout tileset to take the tile colours from. Without
    /// it, Seihai's own built-in tileset.
    pub tileset: Option<PathBuf>,
    /// A tiledefs file (text or `arrays.dat`) for walls and water. Without
    /// it, Seihai's built-in table of the standard tile layout.
    pub tiledefs: Option<PathBuf>,
    /// The client's object sets (a folder with `objects*.txt` GOBJSET01
    /// files or a `tileobjects/` folder, or an `objects.dat`): they say what
    /// the tiles are, so trees, houses, cliffs and hedges come out as such.
    pub objects: Option<PathBuf>,
    /// Also write a picture of each level's tile meanings
    /// (`PW3D-MAPS/<level>.png`).
    pub meaning_maps: bool,
    /// More folders to look for NPC pictures in (`.png`, `.gif`, `.mng`),
    /// after the playerworld folder itself.
    pub images: Vec<PathBuf>,
    /// How tall a wall tile stands, in tiles.
    pub wall_height: f32,
    /// Bits a colour channel keeps (1 to 8): fewer bits, fewer materials.
    pub colour_bits: u8,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            input: PathBuf::new(),
            start: None,
            start_pos: None,
            tileset: None,
            tiledefs: None,
            objects: None,
            meaning_maps: false,
            images: Vec::new(),
            wall_height: 1.5,
            colour_bits: 3,
        }
    }
}

/// How a tile stands in 3D.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    Floor,
    Water,
    Lava,
    /// A wall thrown things fly over (bushes, low walls): half height.
    LowWall,
    Wall,
}

impl Ground {
    fn from_type(raw: Option<u8>) -> Ground {
        match raw {
            None => Ground::Wall,
            Some(20) => Ground::LowWall,
            Some(t) if t >= 20 => Ground::Wall,
            Some(11) => Ground::Water,
            Some(12 | 13) => Ground::Lava,
            Some(_) => Ground::Floor,
        }
    }

    pub fn height(self, wall: f32) -> f32 {
        match self {
            Ground::Floor => 0.0,
            Ground::Water => -0.35,
            Ground::Lava => -0.15,
            Ground::LowWall => wall * 0.5,
            Ground::Wall => wall,
        }
    }

    /// The walker's letter for it.
    pub fn letter(self) -> char {
        match self {
            Ground::Floor => '.',
            Ground::Water => '~',
            Ground::Lava => '^',
            Ground::LowWall | Ground::Wall => '#',
        }
    }

    pub fn walkable(self) -> bool {
        !matches!(self, Ground::LowWall | Ground::Wall)
    }
}

/// A link as the walker takes it: a rectangle in world tiles, the world it
/// leads to, and where it lands there (`None` keeps that coordinate).
#[derive(Debug, Clone, PartialEq)]
pub struct WalkLink {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub world: String,
    /// The destination level's top-left corner in its world, so a kept
    /// coordinate (`playerx`) can be moved into the destination level.
    pub dest_origin: (i32, i32),
    pub dest_x: Option<f32>,
    pub dest_y: Option<f32>,
    /// Added to a kept coordinate (`playerx+2`).
    pub keep_offset: (f32, f32),
    /// The origin of the level the link is in, for the same reason.
    pub from_origin: (i32, i32),
}

/// A sign the walker shows when the player stands next to it.
#[derive(Debug, Clone, PartialEq)]
pub struct WalkSign {
    pub x: i32,
    pub y: i32,
    pub text: String,
}

/// One 3D world: a single level, or every level of one gmap.
#[derive(Debug, Clone)]
pub struct World {
    /// `default` for the start world, else the world's name (the level's or
    /// gmap's file name without extension, lower case, as seihai-gserver
    /// names worlds).
    pub name: String,
    /// Where it came from, for the header and the report.
    pub source: String,
    /// Size in levels.
    pub width: usize,
    pub height: usize,
    /// `(column, row, level file name)` of each level in it.
    pub levels: Vec<(usize, usize, String)>,
    /// `width * 64` by `height * 64` tiles, `None` in a gmap's holes.
    pub ground: Vec<Option<Ground>>,
    pub links: Vec<WalkLink>,
    pub signs: Vec<WalkSign>,
    pub spawn: (f32, f32),
    /// The 3D scene: top height and colour of every tile, where a walker
    /// stands, and the things (trees, buildings ...) standing on tiles.
    pub top: Vec<f32>,
    pub colour: Vec<Colour>,
    pub walk: Vec<Option<f32>>,
    pub things: Vec<((i32, i32), Shape)>,
    pub counts: scene::Counts,
}

impl World {
    pub fn tiles_w(&self) -> usize {
        self.width * 64
    }
    pub fn tiles_h(&self) -> usize {
        self.height * 64
    }
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.tiles_w() && (y as usize) < self.tiles_h())
            .then(|| y as usize * self.tiles_w() + x as usize)
    }

    /// Where a walker stands on tile `(x, y)`, if it can.
    pub fn walk_at(&self, x: i32, y: i32) -> Option<f32> {
        self.index(x, y)
            .and_then(|i| self.walk.get(i).copied().flatten())
    }

    /// The top of tile `(x, y)`; `None` off the world and in holes.
    pub fn top_at(&self, x: i32, y: i32) -> Option<f32> {
        self.index(x, y)
            .filter(|&i| self.ground[i].is_some())
            .map(|i| self.top[i])
    }

    pub fn at(&self, x: i32, y: i32) -> Option<Ground> {
        if x < 0 || y < 0 || x as usize >= self.tiles_w() || y as usize >= self.tiles_h() {
            return None;
        }
        self.ground[y as usize * self.tiles_w() + x as usize]
    }
}

/// What came across, what was approximated and what was dropped, per level.
#[derive(Debug, Default, Clone)]
pub struct LevelReport {
    pub file: String,
    pub world: String,
    pub floor: usize,
    pub water: usize,
    pub lava: usize,
    pub walls: usize,
    pub upper_layer_tiles: usize,
    pub links: usize,
    pub links_unresolved: Vec<String>,
    pub npcs: usize,
    pub npc_scripts: usize,
    /// NPCs drawn as a cut-out of their picture, as a figure in their
    /// picture's colours, and as a figure without a picture.
    pub npc_made: [usize; 3],
    pub signs: usize,
    pub chests: usize,
    pub baddies: usize,
    pub colours: usize,
}

/// The whole conversion: the worlds and the files to write.
#[derive(Debug, Default)]
pub struct Converted {
    pub worlds: Vec<World>,
    pub levels: Vec<LevelReport>,
    pub warnings: Vec<String>,
    pub files: Vec<File>,
    /// The object sets read (file, stamps) and how many stamp names said
    /// nothing.
    pub object_sets: Vec<(String, usize)>,
    pub unnamed_stamps: usize,
}

/// World names: lower case, letters, digits, `_` and `-`.
pub fn world_name(file: &str) -> String {
    let base = Path::new(file)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    base.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn is_level(p: &Path) -> bool {
    matches!(
        p.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .as_deref(),
        Some("nw" | "graal" | "zelda")
    )
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if depth < 6 {
                walk(&p, depth + 1, out);
            }
        } else {
            out.push(p);
        }
    }
}

/// Average colour of each tile id on a `pics1.png`-layout sheet.
struct Palette {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
    bits: u8,
    cache: HashMap<u16, Colour>,
    solid: HashMap<u16, Colour>,
}

impl Palette {
    fn new(png: &[u8], bits: u8) -> Result<Palette, String> {
        let (w, h, rgba) = read_png_rgba(png).map_err(|e| format!("tileset: {e}"))?;
        Ok(Palette {
            w,
            h,
            rgba,
            bits: bits.clamp(1, 8),
            cache: HashMap::new(),
            solid: HashMap::new(),
        })
    }

    fn quantise(&self, rgb: [u32; 3]) -> Colour {
        let shift = 8 - u32::from(self.bits);
        Colour(rgb.map(|c| {
            let q = (c.min(255) >> shift) << shift;
            // The middle of the band, so dark bands are not black.
            (q + ((1 << shift) >> 1)).min(255) as u8
        }))
    }

    fn tile(&mut self, id: u16) -> Colour {
        if let Some(c) = self.cache.get(&id) {
            return *c;
        }
        let t = u32::from(id);
        let (col, row) = (t % 16 + (t / 512) * 16, (t / 16) % 32);
        let (sx, sy) = (col * 16, row * 16);
        let mut sum = [0u32; 3];
        let mut n = 0u32;
        if sx + 16 <= self.w && sy + 16 <= self.h {
            for y in sy..sy + 16 {
                for x in sx..sx + 16 {
                    let i = ((y * self.w + x) * 4) as usize;
                    for (k, v) in sum.iter_mut().enumerate() {
                        *v += u32::from(self.rgba[i + k]);
                    }
                    n += 1;
                }
            }
        }
        let c = if n == 0 {
            Colour([128, 128, 128])
        } else {
            self.quantise(sum.map(|s| s / n))
        };
        self.cache.insert(id, c);
        c
    }

    /// The tile's colour without its grass: the average of the pixels
    /// that are not green, so a house front drawn over grass gives the
    /// colour of its wood or stone. The whole average if all is green.
    fn solid(&mut self, id: u16) -> Colour {
        if let Some(c) = self.solid.get(&id) {
            return *c;
        }
        let t = u32::from(id);
        let (sx, sy) = ((t % 16 + (t / 512) * 16) * 16, ((t / 16) % 32) * 16);
        let mut sum = [0u32; 3];
        let mut n = 0u32;
        if sx + 16 <= self.w && sy + 16 <= self.h {
            for y in sy..sy + 16 {
                for x in sx..sx + 16 {
                    let i = ((y * self.w + x) * 4) as usize;
                    let [r, g, b] = [0, 1, 2].map(|k| u32::from(self.rgba[i + k]));
                    if g > r + 12 && g > b + 12 {
                        continue;
                    }
                    sum = [sum[0] + r, sum[1] + g, sum[2] + b];
                    n += 1;
                }
            }
        }
        let c = if n < 32 {
            self.tile(id)
        } else {
            self.quantise(sum.map(|s| s / n))
        };
        self.solid.insert(id, c);
        c
    }

    fn blend(&self, a: Colour, b: [u8; 3], f: f32) -> Colour {
        let mix =
            [0, 1, 2].map(|k| (f32::from(a.0[k]) * (1.0 - f) + f32::from(b[k]) * f).round() as u32);
        self.quantise(mix)
    }
}

const LINK_TINT: [u8; 3] = [70, 200, 255];
const BADDY_COLOUR: Colour = Colour([230, 40, 40]);
const WOOD: Colour = Colour([140, 95, 50]);
const GOLD: Colour = Colour([230, 190, 40]);
const SIGN_FACE: Colour = Colour([225, 205, 150]);

/// A level read in, with its tiles resolved.
struct Loaded {
    file: String,
    level: Level,
    /// Layer-0 kind and top colour per tile.
    ground: Vec<Ground>,
    colour: Vec<Colour>,
    /// The top tile's colour without grass, for house walls.
    solid: Vec<Colour>,
    /// What each tile is part of (from the top-most tile, else layer 0).
    kinds: Vec<Kind>,
    report: LevelReport,
}

fn load_level(
    file: &str,
    level: Level,
    types: &TileTypes,
    pal: &mut Palette,
    meanings: &Meanings,
) -> Loaded {
    let ids = level.tile_ids();
    let layers: Vec<i32> = level.tile_layers().into_iter().filter(|&l| l > 0).collect();
    let mut report = LevelReport {
        file: file.to_string(),
        ..Default::default()
    };
    let mut ground = Vec::with_capacity(4096);
    let mut colour = Vec::with_capacity(4096);
    let mut solid = Vec::with_capacity(4096);
    let mut kinds = Vec::with_capacity(4096);
    for y in 0..64 {
        for x in 0..64 {
            let id = ids[y * 64 + x];
            let g = Ground::from_type(types.raw(id));
            // The top-most drawn tile gives the colour; layer 0 says how it
            // stands, as in the 2D client's walking test.
            let mut top = id;
            for &l in layers.iter().rev() {
                if let Some(u) = level.tile_id_on_layer(l, x as i32, y as i32) {
                    if u != EMPTY_UPPER_TILE {
                        top = u;
                        report.upper_layer_tiles += 1;
                        break;
                    }
                }
            }
            match g {
                Ground::Floor => report.floor += 1,
                Ground::Water => report.water += 1,
                Ground::Lava => report.lava += 1,
                Ground::LowWall | Ground::Wall => report.walls += 1,
            }
            ground.push(g);
            colour.push(pal.tile(top));
            solid.push(pal.solid(top));
            // An upper layer's thing on plain ground wins; else layer 0's.
            let k0 = meanings.kind(id, types);
            let kt = meanings.kind(top, types);
            kinds.push(if top != id && kt != Kind::Ground {
                kt
            } else {
                k0
            });
        }
    }
    report.npcs = level.npcs.len();
    report.npc_scripts = level
        .npcs
        .iter()
        .filter(|n| !n.script.trim().is_empty())
        .count();
    report.signs = level.signs.len();
    report.chests = level.chests.len();
    report.baddies = level.baddies.len();
    report.links = level.links.len();
    Loaded {
        file: file.to_string(),
        level,
        ground,
        colour,
        solid,
        kinds,
        report,
    }
}

/// Convert the playerworld `o.input`.
pub fn convert(o: &Options) -> Result<Converted, String> {
    if !(o.wall_height.is_finite() && o.wall_height > 0.0) {
        return Err("the wall height must be a positive number".into());
    }
    let mut out = Converted::default();
    let types = match &o.tiledefs {
        Some(p) => {
            let lower = p.to_string_lossy().to_lowercase();
            if lower.ends_with(".dat") {
                TileTypes::load_arrays(p)?
            } else {
                TileTypes::load(p)?
            }
        }
        None => TileTypes::parse(&String::from_utf8_lossy(
            seihai_core::assets::builtin::bytes("tiledefs.txt")
                .ok_or("seihai-core has no built-in tiledefs.txt")?,
        ))?,
    };
    let sheet = match &o.tileset {
        Some(p) => std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?,
        None => seihai_core::assets::builtin::bytes("pics1.png")
            .ok_or("seihai-core has no built-in pics1.png")?
            .to_vec(),
    };
    let mut pal = Palette::new(&sheet, o.colour_bits)?;
    let sets = match &o.objects {
        Some(p) => {
            let sets = read_object_sets(p)?;
            if sets.is_empty() {
                return Err(format!(
                    "no object sets (GOBJSET01 or objects.dat) in {}",
                    p.display()
                ));
            }
            sets
        }
        None => Vec::new(),
    };
    let meanings = Meanings::new(&sets, &types);
    out.object_sets = meanings.sources.clone();
    out.unnamed_stamps = meanings.unnamed.len();

    // Every level and gmap under the input folder, by lower-cased file name.
    let mut paths = Vec::new();
    walk(&o.input, 0, &mut paths);
    let mut image_files = paths.clone();
    for d in &o.images {
        walk(d, 0, &mut image_files);
    }
    let mut images = Images::new(image_files);
    let mut level_paths: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut gmap_paths: BTreeMap<String, PathBuf> = BTreeMap::new();
    for p in paths {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let key = name.to_lowercase();
        let map = if is_level(&p) {
            &mut level_paths
        } else if key.ends_with(".gmap") {
            &mut gmap_paths
        } else {
            continue;
        };
        match map.entry(key) {
            std::collections::btree_map::Entry::Occupied(_) => out.warnings.push(format!(
                "{} has the same name as an earlier file; the first is used",
                p.display()
            )),
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(p);
            }
        }
    }
    if level_paths.is_empty() {
        return Err(format!("no levels under {}", o.input.display()));
    }

    let mut loaded: BTreeMap<String, Loaded> = BTreeMap::new();
    for (key, p) in &level_paths {
        match Level::load(p) {
            Ok(level) => {
                let file = p.file_name().unwrap().to_string_lossy().to_string();
                loaded.insert(
                    key.clone(),
                    load_level(&file, level, &types, &mut pal, &meanings),
                );
            }
            Err(e) => out.warnings.push(format!("left out: {e}")),
        }
    }
    if loaded.is_empty() {
        return Err("none of the levels could be read".into());
    }

    // The start level.
    let start_key = match &o.start {
        Some(s) => {
            let k = s.to_lowercase();
            let k = if loaded.contains_key(&k) {
                k
            } else {
                loaded
                    .keys()
                    .find(|n| world_name(n) == world_name(&k))
                    .cloned()
                    .ok_or(format!("no level {s} in the playerworld"))?
            };
            k
        }
        None => ["onlinestartlocal", "start", "startlevel"]
            .iter()
            .find_map(|w| loaded.keys().find(|k| world_name(k) == *w).cloned())
            .unwrap_or_else(|| loaded.keys().next().unwrap().clone()),
    };

    // Worlds: each gmap, then each level no gmap holds.
    let mut worlds: Vec<World> = Vec::new();
    // Level key -> (world index, column, row).
    let mut place: HashMap<String, (usize, usize, usize)> = HashMap::new();
    for (key, p) in &gmap_paths {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let Some(g) = Gmap::parse(&text) else {
            out.warnings
                .push(format!("left out: {} is not a GRMAP001 gmap", p.display()));
            continue;
        };
        let wi = worlds.len();
        let mut levels = Vec::new();
        for (i, n) in g.names.iter().enumerate() {
            let k = n.trim().to_lowercase();
            if k.is_empty() {
                continue;
            }
            if !loaded.contains_key(&k) {
                out.warnings
                    .push(format!("{key}: level {n} is missing; a hole"));
                continue;
            }
            if place.contains_key(&k) {
                out.warnings.push(format!(
                    "{key}: {n} is in an earlier gmap too; that one is used"
                ));
                continue;
            }
            let (cx, cy) = (i % g.width, i / g.width);
            place.insert(k.clone(), (wi, cx, cy));
            levels.push((cx, cy, k));
        }
        if levels.is_empty() {
            continue;
        }
        worlds.push(World {
            name: world_name(key),
            source: p.file_name().unwrap().to_string_lossy().to_string(),
            width: g.width,
            height: g.height,
            levels,
            ground: Vec::new(),
            links: Vec::new(),
            signs: Vec::new(),
            spawn: (32.0, 32.0),
            top: Vec::new(),
            colour: Vec::new(),
            walk: Vec::new(),
            things: Vec::new(),
            counts: Default::default(),
        });
    }
    for key in loaded.keys() {
        if place.contains_key(key) {
            continue;
        }
        place.insert(key.clone(), (worlds.len(), 0, 0));
        worlds.push(World {
            name: world_name(key),
            source: loaded[key].file.clone(),
            width: 1,
            height: 1,
            levels: vec![(0, 0, key.clone())],
            ground: Vec::new(),
            links: Vec::new(),
            signs: Vec::new(),
            spawn: (32.0, 32.0),
            top: Vec::new(),
            colour: Vec::new(),
            walk: Vec::new(),
            things: Vec::new(),
            counts: Default::default(),
        });
    }
    // Two worlds may not share a name (a gmap and a level, or "a b.nw" and
    // "a_b.nw"): later ones get a number.
    let mut seen: HashMap<String, usize> = HashMap::new();
    for w in &mut worlds {
        let n = seen.entry(w.name.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            out.warnings.push(format!(
                "{} is world {}_{n}: the name was taken",
                w.source, w.name
            ));
            w.name = format!("{}_{n}", w.name);
        }
    }
    let start_world = place[&start_key].0;
    worlds[start_world].name = "default".into();

    // Ground grids, links and signs.
    for (wi, world) in worlds.iter_mut().enumerate() {
        let (tw, th) = (world.width * 64, world.height * 64);
        let mut ground = vec![None; tw * th];
        let mut kinds = vec![Kind::Ground; tw * th];
        let mut colours = vec![Colour([0, 0, 0]); tw * th];
        let mut solids = colours.clone();
        let mut links = Vec::new();
        let mut signs = Vec::new();
        for (cx, cy, key) in world.levels.clone() {
            let l = &loaded[&key];
            let (ox, oy) = ((cx * 64) as i32, (cy * 64) as i32);
            for y in 0..64 {
                for x in 0..64 {
                    let i = (oy as usize + y) * tw + ox as usize + x;
                    ground[i] = Some(l.ground[y * 64 + x]);
                    kinds[i] = l.kinds[y * 64 + x];
                    colours[i] = l.colour[y * 64 + x];
                    solids[i] = l.solid[y * 64 + x];
                }
            }
            for link in &l.level.links {
                let dk = link.dest.trim().to_lowercase();
                let Some(&(dw, dcx, dcy)) = place.get(&dk).or_else(|| {
                    // A link may leave the extension out or name another one.
                    let want = world_name(&dk);
                    place
                        .iter()
                        .find(|(k, _)| world_name(k) == want)
                        .map(|(_, v)| v)
                }) else {
                    continue;
                };
                // Inside a gmap the levels already touch: a link to a
                // neighbour in the same gmap is its edge, and walking on
                // is the way across.
                if dw == wi && (dcx, dcy) != (cx, cy) {
                    continue;
                }
                links.push(WalkLink {
                    x: ox + link.x,
                    y: oy + link.y,
                    w: link.width.max(1),
                    h: link.height.max(1),
                    world: String::new(), // named below, once every world has its name
                    dest_origin: ((dcx * 64) as i32, (dcy * 64) as i32),
                    dest_x: link.new_x,
                    dest_y: link.new_y,
                    keep_offset: (link.x_offset, link.y_offset),
                    from_origin: (ox, oy),
                });
                // The destination world index rides in `w`'s place until the
                // names are final.
                links.last_mut().unwrap().world = dw.to_string();
            }
            for s in &l.level.signs {
                signs.push(WalkSign {
                    x: ox + s.x,
                    y: oy + s.y,
                    text: s.text.clone(),
                });
            }
        }
        let sc = scene::build(
            &scene::Tiles {
                w: tw,
                h: th,
                ground: &ground,
                kind: &kinds,
                colour: &colours,
                solid: &solids,
            },
            o.wall_height,
        );
        world.top = sc.top;
        world.colour = sc.colour;
        world.walk = sc.walk;
        world.things = sc.things;
        world.counts = sc.counts;
        world.ground = ground;
        world.links = links;
        world.signs = signs;
    }
    let names: Vec<String> = worlds.iter().map(|w| w.name.clone()).collect();
    for w in &mut worlds {
        for l in &mut w.links {
            l.world = names[l.world.parse::<usize>().unwrap()].clone();
        }
    }
    // Unresolved links, for the report.
    for l in loaded.values_mut() {
        for link in &l.level.links {
            let dk = link.dest.trim().to_lowercase();
            let want = world_name(&dk);
            if !place.contains_key(&dk) && !place.keys().any(|k| world_name(k) == want) {
                l.report.links_unresolved.push(link.dest.clone());
            }
        }
    }

    // Spawns: the start position in the start world; elsewhere the open
    // floor nearest the middle of the first level.
    for (wi, w) in worlds.iter_mut().enumerate() {
        let (cx, cy, _) = w.levels[0];
        let (ox, oy) = ((cx * 64) as f32, (cy * 64) as f32);
        let want = if wi == start_world {
            let (_, sx, sy) = place[&start_key];
            let (sox, soy) = ((sx * 64) as f32, (sy * 64) as f32);
            o.start_pos
                .map(|(x, y)| (sox + x, soy + y))
                .unwrap_or((sox + 32.0, soy + 32.0))
        } else {
            (ox + 32.0, oy + 32.0)
        };
        w.spawn = nearest_floor(w, want);
    }

    // The files.
    for w in &worlds {
        let mut text = world_header(w);
        for (n, (cx, cy, key)) in w.levels.iter().enumerate() {
            let l = &loaded[key];
            let (shape, npc_made) = level_shape(w, *cx, *cy, l, &pal, &mut images);
            let colours = shape.colours();
            // A gmap's levels carry the gmap's name too.
            let file = if w.levels.len() == 1 {
                format!("pw3d_{}.3ds", world_name(key))
            } else {
                format!("pw3d_{}_{}.3ds", world_name(&w.source), world_name(key))
            };
            text.push_str(&format!(
                "object PW3D_L{n} {file} {} {} 0\n",
                cx * 64,
                -((cy * 64) as i64)
            ));
            out.files.push((format!("files/{file}"), shape.to_3ds("g")));
            let mut r = l.report.clone();
            r.world = w.name.clone();
            r.colours = colours.len();
            r.npc_made = npc_made;
            out.levels.push(r);
        }
        let z = w.walk_at(w.spawn.0 as i32, w.spawn.1 as i32).unwrap_or(0.0) + 1.0;
        text.push_str(&format!(
            "spawn {:.2} {:.2} {:.2}\n",
            w.spawn.0, -w.spawn.1, z
        ));
        text.push_str(&format!("weapon3d {} {}\n", walker::WEAPON, walker::FILE));
        let path = if w.name == "default" {
            "config/graal3dworld.txt".to_string()
        } else {
            format!("config/graal3dworlds/{}.txt", w.name)
        };
        out.files.push((path, text.into_bytes()));
    }
    // Shared shapes and textures.
    out.files
        .push(("files/pw3d_player.3ds".into(), player_shape().to_3ds("p")));
    for i in 0..6 {
        out.files
            .push((format!("files/pw3d_sky{i}.png"), sky_face(i)?));
    }
    out.files.push((
        walker::FILE.to_string(),
        walker::script(&worlds).into_bytes(),
    ));
    // The 2D levels too, under their own names, so 2D players can walk the
    // same world and a TCP warp to one takes its 3D client along.
    for l in loaded.values() {
        let p = &level_paths[&l.file.to_lowercase()];
        let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
        out.files.push((format!("levels/{}", l.file), bytes));
    }
    for (key, p) in &gmap_paths {
        let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
        out.files.push((format!("levels/{key}"), bytes));
    }
    if o.meaning_maps {
        for l in loaded.values() {
            out.files.push((
                format!("PW3D-MAPS/{}.png", world_name(&l.file)),
                meaning_map(&l.kinds)?,
            ));
        }
    }
    out.worlds = worlds;
    out.files
        .push(("PW3D-REPORT.md".into(), report(&out, o).into_bytes()));
    Ok(out)
}

fn nearest_floor(w: &World, want: (f32, f32)) -> (f32, f32) {
    let (wx, wy) = (want.0.floor() as i32, want.1.floor() as i32);
    if w.at(wx, wy).is_some_and(|g| g == Ground::Floor) {
        return want;
    }
    let mut best: Option<(i32, i32, i32)> = None;
    for y in 0..w.tiles_h() as i32 {
        for x in 0..w.tiles_w() as i32 {
            if w.at(x, y) == Some(Ground::Floor) {
                let d = (x - wx).pow(2) + (y - wy).pow(2);
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, x, y));
                }
            }
        }
    }
    best.map_or(want, |(_, x, y)| (x as f32 + 0.5, y as f32 + 0.5))
}

fn world_header(w: &World) -> String {
    let mut t = format!(
        "# {}: the 3D version of the 2D {} ({} x {} levels), from seihai-pw3d.\n\
         # One unit is one tile; map tile (x, y) is (x, -y). Each level is one\n\
         # coloured .3ds object; the PW3DWalk weapon walks the player, stops it\n\
         # at walls and takes the level's links.\n\n",
        w.name, w.source, w.width, w.height
    );
    t.push_str("sun 0.57735 0.57735 -0.57735 0.85 0.85 0.8 1 0.4 0.4 0.45 1\n");
    t.push_str("sky pw3d_sky0,pw3d_sky1,pw3d_sky2,pw3d_sky3,pw3d_sky4,pw3d_sky5 1000\n");
    t.push_str("skyoptions 1 0 0 0.45 0.65 0.95\n");
    t.push_str("playershape pw3d_player.3ds\n");
    t.push_str("movetype interpolate\n");
    t
}

/// The ground of one level: tops of every tile (equal neighbours merged),
/// sides where a tile stands higher than its neighbour, and markers for its
/// NPCs, signs, chests and baddies. In level-local units; the world file
/// puts the object at the level's corner.
fn level_shape(
    w: &World,
    cx: usize,
    cy: usize,
    l: &Loaded,
    pal: &Palette,
    images: &mut Images,
) -> (Shape, [usize; 3]) {
    let (ox, oy) = ((cx * 64) as i32, (cy * 64) as i32);
    let mut s = Shape::default();
    let wi = |x: usize, y: usize| (oy as usize + y) * w.tiles_w() + ox as usize + x;
    // Top colour per tile, with link tiles tinted.
    let mut top: Vec<Colour> = (0..4096).map(|i| w.colour[wi(i % 64, i / 64)]).collect();
    for link in &l.level.links {
        for y in link.y.max(0)..(link.y + link.height.max(1)).min(64) {
            for x in link.x.max(0)..(link.x + link.width.max(1)).min(64) {
                let i = (y * 64 + x) as usize;
                top[i] = pal.blend(top[i], LINK_TINT, 0.45);
            }
        }
    }
    let h = |x: usize, y: usize| w.top[wi(x, y)];
    let floor = w.top.iter().copied().fold(0.0f32, f32::min) - 1.0;
    // Greedy rectangles of equal height and colour.
    let mut done = vec![false; 4096];
    for y in 0..64 {
        for x in 0..64 {
            if done[y * 64 + x] {
                continue;
            }
            let key = (h(x, y).to_bits(), top[y * 64 + x]);
            let same = |x: usize, y: usize| (h(x, y).to_bits(), top[y * 64 + x]) == key;
            let mut x1 = x + 1;
            while x1 < 64 && !done[y * 64 + x1] && same(x1, y) {
                x1 += 1;
            }
            let mut y1 = y + 1;
            'grow: while y1 < 64 {
                for xx in x..x1 {
                    if done[y1 * 64 + xx] || !same(xx, y1) {
                        break 'grow;
                    }
                }
                y1 += 1;
            }
            for yy in y..y1 {
                for xx in x..x1 {
                    done[yy * 64 + xx] = true;
                }
            }
            let z = h(x, y);
            let (fx0, fx1) = (x as f32, x1 as f32);
            let (fy0, fy1) = (-(y1 as f32), -(y as f32));
            s.quad(
                [[fx0, fy0, z], [fx1, fy0, z], [fx1, fy1, z], [fx0, fy1, z]],
                key.1,
            );
        }
    }
    // Sides: east, west, south (+map y, -3D y) and north of each tile, down
    // to the neighbour (or below the lowest ground past the world's edge
    // and in holes).
    for y in 0..64usize {
        for x in 0..64usize {
            let z = h(x, y);
            let side = top[y * 64 + x].shade(0.65);
            let (gx, gy) = (ox + x as i32, oy + y as i32);
            let below = |nx: i32, ny: i32| w.top_at(nx, ny).unwrap_or(floor);
            let (x0, x1) = (x as f32, x as f32 + 1.0);
            let (y0, y1) = (-(y as f32) - 1.0, -(y as f32));
            let zb = below(gx + 1, gy);
            if zb < z {
                s.quad([[x1, y0, zb], [x1, y1, zb], [x1, y1, z], [x1, y0, z]], side);
            }
            let zb = below(gx - 1, gy);
            if zb < z {
                s.quad([[x0, y1, zb], [x0, y0, zb], [x0, y0, z], [x0, y1, z]], side);
            }
            let zb = below(gx, gy + 1);
            if zb < z {
                s.quad([[x0, y0, zb], [x1, y0, zb], [x1, y0, z], [x0, y0, z]], side);
            }
            let zb = below(gx, gy - 1);
            if zb < z {
                s.quad([[x1, y1, zb], [x0, y1, zb], [x0, y1, z], [x1, y1, z]], side);
            }
        }
    }
    // The things standing in this level, moved to its corner.
    for ((tx, ty), thing) in &w.things {
        if (ox..ox + 64).contains(tx) && (oy..oy + 64).contains(ty) {
            let mut t = thing.clone();
            t.shift([-(ox as f32), oy as f32, 0.0]);
            s.extend(&t);
        }
    }
    let floor_at = |x: f32, y: f32| {
        let (tx, ty) = (
            x.floor().clamp(0.0, 63.0) as usize,
            y.floor().clamp(0.0, 63.0) as usize,
        );
        h(tx, ty)
    };
    // NPCs: a cut-out of their picture, or a figure (npc.rs).
    let mut made = [0usize; 3];
    let q = |c: [u32; 3]| pal.quantise(c);
    for n in &l.level.npcs {
        if n.image.trim().is_empty() && n.ani.is_none() {
            continue;
        }
        let z = floor_at(n.x + 0.5, n.y + 0.5);
        let img = if n.ani.is_some() {
            None
        } else {
            images.get(&n.image)
        };
        let (shape, how) = npc_shape(img, n.image_part, n.x, n.y, z, &q);
        made[how as usize] += 1;
        s.extend(&shape);
    }
    for b in &l.level.baddies {
        let z = floor_at(b.x + 0.5, b.y + 0.5);
        s.extend(&super::npc::figure(
            b.x + 1.0,
            -b.y - 1.5,
            z,
            BADDY_COLOUR,
            BADDY_COLOUR.shade(0.6),
        ));
    }
    for c in &l.level.chests {
        let (x, y) = (c.x as f32, c.y as f32);
        let z = floor_at(x + 1.0, y + 1.0);
        s.block(
            [x + 0.15, -y - 1.85, z],
            [x + 1.85, -y - 0.15, z + 0.9],
            GOLD,
            WOOD,
        );
    }
    for sg in &l.level.signs {
        let (x, y) = (sg.x as f32, sg.y as f32);
        let z = floor_at(x + 0.5, y + 0.5);
        s.block(
            [x + 0.42, -y - 0.58, z],
            [x + 0.58, -y - 0.42, z + 0.7],
            WOOD,
            WOOD,
        );
        s.block(
            [x + 0.05, -y - 0.6, z + 0.7],
            [x + 0.95, -y - 0.4, z + 1.3],
            SIGN_FACE,
            WOOD,
        );
    }
    (s, made)
}

/// The player: a blue body with a light head, 1.6 tall, its feet on its
/// origin.
fn player_shape() -> Shape {
    let mut s = Shape::default();
    let body = Colour([40, 80, 200]);
    let head = Colour([240, 200, 160]);
    s.block([-0.3, -0.3, -1.0], [0.3, 0.3, 0.2], body, body.shade(0.7));
    s.block(
        [-0.22, -0.22, 0.2],
        [0.22, 0.22, 0.6],
        head,
        head.shade(0.8),
    );
    s
}

/// One sky face: 256 x 256 24-bit, blue, lighter toward the horizon (the
/// bottom of a side face).
fn sky_face(i: usize) -> Result<Vec<u8>, String> {
    let mut px = Vec::with_capacity(256 * 256 * 3);
    for y in 0..256u32 {
        let t = match i {
            4 => 0.0, // up
            5 => 1.0, // down
            _ => y as f32 / 255.0,
        };
        let c = [
            (90.0 + 120.0 * t) as u8,
            (140.0 + 90.0 * t) as u8,
            (225.0 + 25.0 * t) as u8,
        ];
        for _ in 0..256 {
            px.extend_from_slice(&c);
        }
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 256, 256);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&px).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

fn report(c: &Converted, o: &Options) -> String {
    let mut t = String::from("# seihai-pw3d report\n\n");
    t.push_str(&format!(
        "Input: `{}`. Tile colours from {}; walls and water from {}. Walls {} tall.\n\n",
        o.input.display(),
        o.tileset
            .as_ref()
            .map_or("Seihai's built-in tileset".to_string(), |p| format!(
                "`{}`",
                p.display()
            )),
        o.tiledefs
            .as_ref()
            .map_or("Seihai's built-in tiledefs".to_string(), |p| format!(
                "`{}`",
                p.display()
            )),
        o.wall_height
    ));
    if c.object_sets.is_empty() {
        t.push_str("No object sets (--objects): tiles are read by their tiledefs type only.\n\n");
    } else {
        let total: usize = c.object_sets.iter().map(|s| s.1).sum();
        t.push_str(&format!(
            "What the tiles show, from {} object sets, {total} stamps ({} with names that say nothing):",
            c.object_sets.len(),
            c.unnamed_stamps
        ));
        for (f, n) in &c.object_sets {
            t.push_str(&format!(" `{f}` {n};"));
        }
        t.push_str("\n\n");
    }
    t.push_str(
        "This folder holds the playerworld's own levels; keep it local and do not publish it.\n\n",
    );
    t.push_str("## Worlds\n\n| world | from | levels | links | signs | spawn |\n|---|---|---|---|---|---|\n");
    for w in &c.worlds {
        t.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.1},{:.1} |\n",
            w.name,
            w.source,
            w.levels.len(),
            w.links.len(),
            w.signs.len(),
            w.spawn.0,
            w.spawn.1
        ));
    }
    t.push_str("\n## Levels\n\n| level | world | floor | water | lava | walls | upper-layer tiles | colours | links (unresolved) | NPCs (with scripts) | NPCs as picture / figure / guess | signs | chests | baddies |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for l in &c.levels {
        t.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} ({}) | {} ({}) | {} / {} / {} | {} | {} | {} |\n",
            l.file,
            l.world,
            l.floor,
            l.water,
            l.lava,
            l.walls,
            l.upper_layer_tiles,
            l.colours,
            l.links,
            l.links_unresolved.len(),
            l.npcs,
            l.npc_scripts,
            l.npc_made[0],
            l.npc_made[1],
            l.npc_made[2],
            l.signs,
            l.chests,
            l.baddies
        ));
    }
    let unresolved: Vec<String> = c
        .levels
        .iter()
        .flat_map(|l| {
            l.links_unresolved
                .iter()
                .map(move |d| format!("{} -> {d}", l.file))
        })
        .collect();
    if !unresolved.is_empty() {
        t.push_str("\n## Links to levels not in the playerworld (dropped)\n\n");
        for u in unresolved {
            t.push_str(&format!("- {u}\n"));
        }
    }
    if !c.warnings.is_empty() {
        t.push_str("\n## Warnings\n\n");
        for w in &c.warnings {
            t.push_str(&format!("- {w}\n"));
        }
    }
    t.push_str(
        "\n## What came across\n\n\
         - Lossless: the tile grid's layout and what each tile does to a walker \
         (floor, water, lava, wall), link rectangles and where they lead, sign text, \
         where NPCs, chests and baddies stand, and the gmap layout.\n\
         - Approximated: each tile is one flat colour (its average on the tileset); \
         walls are raised blocks, low walls half as tall, water and lava sunk; upper \
         tile layers give only the colour; NPCs are cut-outs of their picture \
         (or figures where the picture is a sheet, a gani or not found), baddies, \
         chests and signs simple shapes; links are tinted tiles.\n\
         - Dropped: NPC scripts, baddy behaviour, chest items, level \
         music and the tileset's own pictures. The 2D levels are copied beside the \
         3D worlds unchanged, so 2D players still get all of it.\n",
    );
    t
}

/// A 64 x 64 picture of a level's tile meanings, 8 pixels a tile.
fn meaning_map(kinds: &[Kind]) -> Result<Vec<u8>, String> {
    let mut px = Vec::with_capacity(512 * 512 * 3);
    for py in 0..512 {
        for qx in 0..512 {
            let k = kinds[(py / 8) * 64 + qx / 8];
            px.extend_from_slice(&k.debug_rgb());
        }
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 512, 512);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&px).map_err(|e| e.to_string())?;
    }
    Ok(out)
}
