//! Levels to and from Tiled's `.tmx` maps: the job of NW2TMX and
//! nw-converter, so a level can be edited in Tiled and brought back.
//!
//! A level becomes a 64 by 64 map of 16-pixel tiles with one tileset, the
//! level's tile sheet (`pics1.png`, 2048 by 512, 128 columns). Graal tile
//! `n` is the sheet cell [`seihai_draw::tile_cell`] gives, and its gid is
//! that cell's index plus 1. Each board layer is a tile layer named
//! `layer N`; a cell a layer above 0 leaves empty is gid 0.
//!
//! Links, signs, NPCs, baddies and chests become objects (`links`, `signs`,
//! `npcs`, `baddies`, `chests`) at their place, each carrying its own
//! `GLEVNW01` lines in an `nw` property. Reading a map back takes the
//! tiles from the layers and everything else from those `nw` properties,
//! so moving an object in Tiled does not move it in the level: edit its
//! `nw` text for that. Reading accepts what Tiled writes for such a map:
//! CSV layer data and one inline tileset (any positive `firstgid`);
//! other encodings and multiple/external tilesets are refused with a
//! message. The sheet must keep its 128 columns of 16-pixel tiles and
//! 2048 by 512 image, without spacing or margins. Nonzero layer/group
//! positions and offsets are refused rather than silently discarded.
//! TMX is Tiled's format, not Graal's, so it lives
//! here and not in seihai-core.

use seihai_core::assets::{tile_code, BoardRun, Level};
use seihai_draw::{tile_cell, SHEET_COLS, SHEET_ROWS, TILE_PX};

const SIDE: usize = 64;

/// The gid of Graal tile `id` (first gid 1).
pub fn gid(id: u16) -> u32 {
    let (c, r) = tile_cell(id);
    r * SHEET_COLS + c + 1
}

/// The Graal tile of sheet cell `index` (gid minus the first gid).
pub fn tile_of_cell(index: u32) -> Option<u16> {
    let (c, r) = (index % SHEET_COLS, index / SHEET_COLS);
    if r >= SHEET_ROWS {
        return None;
    }
    u16::try_from((c / 16) * 512 + r * 16 + c % 16).ok()
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';') else {
            break;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => ent
                .strip_prefix("#x")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// A level with only the given parts, as `GLEVNW01` lines (no header).
fn nw_lines(part: Level) -> String {
    let text = String::from_utf8_lossy(&crate::level::encode(&part)).into_owned();
    text.strip_prefix("GLEVNW01\n").unwrap_or(&text).to_string()
}

fn empty() -> Level {
    Level {
        boards: Vec::new(),
        npcs: Vec::new(),
        links: Vec::new(),
        signs: Vec::new(),
        baddies: Vec::new(),
        chests: Vec::new(),
    }
}

/// Every layer's 64 by 64 board, `None` where the layer has no tile.
fn layers(level: &Level) -> Vec<(i32, Vec<Option<u16>>)> {
    let mut ids: Vec<i32> = level.boards.iter().map(|r| r.layer).collect();
    ids.push(0);
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .map(|layer| {
            let mut board = vec![None; SIDE * SIDE];
            // The first run wins where runs overlap, as the reader does.
            for run in level.boards.iter().rev().filter(|r| r.layer == layer) {
                if !(0..SIDE as i32).contains(&run.y) {
                    continue;
                }
                for (i, t) in run.tiles.iter().enumerate() {
                    let x = run.x + i as i32;
                    if (0..SIDE as i32).contains(&x) {
                        board[run.y as usize * SIDE + x as usize] =
                            Some(seihai_core::assets::tile_id(*t));
                    }
                }
            }
            if layer == 0 {
                for t in &mut board {
                    t.get_or_insert(0);
                }
            }
            (layer, board)
        })
        .collect()
}

/// `level` as a `.tmx` map whose tileset image is `sheet` (a path Tiled
/// resolves next to the map, such as `pics1.png`).
pub fn to_tmx(level: &Level, sheet: &str) -> String {
    let px = TILE_PX;
    let layers = layers(level);
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let groups = 5;
    out.push_str(&format!(
        "<map version=\"1.10\" orientation=\"orthogonal\" renderorder=\"right-down\" width=\"{SIDE}\" height=\"{SIDE}\" tilewidth=\"{px}\" tileheight=\"{px}\" infinite=\"0\" nextlayerid=\"{}\" nextobjectid=\"{}\">\n",
        layers.len() + groups + 1,
        level.links.len()
            + level.signs.len()
            + level.npcs.len()
            + level.baddies.len()
            + level.chests.len()
            + 1
    ));
    out.push_str(&format!(
        " <tileset firstgid=\"1\" name=\"pics1\" tilewidth=\"{px}\" tileheight=\"{px}\" tilecount=\"{}\" columns=\"{SHEET_COLS}\">\n  <image source=\"{}\" width=\"{}\" height=\"{}\"/>\n </tileset>\n",
        SHEET_COLS * SHEET_ROWS,
        esc(sheet),
        SHEET_COLS * px,
        SHEET_ROWS * px
    ));
    let mut layer_id = 0;
    for (layer, board) in &layers {
        layer_id += 1;
        out.push_str(&format!(
            " <layer id=\"{layer_id}\" name=\"layer {layer}\" width=\"{SIDE}\" height=\"{SIDE}\">\n  <data encoding=\"csv\">\n"
        ));
        let rows: Vec<String> = board
            .chunks(SIDE)
            .map(|row| {
                row.iter()
                    .map(|t| t.map_or(0, gid).to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect();
        out.push_str(&rows.join(",\n"));
        out.push_str("\n</data>\n </layer>\n");
    }
    let mut object_id = 0;
    let mut group = |out: &mut String,
                     name: &str,
                     items: Vec<(f32, f32, f32, f32, String, Level)>| {
        layer_id += 1;
        out.push_str(&format!(
            " <objectgroup id=\"{layer_id}\" name=\"{name}\">\n"
        ));
        for (x, y, w, h, label, part) in items {
            object_id += 1;
            let p = px as f32;
            let size = if w > 0.0 && h > 0.0 {
                format!(" width=\"{}\" height=\"{}\"", w * p, h * p)
            } else {
                String::new()
            };
            out.push_str(&format!(
                "  <object id=\"{object_id}\" name=\"{}\" x=\"{}\" y=\"{}\"{size}>\n   <properties>\n    <property name=\"nw\">{}</property>\n   </properties>\n  </object>\n",
                esc(&label),
                x * p,
                y * p,
                esc(&nw_lines(part))
            ));
        }
        out.push_str(" </objectgroup>\n");
    };
    let one = |f: &dyn Fn(&mut Level)| {
        let mut l = empty();
        f(&mut l);
        l
    };
    group(
        &mut out,
        "links",
        level
            .links
            .iter()
            .map(|k| {
                (
                    k.x as f32,
                    k.y as f32,
                    k.width as f32,
                    k.height as f32,
                    k.dest.clone(),
                    one(&|l| l.links.push(k.clone())),
                )
            })
            .collect(),
    );
    group(
        &mut out,
        "signs",
        level
            .signs
            .iter()
            .map(|s| {
                let first = s.text.lines().next().unwrap_or("").to_string();
                (
                    s.x as f32,
                    s.y as f32,
                    2.0,
                    1.0,
                    first,
                    one(&|l| l.signs.push(s.clone())),
                )
            })
            .collect(),
    );
    group(
        &mut out,
        "npcs",
        level
            .npcs
            .iter()
            .map(|n| {
                (
                    n.x,
                    n.y,
                    0.0,
                    0.0,
                    n.image.clone(),
                    one(&|l| l.npcs.push(n.clone())),
                )
            })
            .collect(),
    );
    group(
        &mut out,
        "baddies",
        level
            .baddies
            .iter()
            .map(|b| {
                (
                    b.x,
                    b.y,
                    0.0,
                    0.0,
                    b.kind.clone(),
                    one(&|l| l.baddies.push(b.clone())),
                )
            })
            .collect(),
    );
    group(
        &mut out,
        "chests",
        level
            .chests
            .iter()
            .map(|c| {
                (
                    c.x as f32,
                    c.y as f32,
                    2.0,
                    2.0,
                    c.item.clone(),
                    one(&|l| l.chests.push(c.clone())),
                )
            })
            .collect(),
    );
    out.push_str("</map>\n");
    out
}

/// One XML tag or the text before it.
enum Piece<'a> {
    Open {
        name: &'a str,
        attrs: Vec<(&'a str, String)>,
        closed: bool,
    },
    Close(&'a str),
    Text(&'a str),
}

fn pieces(xml: &str) -> Result<Vec<Piece<'_>>, String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            out.push(Piece::Text(rest));
            break;
        };
        if lt > 0 {
            out.push(Piece::Text(&rest[..lt]));
        }
        rest = &rest[lt..];
        if rest.starts_with("<!--") {
            let end = rest.find("-->").ok_or("an unclosed comment")?;
            rest = &rest[end + 3..];
            continue;
        }
        if rest.starts_with("<![CDATA[") {
            return Err("CDATA is not read".into());
        }
        let gt = rest.find('>').ok_or("an unclosed tag")?;
        let tag = &rest[1..gt];
        rest = &rest[gt + 1..];
        if tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            out.push(Piece::Close(name.trim()));
            continue;
        }
        let closed = tag.ends_with('/');
        let tag = tag.trim_end_matches('/');
        let name_end = tag.find(char::is_whitespace).unwrap_or(tag.len());
        let name = &tag[..name_end];
        let mut attrs = Vec::new();
        let mut a = tag[name_end..].trim_start();
        while !a.is_empty() {
            let eq = a
                .find('=')
                .ok_or_else(|| format!("<{name}>: an attribute with no value"))?;
            let key = a[..eq].trim();
            let after = a[eq + 1..].trim_start();
            let q = after.chars().next().ok_or("an attribute with no value")?;
            if q != '"' && q != '\'' {
                return Err(format!("<{name}>: an unquoted attribute"));
            }
            let close = after[1..]
                .find(q)
                .ok_or_else(|| format!("<{name}>: an unclosed attribute"))?;
            attrs.push((key, unesc(&after[1..1 + close])));
            a = after[close + 2..].trim_start();
        }
        out.push(Piece::Open {
            name,
            attrs,
            closed,
        });
    }
    Ok(out)
}

fn attr<'a>(attrs: &'a [(&str, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.as_str())
}

fn zero_attrs(attrs: &[(&str, String)], keys: &[&str], element: &str) -> Result<(), String> {
    for key in keys {
        if let Some(value) = attr(attrs, key) {
            if value.parse::<f64>().ok() != Some(0.0) {
                return Err(format!(
                    "<{element}>: nonzero or invalid {key} is not supported"
                ));
            }
        }
    }
    Ok(())
}

/// A level from a `.tmx` map written by [`to_tmx`] or by Tiled after
/// editing one. The warnings name what was skipped.
pub fn from_tmx(xml: &str) -> Result<(Level, Vec<String>), String> {
    let pieces = pieces(xml)?;
    // Refuse incomplete documents before returning a level that the CLI
    // could save over an existing file.
    let mut stack = Vec::new();
    let mut maps = 0;
    let mut sheet_images = 0;
    for piece in &pieces {
        match piece {
            Piece::Open {
                name,
                attrs,
                closed,
            } => {
                if stack.is_empty() {
                    if *name != "map" || maps != 0 {
                        return Err("expected one map root".into());
                    }
                    maps += 1;
                }
                if matches!(*name, "layer" | "group" | "objectgroup") {
                    zero_attrs(attrs, &["x", "y", "offsetx", "offsety"], name)?;
                }
                if *name == "tileoffset" {
                    zero_attrs(attrs, &["x", "y"], name)?;
                }
                if *name == "image" && stack.last() == Some(&"tileset") {
                    sheet_images += 1;
                    if attr(attrs, "width").and_then(|v| v.parse::<u32>().ok())
                        != Some(SHEET_COLS * TILE_PX)
                        || attr(attrs, "height").and_then(|v| v.parse::<u32>().ok())
                            != Some(SHEET_ROWS * TILE_PX)
                    {
                        return Err("the tileset image must be 2048 by 512 pixels".into());
                    }
                }
                if !closed {
                    stack.push(*name);
                }
            }
            Piece::Close(name) => {
                if stack.pop() != Some(*name) {
                    return Err(format!("unmatched closing tag </{name}>"));
                }
            }
            Piece::Text(text) if stack.is_empty() && !text.trim().is_empty() => {
                return Err("text outside the map".into());
            }
            _ => {}
        }
    }
    if maps != 1 || !stack.is_empty() {
        return Err("missing or unclosed map".into());
    }
    if sheet_images != 1 {
        return Err("use one tileset image with explicit dimensions".into());
    }
    let mut warnings = Vec::new();
    let mut firstgid: Option<u32> = None;
    let mut boards: Vec<(i32, Vec<u32>)> = Vec::new();
    let mut nw = String::new();
    let mut layer: Option<i32> = None;
    let mut in_data = false;
    let mut in_nw = false;
    let mut data = String::new();
    let mut seen_layers = 0;
    for p in &pieces {
        match p {
            Piece::Open {
                name,
                attrs,
                closed,
            } => match *name {
                "map" => {
                    let size = (attr(attrs, "width"), attr(attrs, "height"));
                    if size != (Some("64"), Some("64")) {
                        return Err("a level map is 64 by 64 tiles".into());
                    }
                    if attr(attrs, "infinite") == Some("1") {
                        return Err("an infinite map is not read; turn it into a fixed one".into());
                    }
                    if attr(attrs, "orientation") != Some("orthogonal")
                        || attr(attrs, "tilewidth") != Some("16")
                        || attr(attrs, "tileheight") != Some("16")
                    {
                        return Err("a level map must be orthogonal with 16 by 16 tiles".into());
                    }
                }
                "tileset" => {
                    let gid = attr(attrs, "firstgid")
                        .and_then(|v| v.parse().ok())
                        .ok_or("a tileset with no firstgid")?;
                    if firstgid.is_some() {
                        return Err("more than one tileset is not supported".into());
                    }
                    if gid == 0 || attr(attrs, "source").is_some() {
                        return Err("use one inline tileset with a positive firstgid".into());
                    }
                    for (key, expected) in [
                        ("tilewidth", TILE_PX),
                        ("tileheight", TILE_PX),
                        ("columns", SHEET_COLS),
                        ("tilecount", SHEET_COLS * SHEET_ROWS),
                    ] {
                        if attr(attrs, key).and_then(|v| v.parse::<u32>().ok()) != Some(expected) {
                            return Err(format!("tileset {key} must be {expected}"));
                        }
                    }
                    zero_attrs(attrs, &["spacing", "margin"], "tileset")?;
                    firstgid = Some(gid);
                }
                "layer" => {
                    if attr(attrs, "width") != Some("64") || attr(attrs, "height") != Some("64") {
                        return Err("a tile layer must be 64 by 64 tiles".into());
                    }
                    seen_layers += 1;
                    let name = attr(attrs, "name").unwrap_or("");
                    layer = Some(
                        name.strip_prefix("layer ")
                            .and_then(|n| n.trim().parse().ok())
                            .unwrap_or(seen_layers - 1),
                    );
                }
                "data" => {
                    if !closed {
                        match attr(attrs, "encoding") {
                            Some("csv") => {}
                            other => {
                                return Err(format!(
                                "layer data in {} is not read; save the map with CSV layer format",
                                other.unwrap_or("XML")
                            ))
                            }
                        }
                        in_data = true;
                        data.clear();
                    }
                }
                "property" if attr(attrs, "name") == Some("nw") => {
                    if let Some(v) = attr(attrs, "value") {
                        nw.push_str(v);
                        if !v.ends_with('\n') {
                            nw.push('\n');
                        }
                    } else if !closed {
                        in_nw = true;
                    }
                }
                _ => {}
            },
            Piece::Text(t) => {
                if in_data {
                    data.push_str(t);
                } else if in_nw {
                    let v = unesc(t);
                    nw.push_str(&v);
                    if !v.ends_with('\n') {
                        nw.push('\n');
                    }
                }
            }
            Piece::Close(name) => match *name {
                "data" if in_data => {
                    in_data = false;
                    let cells: Result<Vec<u32>, String> = data
                        .split(',')
                        .map(str::trim)
                        .filter(|c| !c.is_empty())
                        .map(|c| c.parse().map_err(|_| format!("{c} is not a tile")))
                        .collect();
                    let cells = cells?;
                    if cells.len() != SIDE * SIDE {
                        return Err(format!("a layer has {} cells, not 4096", cells.len()));
                    }
                    boards.push((layer.unwrap_or(0), cells));
                }
                "property" => in_nw = false,
                _ => {}
            },
        }
    }
    if boards.is_empty() || boards.len() != seen_layers as usize {
        return Err("each tile layer must contain CSV data".into());
    }
    let first = firstgid.ok_or("the map has no tileset")?;
    let mut level = if nw.trim().is_empty() {
        empty()
    } else {
        Level::parse(&format!("GLEVNW01\n{nw}"))?
    };
    level.boards.retain(|_| false);
    let mut odd = 0;
    for (layer, cells) in boards {
        for (y, row) in cells.chunks(SIDE).enumerate() {
            // One run per stretch of set cells.
            let mut x = 0;
            while x < SIDE {
                let tile = |c: u32| {
                    // Flip bits (Tiled's top three) are dropped.
                    let g = c & 0x1FFF_FFFF;
                    if g == 0 {
                        return None;
                    }
                    g.checked_sub(first).and_then(tile_of_cell)
                };
                if tile(row[x]).is_none() {
                    if row[x] != 0 {
                        odd += 1;
                    }
                    x += 1;
                    continue;
                }
                let start = x;
                let mut tiles = Vec::new();
                while x < SIDE {
                    match tile(row[x]) {
                        Some(t) => tiles.push(tile_code(t)),
                        None => break,
                    }
                    x += 1;
                }
                level.boards.push(BoardRun {
                    x: start as i32,
                    y: y as i32,
                    width: tiles.len() as i32,
                    layer,
                    tiles,
                });
            }
        }
    }
    if odd > 0 {
        warnings.push(format!(
            "{odd} cells name tiles that are not on the tile sheet; left empty"
        ));
    }
    Ok((level, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVEL: &str = "GLEVNW01
BOARD 0 0 3 0 ABACAD
BOARD 5 2 2 1 BAgB
LINK next.nw 0 0 1 4 30 playery
SIGN 2 3
Hello & <welcome>
SIGNEND
NPC door.png 10 11.5
if (playertouchsme) { message \"a, b\"; }
NPCEND
";

    #[test]
    fn gids_match_the_sheet_and_come_back() {
        assert_eq!(gid(0), 1);
        assert_eq!(gid(16), 129, "tile 16 is the next row");
        assert_eq!(gid(512), 17, "tile 512 is the next 16-column block");
        for id in [0u16, 1, 15, 16, 511, 512, 2047, 4095] {
            assert_eq!(tile_of_cell(gid(id) - 1), Some(id));
        }
        assert_eq!(tile_of_cell(SHEET_COLS * SHEET_ROWS), None);
    }

    #[test]
    fn refuses_incomplete_maps_and_unsupported_tilesets() {
        let tmx = to_tmx(&Level::parse(LEVEL).unwrap(), "pics1.png");
        for bad in [
            tmx.replace("</map>", ""),
            tmx.replace("</data>", "</wrong>"),
            tmx.replace("<data encoding=\"csv\">", "<data encoding=\"csv\"/><!--")
                .replace("</data>", "-->"),
            tmx.replace("</map>", "<tileset firstgid=\"1000\"/></map>"),
            tmx.replace("firstgid=\"1\"", "firstgid=\"0\""),
            tmx.replace("firstgid=\"1\"", "firstgid=\"1\" source=\"other.tsx\""),
            tmx.replace("orientation=\"orthogonal\"", "orientation=\"isometric\""),
            tmx.replace("tilewidth=\"16\"", "tilewidth=\"32\""),
            tmx.replace("width=\"64\"", "width=\"32\""),
        ] {
            assert!(from_tmx(&bad).is_err(), "unsupported map was accepted");
        }
    }

    #[test]
    fn refuses_sheet_layout_changes_and_layer_transforms() {
        let tmx = to_tmx(&Level::parse(LEVEL).unwrap(), "pics1.png");
        for (old, new) in [
            ("columns=\"128\"", "columns=\"64\""),
            ("tilecount=\"4096\"", "tilecount=\"2048\""),
            (
                "name=\"pics1\" tilewidth=\"16\"",
                "name=\"pics1\" tilewidth=\"32\"",
            ),
            (
                "name=\"pics1\" tilewidth=\"16\"",
                "name=\"pics1\" tilewidth=\"16\" margin=\"1\"",
            ),
            (
                "name=\"pics1\" tilewidth=\"16\"",
                "name=\"pics1\" tilewidth=\"16\" spacing=\"2\"",
            ),
            ("width=\"2048\"", "width=\"1024\""),
            ("height=\"512\"", "height=\"1024\""),
            ("</tileset>", "<tileoffset x=\"1\" y=\"0\"/></tileset>"),
        ] {
            assert!(tmx.contains(old), "fixture must exercise {old}");
            assert!(from_tmx(&tmx.replace(old, new)).is_err(), "accepted {new}");
        }
        for element in ["layer", "objectgroup", "group"] {
            for key in ["x", "y", "offsetx", "offsety"] {
                let changed = if element == "group" {
                    tmx.replace("</map>", &format!("<group {key}=\"1\"/></map>"))
                } else {
                    tmx.replace(&format!("<{element} "), &format!("<{element} {key}=\"1\" "))
                };
                assert!(from_tmx(&changed).is_err(), "accepted {element} {key}");
            }
        }
        let zero = tmx
            .replace(
                "name=\"pics1\"",
                "name=\"pics1\" spacing=\"0\" margin=\"0\"",
            )
            .replace(
                "<layer ",
                "<layer x=\"0\" y=\"0\" offsetx=\"0.0\" offsety=\"-0\" ",
            )
            .replace("</map>", "<group offsetx=\"0\"/></map>");
        assert!(from_tmx(&zero).is_ok());
    }

    #[test]
    fn a_level_round_trips_through_tmx() {
        let level = Level::parse(LEVEL).unwrap();
        let tmx = to_tmx(&level, "pics1.png");
        assert!(tmx.contains("<layer id=\"1\" name=\"layer 0\""));
        assert!(tmx.contains("name=\"layer 1\""));
        assert!(tmx.contains("<image source=\"pics1.png\" width=\"2048\" height=\"512\"/>"));
        assert!(tmx.contains("name=\"next.nw\" x=\"0\" y=\"0\" width=\"16\" height=\"64\""));
        let (back, warnings) = from_tmx(&tmx).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(back.tile_ids(), level.tile_ids());
        let upper = |l: &Level| {
            l.boards
                .iter()
                .filter(|r| r.layer == 1)
                .flat_map(|r| r.tiles.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(upper(&back), upper(&level));
        assert_eq!(back.links.len(), 1);
        assert_eq!(back.signs[0].text, level.signs[0].text);
        assert_eq!(back.npcs[0].script, level.npcs[0].script);
        assert_eq!((back.npcs[0].x, back.npcs[0].y), (10.0, 11.5));
    }

    #[test]
    fn edits_in_tiled_come_back_and_odd_maps_are_refused() {
        let level = Level::parse(LEVEL).unwrap();
        let tmx = to_tmx(&level, "pics1.png");
        // Tiled painted tile 16 at (0, 0): gid 129 (tile 1 was gid 2).
        let edited = tmx.replacen(
            "<data encoding=\"csv\">\n2,",
            "<data encoding=\"csv\">\n129,",
            1,
        );
        let (back, _) = from_tmx(&edited).unwrap();
        assert_eq!(back.tile_ids()[0], 16);
        let b64 = tmx.replace("encoding=\"csv\"", "encoding=\"base64\"");
        assert!(from_tmx(&b64).unwrap_err().contains("CSV"));
        let small = tmx.replacen(
            "width=\"64\" height=\"64\"",
            "width=\"10\" height=\"10\"",
            1,
        );
        assert!(from_tmx(&small).is_err());
        // A second firstgid moves every gid.
        let shifted = tmx.replace("firstgid=\"1\"", "firstgid=\"5\"").replacen(
            "<data encoding=\"csv\">\n2,3,4,",
            "<data encoding=\"csv\">\n6,7,8,",
            1,
        );
        let (back, warnings) = from_tmx(&shifted).unwrap();
        assert_eq!(&back.tile_ids()[..3], &[1, 2, 3]);
        assert!(
            warnings[0].contains("not on the tile sheet"),
            "{warnings:?}"
        );
    }
}
