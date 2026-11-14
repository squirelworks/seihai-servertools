//! Build a `.gmap` (`GRMAP001`) from levels: the job of the classic map
//! makers (Graal Map Maker, MapGenerator, the generators' gmap step).
//!
//! Three ways to place the levels on the grid:
//!
//! * [`from_list`]: the names, row by row, into a grid of a given width;
//! * [`from_names`]: names of the form `<prefix><column letters><row
//!   digits>.nw` (`world-a01.nw`, `worldAA12.nw`), the pattern the
//!   generators write;
//! * [`from_links`]: walk the edge links of an old linked world (a link
//!   along the left, right, top or bottom edge leads to the neighbour on
//!   that side), so a world made of single levels becomes one gmap.
//!
//! The grid is read back with `seihai-core`'s [`Gmap`]; [`write`] is the
//! only writer. Generator keys are not written.

use std::collections::{BTreeMap, HashMap, VecDeque};

use seihai_core::assets::{Level, Link};
pub use seihai_core::gmap::Gmap;

/// The most levels across or down a gmap the reader accepts.
pub const MAX_SIDE: usize = 255;

/// A grid with what happened while it was built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    pub gmap: Gmap,
    /// Names or links that could not be placed, one line each.
    pub notes: Vec<String>,
}

fn grid(width: usize, height: usize) -> Result<Gmap, String> {
    if width == 0 || height == 0 {
        return Err("the grid is empty".into());
    }
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(format!(
            "a {width} by {height} grid is larger than {MAX_SIDE} by {MAX_SIDE}"
        ));
    }
    Ok(Gmap {
        width,
        height,
        names: vec![String::new(); width * height],
        ..Gmap::default()
    })
}

/// Place `names` row by row in a grid `width` wide. An empty name, or `-`,
/// is a hole.
pub fn from_list(names: &[String], width: usize) -> Result<Built, String> {
    if width == 0 {
        return Err("the width must be at least 1".into());
    }
    let height = names.len().div_ceil(width);
    let mut gmap = grid(width, height)?;
    for (i, name) in names.iter().enumerate() {
        if name != "-" {
            gmap.names[i] = name.clone();
        }
    }
    Ok(Built {
        gmap,
        notes: Vec::new(),
    })
}

/// Column letters to a 0-based index: `a` 0, `z` 25, `aa` 26, `ab` 27.
fn column(letters: &str) -> Option<usize> {
    let mut n = 0usize;
    for c in letters.chars() {
        let d = (c.to_ascii_lowercase() as u8).checked_sub(b'a')? as usize;
        if d > 25 {
            return None;
        }
        n = n.checked_mul(26)?.checked_add(d + 1)?;
    }
    n.checked_sub(1)
}

/// The ways to read `name` as `<prefix><column><row>`: the column is the
/// last one or two letters before the row digits, so `worldaa12.nw` reads
/// as `world` column `aa` and as `worlda` column `a`. [`from_names`] keeps
/// the reading whose prefix most names share.
fn split_name(name: &str) -> Vec<(String, usize, usize)> {
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
    let digits = stem.len() - stem.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return Vec::new();
    }
    let (head, row) = stem.split_at(stem.len() - digits);
    let Ok(row) = row.parse::<usize>() else {
        return Vec::new();
    };
    let letters = head.len()
        - head
            .trim_end_matches(|c: char| c.is_ascii_alphabetic())
            .len();
    (1..=letters.min(2))
        .filter_map(|n| {
            let (prefix, col) = head.split_at(head.len() - n);
            Some((prefix.to_ascii_lowercase(), column(col)?, row))
        })
        .collect()
}

/// Place levels by their names (`<prefix><column><row>`). The prefix most
/// names share is the world; the smallest row and column found become the
/// grid's top-left. Names that do not fit are listed in `notes`.
pub fn from_names(names: &[String]) -> Result<Built, String> {
    let mut notes = Vec::new();
    let readings: Vec<_> = names.iter().map(|n| (n, split_name(n))).collect();
    let mut prefixes: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, r) in &readings {
        for (p, _, _) in r {
            *prefixes.entry(p.as_str()).or_default() += 1;
        }
    }
    let Some((prefix, _)) = prefixes.iter().max_by_key(|(_, n)| **n) else {
        return Err("no level name has the <name><column><row> form".into());
    };
    let prefix = prefix.to_string();
    let mut placed: Vec<(usize, usize, String)> = Vec::new();
    for (name, r) in &readings {
        match r.iter().find(|(p, _, _)| *p == prefix) {
            Some((_, c, row)) => placed.push((*c, *row, (*name).clone())),
            None if r.is_empty() => {
                notes.push(format!("{name}: not a <name><column><row> level name"))
            }
            None => notes.push(format!("{name}: another world's name (not \"{prefix}\")")),
        }
    }
    let min_c = placed.iter().map(|p| p.0).min().unwrap_or(0);
    let min_r = placed.iter().map(|p| p.1).min().unwrap_or(0);
    let width = placed.iter().map(|p| p.0 - min_c + 1).max().unwrap_or(0);
    let height = placed.iter().map(|p| p.1 - min_r + 1).max().unwrap_or(0);
    let mut gmap = grid(width, height)?;
    placed.sort();
    for (c, r, name) in placed {
        let cell = &mut gmap.names[(r - min_r) * width + (c - min_c)];
        if cell.is_empty() {
            *cell = name;
        } else {
            notes.push(format!("{name}: its cell already holds {cell}"));
        }
    }
    Ok(Built { gmap, notes })
}

/// Which side of a 64 by 64 level an edge link sits on, as a cell step.
///
/// A link is an edge link when its box touches that edge and is at most two
/// tiles deep there (a full-width warp area is not). A link that touches two
/// edges at once (a corner) is not used.
pub fn edge_step(link: &Link) -> Option<(i32, i32)> {
    let thin_x = link.width <= 2;
    let thin_y = link.height <= 2;
    let left = link.x <= 0 && thin_x;
    let right = link.x + link.width >= 64 && link.x >= 62 && thin_x;
    let top = link.y <= 0 && thin_y;
    let bottom = link.y + link.height >= 64 && link.y >= 62 && thin_y;
    match (left, right, top, bottom) {
        (true, false, false, false) => Some((-1, 0)),
        (false, true, false, false) => Some((1, 0)),
        (false, false, true, false) => Some((0, -1)),
        (false, false, false, true) => Some((0, 1)),
        _ => None,
    }
}

/// Walk edge links from `start`. `load` returns a level by name (as links
/// name it), or `None` when it is missing; a missing neighbour still gets
/// its cell, and is noted. Conflicts (two levels claiming one cell, one
/// level reached at two cells) keep the first and are noted.
pub fn from_links(
    start: &str,
    mut load: impl FnMut(&str) -> Option<Level>,
) -> Result<Built, String> {
    let key = |n: &str| n.to_ascii_lowercase();
    let mut notes = Vec::new();
    let mut at: HashMap<(i32, i32), String> = HashMap::new();
    let mut cell_of: HashMap<String, (i32, i32)> = HashMap::new();
    let mut queue = VecDeque::new();
    at.insert((0, 0), start.to_string());
    cell_of.insert(key(start), (0, 0));
    queue.push_back(start.to_string());
    while let Some(name) = queue.pop_front() {
        let here = cell_of[&key(&name)];
        let Some(level) = load(&name) else {
            notes.push(format!("{name}: not found, kept as a cell without links"));
            continue;
        };
        for link in &level.links {
            let Some((dx, dy)) = edge_step(link) else {
                continue;
            };
            let cell = (here.0 + dx, here.1 + dy);
            match (cell_of.get(&key(&link.dest)), at.get(&cell)) {
                (Some(&known), _) if known == cell => {}
                (Some(&known), _) => notes.push(format!(
                    "{name} -> {}: already placed at {},{}; this link puts it at {},{}",
                    link.dest, known.0, known.1, cell.0, cell.1
                )),
                (None, Some(other)) => notes.push(format!(
                    "{name} -> {}: cell {},{} already holds {other}",
                    link.dest, cell.0, cell.1
                )),
                (None, None) => {
                    at.insert(cell, link.dest.clone());
                    cell_of.insert(key(&link.dest), cell);
                    queue.push_back(link.dest.clone());
                }
            }
        }
        if at.len() > MAX_SIDE * MAX_SIDE {
            return Err("the linked world is larger than a gmap can hold".into());
        }
    }
    let min_x = at.keys().map(|c| c.0).min().unwrap_or(0);
    let min_y = at.keys().map(|c| c.1).min().unwrap_or(0);
    let width = (at.keys().map(|c| c.0).max().unwrap_or(0) - min_x + 1) as usize;
    let height = (at.keys().map(|c| c.1).max().unwrap_or(0) - min_y + 1) as usize;
    let mut gmap = grid(width, height)?;
    for ((x, y), name) in at {
        gmap.names[(y - min_y) as usize * width + (x - min_x) as usize] = name;
    }
    Ok(Built { gmap, notes })
}

/// `GRMAP001` text: `WIDTH`, `HEIGHT`, the `LEVELNAMES` grid (each name
/// quoted, `""` for a hole), and the reader keys `Gmap` keeps when set.
pub fn write(gmap: &Gmap) -> String {
    let mut out = String::from("GRMAP001\n");
    out.push_str(&format!("WIDTH {}\nHEIGHT {}\n", gmap.width, gmap.height));
    if let Some(img) = &gmap.map_img {
        out.push_str(&format!("MAPIMG {img}\n"));
    }
    if let Some(img) = &gmap.minimap_img {
        out.push_str(&format!("MINIMAPIMG {img}\n"));
    }
    if gmap.no_auto_mapping {
        out.push_str("NOAUTOMAPPING\n");
    }
    if gmap.load_full_map {
        out.push_str("LOADFULLMAP\n");
    }
    out.push_str("LEVELNAMES\n");
    for row in gmap.names.chunks(gmap.width.max(1)) {
        let cells: Vec<String> = row.iter().map(|n| format!("\"{n}\"")).collect();
        out.push_str(&cells.join(","));
        out.push_str(",\n");
    }
    out.push_str("LEVELNAMESEND\n");
    if !gmap.load_at_start.is_empty() {
        out.push_str("LOADATSTART\n");
        let cells: Vec<String> = gmap
            .load_at_start
            .iter()
            .map(|n| format!("\"{n}\""))
            .collect();
        out.push_str(&cells.join(","));
        out.push_str("\nLOADATSTARTEND\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn list_fills_rows_and_round_trips_through_the_reader() {
        let b = from_list(&names(&["a.nw", "b.nw", "-", "d.nw", "e.nw"]), 2).unwrap();
        assert_eq!((b.gmap.width, b.gmap.height), (2, 3));
        let text = write(&b.gmap);
        let back = Gmap::parse(&text).unwrap();
        assert_eq!(back.names, names(&["a.nw", "b.nw", "", "d.nw", "e.nw", ""]));
        assert_eq!((back.width, back.height), (2, 3));
    }

    #[test]
    fn column_letters_count_like_spreadsheet_columns() {
        assert_eq!(column("a"), Some(0));
        assert_eq!(column("Z"), Some(25));
        assert_eq!(column("aa"), Some(26));
        assert_eq!(column("ab"), Some(27));
        assert_eq!(split_name("world-b07.nw"), vec![("world-".into(), 1, 7)]);
        assert_eq!(
            split_name("worldAA12.nw"),
            vec![("worlda".into(), 0, 12), ("world".into(), 26, 12)]
        );
        assert!(split_name("house.nw").is_empty());
    }

    #[test]
    fn names_place_by_column_and_row_and_report_strays() {
        let b = from_names(&names(&[
            "w-a01.nw", "w-b01.nw", "w-a02.nw", "w-c02.nw", "house.nw", "x-a01.nw",
        ]))
        .unwrap();
        assert_eq!((b.gmap.width, b.gmap.height), (3, 2));
        assert_eq!(
            b.gmap.names,
            names(&["w-a01.nw", "w-b01.nw", "", "w-a02.nw", "", "w-c02.nw"])
        );
        assert_eq!(b.notes.len(), 2, "{:?}", b.notes);
    }

    fn level_with_links(links: &[(&str, i32, i32, i32, i32)]) -> Level {
        let mut text = String::from("GLEVNW01\n");
        for (dest, x, y, w, h) in links {
            text.push_str(&format!("LINK {dest} {x} {y} {w} {h} playerx playery\n"));
        }
        Level::parse(&text).unwrap()
    }

    #[test]
    fn two_letter_columns_follow_the_shared_prefix() {
        let b = from_names(&names(&["mapy1.nw", "mapz1.nw", "mapaa1.nw"])).unwrap();
        assert_eq!(b.gmap.names, names(&["mapy1.nw", "mapz1.nw", "mapaa1.nw"]));
        assert!(b.notes.is_empty(), "{:?}", b.notes);
    }

    #[test]
    fn edge_steps() {
        let l = level_with_links(&[
            ("w.nw", 0, 0, 1, 64),
            ("e.nw", 63, 0, 1, 64),
            ("n.nw", 0, 0, 64, 1),
            ("s.nw", 0, 63, 64, 1),
            ("door.nw", 30, 30, 2, 1),
            ("big.nw", 0, 0, 64, 64),
        ]);
        let steps: Vec<_> = l.links.iter().map(edge_step).collect();
        assert_eq!(
            steps,
            vec![
                Some((-1, 0)),
                Some((1, 0)),
                Some((0, -1)),
                Some((0, 1)),
                None,
                None
            ]
        );
    }

    #[test]
    fn links_walk_a_two_by_two_world_and_note_conflicts() {
        let mut world: HashMap<&str, Level> = HashMap::new();
        world.insert(
            "a.nw",
            level_with_links(&[("b.nw", 63, 0, 1, 64), ("c.nw", 0, 63, 64, 1)]),
        );
        world.insert(
            "b.nw",
            level_with_links(&[("a.nw", 0, 0, 1, 64), ("d.nw", 0, 63, 64, 1)]),
        );
        // c says its east neighbour is x.nw, but d already holds that cell.
        world.insert(
            "c.nw",
            level_with_links(&[("a.nw", 0, 0, 64, 1), ("x.nw", 63, 0, 1, 64)]),
        );
        world.insert("d.nw", level_with_links(&[("b.nw", 0, 0, 64, 1)]));
        let b = from_links("a.nw", |n| world.get(n).cloned()).unwrap();
        assert_eq!((b.gmap.width, b.gmap.height), (2, 2));
        assert_eq!(b.gmap.names, names(&["a.nw", "b.nw", "c.nw", "d.nw"]));
        assert_eq!(b.notes.len(), 1, "{:?}", b.notes);
        assert!(b.notes[0].contains("x.nw"));
    }

    #[test]
    fn links_shift_the_grid_when_the_start_is_not_top_left() {
        let mut world: HashMap<&str, Level> = HashMap::new();
        world.insert("b.nw", level_with_links(&[("a.nw", 0, 0, 1, 64)]));
        let b = from_links("b.nw", |n| world.get(n).cloned()).unwrap();
        assert_eq!(b.gmap.names, names(&["a.nw", "b.nw"]));
        assert_eq!(b.notes.len(), 1, "a.nw is missing: {:?}", b.notes);
    }
}
