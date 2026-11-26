//! What the tiles mean: trees, houses, cliffs, hedges, water, paths and the
//! rest, so a level can be rebuilt as a 3D place rather than coloured
//! tiles.
//!
//! The meaning comes from the client's own object sets: the named stamps
//! its level editor places (`GOBJSET01` text files, `objects*.txt`, or the
//! 1.x binary `objects.dat`). Every tile id a stamp uses gets a vote for
//! the kind the stamp's name says ("Tree", "House", "Platform North",
//! "Green Fences" ...). A tile used by stamps of several kinds that is not
//! a wall is ground: stamps carry the grass around a tree or a well. The
//! object sets are read from the user's own client at conversion time;
//! none of them is in this repository.
//!
//! Tiles no stamp uses fall back on the tile types: walls are walls,
//! water is water.

use seihai_core::assets::{ObjectSet, TileTypes};
use std::collections::HashMap;
use std::path::Path;

/// What a tile is part of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Ground,
    Path,
    Water,
    Lava,
    Hole,
    Bridge,
    Stairs,
    Tree,
    Shrub,
    Rock,
    Building,
    Door,
    CastleWall,
    Fence,
    Cliff,
    Furniture,
    /// A wall the object sets do not name.
    Wall,
}

impl Kind {
    pub const ALL: [Kind; 17] = [
        Kind::Ground,
        Kind::Path,
        Kind::Water,
        Kind::Lava,
        Kind::Hole,
        Kind::Bridge,
        Kind::Stairs,
        Kind::Tree,
        Kind::Shrub,
        Kind::Rock,
        Kind::Building,
        Kind::Door,
        Kind::CastleWall,
        Kind::Fence,
        Kind::Cliff,
        Kind::Furniture,
        Kind::Wall,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Ground => "ground",
            Kind::Path => "path",
            Kind::Water => "water",
            Kind::Lava => "lava",
            Kind::Hole => "hole",
            Kind::Bridge => "bridge",
            Kind::Stairs => "stairs",
            Kind::Tree => "tree",
            Kind::Shrub => "shrub",
            Kind::Rock => "rock",
            Kind::Building => "building",
            Kind::Door => "door",
            Kind::CastleWall => "castle wall",
            Kind::Fence => "fence",
            Kind::Cliff => "cliff",
            Kind::Furniture => "furniture",
            Kind::Wall => "wall",
        }
    }

    /// A colour for the debug maps.
    pub fn debug_rgb(self) -> [u8; 3] {
        match self {
            Kind::Ground => [90, 170, 80],
            Kind::Path => [200, 190, 150],
            Kind::Water => [40, 90, 220],
            Kind::Lava => [240, 90, 20],
            Kind::Hole => [20, 20, 20],
            Kind::Bridge => [150, 100, 50],
            Kind::Stairs => [230, 230, 120],
            Kind::Tree => [20, 90, 30],
            Kind::Shrub => [120, 220, 60],
            Kind::Rock => [140, 140, 140],
            Kind::Building => [200, 60, 60],
            Kind::Door => [255, 220, 0],
            Kind::CastleWall => [110, 90, 160],
            Kind::Fence => [180, 120, 220],
            Kind::Cliff => [130, 80, 40],
            Kind::Furniture => [250, 150, 200],
            Kind::Wall => [70, 70, 70],
        }
    }
}

/// The kind a stamp's name says, if any.
pub fn kind_of_name(name: &str) -> Option<Kind> {
    let n = name.to_lowercase();
    let has = |w: &str| n.contains(w);
    // A floor is a floor whatever room it is in ("CastleFloor Red",
    // "CastleCarpet Border"); the black void round rooms and caves is a hole.
    let floor = (has("floor") || has("carpet")) && !has("carpet+");
    Some(if floor {
        Kind::Ground
    } else if has("blackness") {
        Kind::Hole
    } else if has("door") || has("cave") || has("entrance") {
        Kind::Door
    } else if has("stair") {
        Kind::Stairs
    } else if has("bridge") {
        Kind::Bridge
    } else if has("treehouse") || has("house") || has("cathedral") {
        Kind::Building
    } else if has("stump") {
        Kind::Rock
    } else if has("tree") {
        Kind::Tree
    } else if has("bush") || has("rooshe") || has("sprout") || has("swamp") || has("flower") {
        Kind::Shrub
    } else if has("fence") || has("stake") || has("jule") || has("post") {
        Kind::Fence
    } else if has("castle") || has("mine") || has("curtain") || n.starts_with("wall") {
        Kind::CastleWall
    } else if has("corner") {
        // Inside walls ("CornerSW", "InsideCornerNE").
        Kind::CastleWall
    } else if has("platform") || has("slab") || has("projection") || has("edges") {
        Kind::Cliff
    } else if has("lava") && (has("rock") || has("bubble")) {
        Kind::Rock
    } else if has("lava") {
        Kind::Lava
    } else if has("water stone") {
        Kind::Rock
    } else if has("water") || has("falls") {
        Kind::Water
    } else if has("hole") || has("blackness") || has("muck") {
        Kind::Hole
    } else if has("stone")
        || has("rock")
        || has("boulder")
        || has("statue")
        || has("pillar")
        || has("crystal")
    {
        Kind::Rock
    } else if has("way") || has("sand") || has("walk") {
        Kind::Path
    } else if has("bed")
        || has("table")
        || has("chair")
        || has("bench")
        || has("shel")
        || has("stove")
        || has("barrel")
        || has("vase")
        || has("lamp")
        || has("well")
        || has("grave")
        || has("sign")
        || has("letterbox")
        || has("meal")
        || has("basin")
        || has("cage")
        || has("carpet+")
        || has("watering")
    {
        Kind::Furniture
    } else if has("ground")
        || has("floor")
        || has("carpet")
        || has("marble")
        || has("grass")
        || has("snow")
    {
        Kind::Ground
    } else {
        return None;
    })
}

/// Tile id to kind, from object sets and tile types.
#[derive(Debug, Clone, Default)]
pub struct Meanings {
    pub by_id: HashMap<u16, Kind>,
    /// The object sets read, with their stamp counts, for the report.
    pub sources: Vec<(String, usize)>,
    /// Stamps whose names say nothing we use.
    pub unnamed: Vec<String>,
}

/// Stamps from a 1.x binary `objects.dat`: name, newline, width and height
/// bytes, then `width * height` little-endian 32-bit tile ids.
pub fn parse_objects_dat(data: &[u8]) -> Result<Vec<(String, Vec<u16>)>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let nl = data[i..]
            .iter()
            .position(|&b| b == b'\n')
            .ok_or("objects.dat: a name without its end")?;
        let name = String::from_utf8_lossy(&data[i..i + nl]).to_string();
        let at = i + nl + 1;
        if at + 2 > data.len() {
            return Err("objects.dat: cut short".into());
        }
        let (w, h) = (data[at] as usize, data[at + 1] as usize);
        let body = at + 2;
        let end = body + 4 * w * h;
        if end > data.len() {
            return Err(format!("objects.dat: {name} is cut short"));
        }
        let ids = data[body..end]
            .chunks(4)
            .map(|c| (u32::from_le_bytes([c[0], c[1], c[2], c[3]]) & 0xFFF) as u16)
            .collect();
        out.push((name, ids));
        i = end;
    }
    Ok(out)
}

/// Object sets as read: each file's name and its stamps (name, tile ids).
pub type ObjectSets = Vec<(String, Vec<(String, Vec<u16>)>)>;

/// Read every object set in `dir` (and its `tileobjects/` folder):
/// `GOBJSET01` text files and `objects.dat`.
pub fn read_object_sets(dir: &Path) -> Result<ObjectSets, String> {
    let mut found = Vec::new();
    let mut paths = Vec::new();
    for d in [dir.to_path_buf(), dir.join("tileobjects")] {
        if let Ok(rd) = std::fs::read_dir(&d) {
            let mut ps: Vec<_> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
            ps.sort();
            paths.extend(ps);
        }
    }
    if dir.is_file() {
        paths.push(dir.to_path_buf());
    }
    for p in paths {
        if !p.is_file() {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let data = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        if data.starts_with(b"GOBJSET01") {
            let set = ObjectSet::parse(&String::from_utf8_lossy(&data))
                .map_err(|e| format!("{}: {e}", p.display()))?;
            found.push((
                name,
                set.stamps
                    .iter()
                    .map(|s| (s.name.clone(), s.ids()))
                    .collect(),
            ));
        } else if name.eq_ignore_ascii_case("objects.dat") {
            found.push((name, parse_objects_dat(&data)?));
        }
    }
    Ok(found)
}

impl Meanings {
    /// Build the table from object sets (may be empty).
    pub fn new(sets: &ObjectSets, types: &TileTypes) -> Meanings {
        let mut votes: HashMap<u16, HashMap<Kind, u32>> = HashMap::new();
        let mut m = Meanings::default();
        for (file, stamps) in sets {
            m.sources.push((file.clone(), stamps.len()));
            for (name, ids) in stamps {
                let Some(k) = kind_of_name(name) else {
                    m.unnamed.push(name.clone());
                    continue;
                };
                for &id in ids {
                    *votes.entry(id).or_default().entry(k).or_default() += 1;
                }
            }
        }
        for (id, v) in votes {
            let raw = types.raw(id).unwrap_or(22);
            let wall = raw >= 20;
            let water = raw == 11;
            let lava = raw == 12 || raw == 13;
            // Several kinds on a tile that does not block: the ground
            // around stamps.
            let kinds = v.len();
            let best = v
                .iter()
                .max_by_key(|(k, n)| (**n, std::cmp::Reverse(**k)))
                .map(|(k, _)| *k)
                .unwrap();
            let k = if water {
                if matches!(best, Kind::Bridge | Kind::Stairs) {
                    best
                } else {
                    Kind::Water
                }
            } else if lava {
                Kind::Lava
            } else if matches!(best, Kind::Water | Kind::Lava | Kind::Hole) {
                // Shores and rims drawn with a pond stamp, but not water.
                if wall {
                    Kind::Wall
                } else {
                    Kind::Ground
                }
            } else if wall && v.contains_key(&Kind::Hole) {
                // The black void a "Blackness" stamp names, whatever else
                // (a cathedral, a cave entry) is drawn round it.
                Kind::Hole
            } else if !wall
                && kinds >= 2
                && !matches!(best, Kind::Path | Kind::Stairs | Kind::Bridge)
            {
                // Ground several stamps share: under doors too (a castle
                // floor tile that a door stamp also uses).
                Kind::Ground
            } else if !wall
                && matches!(
                    best,
                    Kind::Tree
                        | Kind::Rock
                        | Kind::Shrub
                        | Kind::Fence
                        | Kind::Furniture
                        | Kind::Cliff
                )
            {
                // Walkable parts of solid things: canopy edges, shadows.
                // A tree's crown is drawn over walkable tiles too.
                if best == Kind::Tree {
                    Kind::Tree
                } else {
                    Kind::Ground
                }
            } else {
                best
            };
            m.by_id.insert(id, k);
        }
        m
    }

    /// The kind of tile `id`.
    pub fn kind(&self, id: u16, types: &TileTypes) -> Kind {
        let raw = types.raw(id);
        match raw {
            Some(11) => {
                return self
                    .by_id
                    .get(&id)
                    .copied()
                    .filter(|k| matches!(k, Kind::Bridge | Kind::Stairs))
                    .unwrap_or(Kind::Water)
            }
            Some(12 | 13) => return Kind::Lava,
            _ => {}
        }
        if let Some(k) = self.by_id.get(&id) {
            return *k;
        }
        match raw {
            None => Kind::Wall,
            Some(t) if t >= 20 => Kind::Wall,
            _ => Kind::Ground,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_say_what_a_stamp_is() {
        assert_eq!(kind_of_name("Tree"), Some(Kind::Tree));
        assert_eq!(kind_of_name("Green Little Tree"), Some(Kind::Tree));
        assert_eq!(kind_of_name("Tree Stump"), Some(Kind::Rock));
        assert_eq!(kind_of_name("TreeHouse"), Some(Kind::Building));
        assert_eq!(kind_of_name("HouseDoor South"), Some(Kind::Door));
        assert_eq!(kind_of_name("Platform NorthWest"), Some(Kind::Cliff));
        assert_eq!(kind_of_name("Green Fences"), Some(Kind::Fence));
        assert_eq!(kind_of_name("CastleWall North"), Some(Kind::CastleWall));
        assert_eq!(kind_of_name("Water Stone"), Some(Kind::Rock));
        assert_eq!(kind_of_name("Lava Bridge East/West"), Some(Kind::Bridge));
        assert_eq!(kind_of_name("VillageWay Vertical"), Some(Kind::Path));
        assert_eq!(kind_of_name("Dining Table"), Some(Kind::Furniture));
        assert_eq!(kind_of_name("CastleFloor Red"), Some(Kind::Ground));
        assert_eq!(kind_of_name("CastleCarpet Border"), Some(Kind::Ground));
        assert_eq!(kind_of_name("Blackness"), Some(Kind::Hole));
        assert_eq!(kind_of_name("Alien Pipe"), None);
    }

    #[test]
    fn objects_dat_reads() {
        let mut d = b"Bush\n\x02\x02".to_vec();
        for id in [2u32, 3, 18, 19] {
            d.extend_from_slice(&id.to_le_bytes());
        }
        let s = parse_objects_dat(&d).unwrap();
        assert_eq!(s, vec![("Bush".to_string(), vec![2, 3, 18, 19])]);
    }
}
