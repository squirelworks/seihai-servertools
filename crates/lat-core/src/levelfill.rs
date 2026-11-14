//! LevelFiller: scatter a theme's tile objects (flowers, bushes, rocks) over
//! `GLEVNW01` levels, and LFThemeMaker's "X on Y" patch themes. Behaviour
//! notes: Graal_analysis `tools/levelfiller.md` (a static teardown of the
//! 2001 v0.6 build and the v0.7 manual).
//!
//! What is here: `.nwt` themes (read and write), the quantity forms, the
//! placement rules, complex objects through [`crate::lfscript`], the level
//! text kept as LevelFiller keeps it, and `.tm` recipes. Not yet: border
//! objects (`.top=` …, kept but not drawn) and "fill across levels".
//!
//! `.nwt` and `.tm` are LevelFiller's own files, not Graal formats, so they
//! are read here and not in seihai-core. The level board goes through
//! seihai-core's `Level` and its tile codes.

use crate::generate::DelphiRandom;
use crate::lfscript::{self, Built, Stamp};
use seihai_core::assets::{tile_code, tile_id, Level};

/// A level is 64 cells a side.
const SIDE: usize = 64;

/// How many placements a rule tries.
#[derive(Debug, Clone, PartialEq)]
pub enum Quantity {
    /// `n`: try `n` placements.
    Count(u32),
    /// `p%`: a share of the level's cells; each try uses up the object's area.
    Percent(f64),
    /// `r(a,b)` or `r(a,b)%`: a random whole number from `a` to `b`.
    Random { low: i64, high: i64, percent: bool },
    /// `c(p%,n)`: `n` rolls at `p`% each; the number of hits.
    Chance { percent: f64, rolls: u32 },
}

impl Quantity {
    /// Read a quantity as a theme writes it.
    pub fn parse(text: &str) -> Result<Self, String> {
        let t = text.trim();
        let bad = || format!("{t:?}: not a quantity (n, p%, r(a,b), c(p%,n))");
        let args = |inner: &str| -> Vec<String> {
            inner.split(',').map(|s| s.trim().to_string()).collect()
        };
        let lower = t.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("r(") {
            let (inner, tail) = rest.split_once(')').ok_or_else(bad)?;
            let a = args(inner);
            let [low, high] = a.as_slice() else {
                return Err(bad());
            };
            let percent = match tail.trim() {
                "" => false,
                "%" => true,
                _ => return Err(bad()),
            };
            let (low, high) = (
                low.parse().map_err(|_| bad())?,
                high.parse().map_err(|_| bad())?,
            );
            return Ok(Self::Random { low, high, percent });
        }
        if let Some(rest) = lower.strip_prefix("c(") {
            let inner = rest.strip_suffix(')').ok_or_else(bad)?;
            let a = args(inner);
            let [p, n] = a.as_slice() else {
                return Err(bad());
            };
            let p = p
                .strip_suffix('%')
                .ok_or("Parameter is not a percentage.")?
                .trim()
                .parse()
                .map_err(|_| bad())?;
            return Ok(Self::Chance {
                percent: p,
                rolls: n.parse().map_err(|_| bad())?,
            });
        }
        if let Some(p) = t.strip_suffix('%') {
            return Ok(Self::Percent(p.trim().parse().map_err(|_| bad())?));
        }
        Ok(Self::Count(t.parse().map_err(|_| bad())?))
    }

    /// The amount to use up, and whether each try costs the object's area
    /// (a `%` form) instead of 1.
    fn amount(&self, cells: usize, rng: &mut DelphiRandom) -> (f64, bool) {
        let share = |p: f64| (cells as f64 * p / 100.0).round();
        match *self {
            Self::Count(n) => (f64::from(n), false),
            Self::Percent(p) => (share(p), true),
            Self::Random { low, high, percent } => {
                let (lo, hi) = (low.min(high), low.max(high));
                let span = (hi - lo + 1).clamp(1, i64::from(u32::MAX)) as u32;
                let v = (lo + i64::from(rng.below(span))) as f64;
                if percent {
                    (share(v), true)
                } else {
                    (v, false)
                }
            }
            Self::Chance { percent, rolls } => {
                let hits = (0..rolls)
                    .filter(|_| f64::from(rng.below(10_000)) < percent * 100.0)
                    .count();
                (hits as f64, false)
            }
        }
    }

    fn write(&self) -> String {
        match *self {
            Self::Count(n) => n.to_string(),
            Self::Percent(p) => format!("{p}%"),
            Self::Random { low, high, percent } => {
                format!("r({low},{high}){}", if percent { "%" } else { "" })
            }
            Self::Chance { percent, rolls } => format!("c({percent}%,{rolls})"),
        }
    }
}

/// `name=quantity[|tile]` in `[rules]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub object: String,
    pub quantity: Quantity,
    /// The tile the object may cover; `None` uses the theme's.
    pub replace: Option<u16>,
}

/// One object of `[objects]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub name: String,
    /// `<name>*`: the body is object-language script, not tile rows.
    pub complex: bool,
    /// Dotted properties (`.author=`, `.top=` …), without the dot.
    pub props: Vec<(String, String)>,
    pub width: usize,
    pub height: usize,
    /// Row by row; `None` is a transparent cell.
    pub cells: Vec<Option<u16>>,
    /// A complex object's script, `#l` / `#g` already turned back into `<` / `>`.
    pub script: String,
}

impl Object {
    /// A plain object from its rows.
    pub fn from_rows(name: &str, rows: &[Vec<Option<u16>>]) -> Self {
        let width = rows.iter().map(Vec::len).max().unwrap_or(0);
        let mut cells = Vec::with_capacity(width * rows.len());
        for r in rows {
            cells.extend(r.iter().copied());
            cells.extend(std::iter::repeat_n(None, width - r.len()));
        }
        Self {
            name: name.to_string(),
            complex: false,
            props: Vec::new(),
            width,
            height: rows.len(),
            cells,
            script: String::new(),
        }
    }
}

/// A `.nwt` theme.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Theme {
    pub name: String,
    pub author: String,
    /// `repbase=`: the level's background tile to fill over.
    pub repbase: Option<u16>,
    /// `base=`: the background a preview level is made of.
    pub base: Option<u16>,
    pub rules: Vec<Rule>,
    pub objects: Vec<Object>,
}

fn code(text: &str) -> Result<u16, String> {
    match text.trim().as_bytes() {
        [a, b] => Ok(tile_id([*a, *b])),
        _ => Err(format!("{text:?}: a tile is two characters, like AA")),
    }
}

fn row_cells(line: &str) -> Vec<Option<u16>> {
    line.as_bytes()
        .chunks(2)
        .map(|p| match p {
            [b' ', b' '] | [b' '] => None,
            [a, b] => Some(tile_id([*a, *b])),
            [a] => Some(tile_id([*a, b'A'])),
            _ => None,
        })
        .collect()
}

impl Theme {
    /// Read a `.nwt` file's text.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut theme = Theme::default();
        let mut section = String::new();
        let mut current: Option<(Object, Vec<Vec<Option<u16>>>)> = None;
        let finish = |cur: Option<(Object, Vec<Vec<Option<u16>>>)>, out: &mut Vec<Object>| {
            if let Some((mut obj, rows)) = cur {
                if !obj.complex {
                    let plain = Object::from_rows(&obj.name, &rows);
                    (obj.width, obj.height, obj.cells) = (plain.width, plain.height, plain.cells);
                }
                out.push(obj);
            }
        };
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim_end_matches('\r');
            let at = |m: String| format!("line {}: {m}", n + 1);
            let trimmed = line.trim();
            if let Some(s) = trimmed
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .filter(|s| !s.contains(['<', '>']))
            {
                section = s.to_ascii_lowercase();
                continue;
            }
            match section.as_str() {
                "theme" | "rules" => {
                    if trimmed.is_empty() || trimmed.starts_with(';') {
                        continue;
                    }
                    let (k, v) = trimmed
                        .split_once('=')
                        .ok_or_else(|| at(format!("{trimmed:?}: expected key=value")))?;
                    let (k, v) = (k.trim(), v.trim());
                    match (section.as_str(), k.to_ascii_lowercase().as_str()) {
                        ("theme", "name") => theme.name = v.to_string(),
                        ("theme", "author") => theme.author = v.to_string(),
                        ("theme", _) => {}
                        ("rules", "repbase") => theme.repbase = Some(code(v).map_err(at)?),
                        ("rules", "base") => theme.base = Some(code(v).map_err(at)?),
                        _ => {
                            let (q, tile) = match v.split_once('|') {
                                Some((q, t)) => (q, Some(code(t).map_err(at)?)),
                                None => (v, None),
                            };
                            theme.rules.push(Rule {
                                object: k.to_string(),
                                quantity: Quantity::parse(q).map_err(at)?,
                                replace: tile,
                            });
                        }
                    }
                }
                "objects" => {
                    if let Some(name) = trimmed.strip_prefix('<').and_then(|s| {
                        s.strip_suffix(">*")
                            .map(|s| (s, true))
                            .or(s.strip_suffix('>').map(|s| (s, false)))
                    }) {
                        finish(current.take(), &mut theme.objects);
                        let (name, complex) = name;
                        current = Some((
                            Object {
                                name: name.to_string(),
                                complex,
                                props: Vec::new(),
                                width: 0,
                                height: 0,
                                cells: Vec::new(),
                                script: String::new(),
                            },
                            Vec::new(),
                        ));
                        continue;
                    }
                    let Some((obj, rows)) = current.as_mut() else {
                        continue;
                    };
                    if let Some(prop) = trimmed.strip_prefix('.').filter(|p| p.contains('=')) {
                        let (k, v) = prop.split_once('=').unwrap_or((prop, ""));
                        obj.props.push((k.trim().to_string(), v.trim().to_string()));
                    } else if obj.complex {
                        let text = line.replace("#l", "<").replace("#g", ">");
                        obj.script.push_str(&text);
                        obj.script.push('\n');
                    } else if !line.is_empty() {
                        rows.push(row_cells(line));
                    }
                }
                _ => {}
            }
        }
        finish(current.take(), &mut theme.objects);
        Ok(theme)
    }

    /// The object called `name` (names are case sensitive).
    pub fn object(&self, name: &str) -> Option<&Object> {
        self.objects.iter().find(|o| o.name == name)
    }

    /// The theme as `.nwt` text (LF line ends).
    pub fn write(&self) -> String {
        let t = |id: u16| String::from_utf8_lossy(&tile_code(id)).into_owned();
        let mut out = format!(
            "[theme]\nname={}\nauthor={}\n\n[rules]\n",
            self.name, self.author
        );
        if let Some(r) = self.repbase {
            out.push_str(&format!("repbase={}\n", t(r)));
        }
        if let Some(b) = self.base {
            out.push_str(&format!("base={}\n", t(b)));
        }
        for r in &self.rules {
            out.push_str(&format!("{}={}", r.object, r.quantity.write()));
            if let Some(tile) = r.replace {
                out.push_str(&format!("|{}", t(tile)));
            }
            out.push('\n');
        }
        out.push_str("\n[objects]\n");
        for o in &self.objects {
            out.push_str(&format!(
                "<{}>{}\n",
                o.name,
                if o.complex { "*" } else { "" }
            ));
            for (k, v) in &o.props {
                out.push_str(&format!(".{k}={v}\n"));
            }
            if o.complex {
                out.push_str(&o.script.replace('<', "#l").replace('>', "#g"));
                continue;
            }
            for row in o.cells.chunks(o.width.max(1)) {
                for c in row {
                    out.push_str(&c.map_or("  ".to_string(), t));
                }
                out.push('\n');
            }
        }
        out
    }
}

/// A level as LevelFiller holds it: the 64 by 64 layer-0 board, and every
/// other line kept as it was.
#[derive(Debug, Clone, PartialEq)]
pub struct FillLevel {
    pub board: Vec<u16>,
    tail: Vec<String>,
}

impl FillLevel {
    /// Read a `GLEVNW01` level's text.
    pub fn parse(text: &str) -> Result<Self, String> {
        let level = Level::parse(text)?;
        let mut tail = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let line = line.trim_end_matches('\r');
            if i == 0 && line.starts_with("GLEVNW01") {
                continue;
            }
            let w: Vec<&str> = line.split_whitespace().collect();
            if w.first() == Some(&"BOARD") && w.get(4) == Some(&"0") {
                continue;
            }
            tail.push(line.to_string());
        }
        Ok(Self {
            board: level.tile_ids(),
            tail,
        })
    }

    /// `GLEVNW01`, the 64 board rows, then the kept lines.
    pub fn write(&self) -> String {
        let mut out = String::from("GLEVNW01\n");
        for (y, row) in self.board.chunks(SIDE).enumerate() {
            out.push_str(&format!("BOARD 0 {y} 64 0 "));
            for &t in row {
                out.push_str(&String::from_utf8_lossy(&tile_code(t)));
            }
            out.push('\n');
        }
        for line in &self.tail {
            out.push_str(line);
            out.push('\n');
        }
        out
    }
}

/// How a fill runs.
#[derive(Debug, Clone)]
pub struct FillOptions {
    /// Pick positions only where the whole object fits in the level
    /// (LevelFiller's "keep inside the level"). Without it an object may
    /// be tried over the edge, where it never fits.
    pub keep_inside: bool,
}

/// What a fill did, per rule, for the log.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FillReport {
    /// `(object, placed, tried)`.
    pub placed: Vec<(String, u32, u32)>,
    pub notes: Vec<String>,
    /// What complex objects' `debug` and `msg` said.
    pub log: Vec<String>,
}

/// Fill `level` with `theme`'s rules, in order.
pub fn fill(
    level: &mut FillLevel,
    theme: &Theme,
    rng: &mut DelphiRandom,
    opts: &FillOptions,
) -> FillReport {
    let mut report = FillReport::default();
    for rule in &theme.rules {
        let Some(obj) = theme.object(&rule.object) else {
            report
                .notes
                .push(format!("{}: no such object", rule.object));
            continue;
        };
        let Some(replace) = rule.replace.or(theme.repbase).or(theme.base) else {
            report.notes.push(format!(
                "{}: no tile to replace (no |tile, repbase or base)",
                obj.name
            ));
            continue;
        };
        let (mut left, by_area) = rule.quantity.amount(SIDE * SIDE, rng);
        let (mut placed, mut tried) = (0, 0);
        while left > 0.0 {
            tried += 1;
            // A complex object builds a new picture for every placement.
            let stamp = match resolve(theme, obj, rng, &mut report.log, 0) {
                Ok(s) => s,
                Err(e) => {
                    report
                        .notes
                        .push(format!("{}: {e}; rule stopped", obj.name));
                    break;
                }
            };
            if place(level, &stamp, replace, rng, opts) {
                placed += 1;
            }
            left -= if by_area {
                (stamp.width * stamp.height).max(1) as f64
            } else {
                1.0
            };
        }
        report.placed.push((obj.name.clone(), placed, tried));
    }
    report
}

/// The picture `obj` places: its tiles, or what its script builds (a
/// `return` followed to another object or a tile, at most 8 deep).
fn resolve(
    theme: &Theme,
    obj: &Object,
    rng: &mut DelphiRandom,
    log: &mut Vec<String>,
    depth: usize,
) -> Result<Stamp, String> {
    if !obj.complex {
        return Ok(Stamp {
            width: obj.width,
            height: obj.height,
            cells: obj.cells.clone(),
        });
    }
    if depth >= 8 {
        return Err("return goes more than 8 objects deep".into());
    }
    let lookup = |name: &str| {
        theme.object(name).filter(|o| !o.complex).map(|o| Stamp {
            width: o.width,
            height: o.height,
            cells: o.cells.clone(),
        })
    };
    match lfscript::build(&obj.script, rng, &lookup, log)? {
        Built::Stamp(s) => Ok(s),
        Built::Tile(t) => Ok(Stamp {
            width: 1,
            height: 1,
            cells: vec![Some(t)],
        }),
        Built::Return(name) => {
            let next = theme
                .object(&name)
                .ok_or(format!("return({name}): no such object"))?;
            resolve(theme, next, rng, log, depth + 1)
        }
    }
}

/// One placement: up to three positions, the first where every cell of the
/// object's box holds `replace`.
fn place(
    level: &mut FillLevel,
    obj: &Stamp,
    replace: u16,
    rng: &mut DelphiRandom,
    opts: &FillOptions,
) -> bool {
    let (w, h) = (obj.width, obj.height);
    if w == 0 || h == 0 {
        return false;
    }
    let (span_x, span_y) = if opts.keep_inside {
        (SIDE.saturating_sub(w) + 1, SIDE.saturating_sub(h) + 1)
    } else {
        (SIDE, SIDE)
    };
    for _ in 0..3 {
        let row = rng.below(span_y as u32) as usize;
        let col = rng.below(span_x as u32) as usize;
        if row + h > SIDE || col + w > SIDE {
            continue;
        }
        let fits =
            (0..h).all(|dy| (0..w).all(|dx| level.board[(row + dy) * SIDE + col + dx] == replace));
        if !fits {
            continue;
        }
        for dy in 0..h {
            for dx in 0..w {
                if let Some(t) = obj.cells[dy * w + dx] {
                    level.board[(row + dy) * SIDE + col + dx] = t;
                }
            }
        }
        return true;
    }
    false
}

/// An LFThemeMaker recipe (`.tm`).
#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    /// `Tile00` … `Tile15`, by corner mask (top-left 1, top-right 2,
    /// bottom-right 4, bottom-left 8). `Tile00` is the tile patches go on.
    pub tiles: [u16; 16],
    pub width: (usize, usize),
    pub height: (usize, usize),
    /// Share of the inside corners, in percent.
    pub density: (u32, u32),
    pub base_name: String,
    pub count: usize,
    pub quantity: String,
    pub description: String,
}

impl Recipe {
    /// Read a `.tm` file's text.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut map = std::collections::HashMap::new();
        for line in text.lines() {
            let l = line.trim();
            if l.starts_with('[') || l.starts_with(';') {
                continue;
            }
            if let Some((k, v)) = l.split_once('=') {
                map.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
            }
        }
        let get = |k: &str| map.get(k).cloned().ok_or(format!("the recipe has no {k}"));
        let num = |k: &str| -> Result<usize, String> {
            get(k)?.parse().map_err(|_| format!("{k}: not a number"))
        };
        let mut tiles = [0u16; 16];
        for (i, t) in tiles.iter_mut().enumerate() {
            *t = code(&get(&format!("tile{i:02}"))?)?;
        }
        let clamp = |v: usize| v.clamp(3, 32);
        Ok(Self {
            tiles,
            width: (clamp(num("widthmin")?), clamp(num("widthmax")?)),
            height: (clamp(num("heightmin")?), clamp(num("heightmax")?)),
            density: (num("densitymin")? as u32, num("densitymax")? as u32),
            base_name: get("baseobjectname")?,
            count: num("#ofobjects")?,
            quantity: get("qtyperobject").unwrap_or_else(|_| "c(5%,1)".into()),
            description: get("description").unwrap_or_default(),
        })
    }
}

fn between(rng: &mut DelphiRandom, (a, b): (usize, usize)) -> usize {
    let (lo, hi) = (a.min(b), a.max(b));
    lo + rng.below((hi - lo + 1) as u32) as usize
}

/// LFThemeMaker: `count` random patches, each a blob of "inside" corners
/// grown from the middle of a grid of the chosen size until it holds the
/// chosen density, every cell drawn as `tiles[mask]` of its four corners
/// (mask 0 left transparent). The growth rule is this port's own; the
/// original's blob code was not traced.
pub fn make_theme(recipe: &Recipe, seed: u32) -> Result<Theme, String> {
    let mut rng = DelphiRandom(seed);
    let quantity = Quantity::parse(&recipe.quantity)?;
    let mut theme = Theme {
        name: recipe.description.clone(),
        author: "LFThemeMaker recipe".into(),
        base: Some(recipe.tiles[0]),
        ..Theme::default()
    };
    for i in 0..recipe.count {
        let (w, h) = (
            between(&mut rng, recipe.width),
            between(&mut rng, recipe.height),
        );
        let density = between(
            &mut rng,
            (recipe.density.0 as usize, recipe.density.1 as usize),
        )
        .clamp(1, 100);
        // Corners (w + 1) by (h + 1); the outer ring stays outside so every
        // patch closes.
        let (cw, ch) = (w + 1, h + 1);
        let mut inside = vec![false; cw * ch];
        let inner = (cw - 2) * (ch - 2);
        let want = (inner * density).div_ceil(100).max(1);
        let mut grown = vec![(cw / 2, ch / 2)];
        inside[(ch / 2) * cw + cw / 2] = true;
        let mut guard = 0;
        while grown.len() < want && guard < inner * 50 {
            guard += 1;
            let (x, y) = grown[rng.below(grown.len() as u32) as usize];
            let (nx, ny) = match rng.below(4) {
                0 => (x + 1, y),
                1 => (x.wrapping_sub(1), y),
                2 => (x, y + 1),
                _ => (x, y.wrapping_sub(1)),
            };
            if nx == 0 || ny == 0 || nx >= cw - 1 || ny >= ch - 1 || inside[ny * cw + nx] {
                continue;
            }
            inside[ny * cw + nx] = true;
            grown.push((nx, ny));
        }
        let corner = |x: usize, y: usize| inside[y * cw + x];
        let rows: Vec<Vec<Option<u16>>> = (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| {
                        let mask = usize::from(corner(x, y))
                            | usize::from(corner(x + 1, y)) << 1
                            | usize::from(corner(x + 1, y + 1)) << 2
                            | usize::from(corner(x, y + 1)) << 3;
                        (mask != 0).then_some(recipe.tiles[mask])
                    })
                    .collect()
            })
            .collect();
        let name = format!("{}{}", recipe.base_name, i + 1);
        theme.objects.push(Object::from_rows(&name, &rows));
        theme.rules.push(Rule {
            object: name,
            quantity: quantity.clone(),
            replace: Some(recipe.tiles[0]),
        });
    }
    Ok(theme)
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEME: &str = "[theme]\r\nname=Test\r\nauthor=us\r\n\r\n[rules]\r\nbase=AA\r\n;flowers first\r\nflower=10\r\nrock=1%|AA\r\nfancy=r(1,2)\r\n\r\n[objects]\r\n<flower>\r\n.author=us\r\nAB\r\n<rock>\r\nACAC\r\nAC  \r\n<fancy>*\r\nsetsize(2,2);\r\nif (1#g0) {add(*AB,1);}\r\n";

    fn grass() -> FillLevel {
        FillLevel::parse("GLEVNW01\nNPC - 1 1\nmessage;\nNPCEND\n").unwrap()
    }

    #[test]
    fn reads_quantities() {
        assert_eq!(Quantity::parse("12").unwrap(), Quantity::Count(12));
        assert_eq!(Quantity::parse("2.5%").unwrap(), Quantity::Percent(2.5));
        assert_eq!(
            Quantity::parse("r(1, 4)%").unwrap(),
            Quantity::Random {
                low: 1,
                high: 4,
                percent: true
            }
        );
        assert_eq!(
            Quantity::parse("c(5%,3)").unwrap(),
            Quantity::Chance {
                percent: 5.0,
                rolls: 3
            }
        );
        assert_eq!(
            Quantity::parse("c(5,3)").unwrap_err(),
            "Parameter is not a percentage."
        );
        assert!(Quantity::parse("lots").is_err());
    }

    #[test]
    fn reads_and_writes_a_theme() {
        let t = Theme::parse(THEME).unwrap();
        assert_eq!((t.name.as_str(), t.base), ("Test", Some(0)));
        assert_eq!(t.rules.len(), 3);
        assert_eq!(t.rules[1].replace, Some(0));
        let rock = t.object("rock").unwrap();
        assert_eq!((rock.width, rock.height), (2, 2));
        assert_eq!(rock.cells, [Some(2), Some(2), Some(2), None]);
        assert_eq!(
            t.object("flower").unwrap().props,
            [("author".into(), "us".into())]
        );
        let fancy = t.object("fancy").unwrap();
        assert!(fancy.complex && fancy.script.contains("if (1>0)"));
        assert_eq!(Theme::parse(&t.write()).unwrap(), t);
    }

    #[test]
    fn fills_only_over_the_replace_tile_and_keeps_the_tail() {
        let theme = Theme::parse(THEME).unwrap();
        let mut level = grass();
        let report = fill(
            &mut level,
            &theme,
            &mut DelphiRandom(7),
            &FillOptions { keep_inside: true },
        );
        assert_eq!(report.placed[0].0, "flower");
        assert_eq!(report.placed[0].2, 10, "ten tries");
        assert!(report.placed[0].1 >= 1);
        // 1% of 4096 cells is 41, each rock try costs its 4 cells: 11 tries.
        assert_eq!(report.placed[1].2, 11);
        // The complex object builds a 2 x 2 with one flower and is placed.
        assert_eq!(report.placed[2].0, "fancy");
        assert!(report.notes.is_empty(), "{:?}", report.notes);
        let flowers = level.board.iter().filter(|&&t| t == 1).count();
        assert_eq!(flowers as u32, report.placed[0].1 + report.placed[2].1);
        let text = level.write();
        assert!(text.starts_with("GLEVNW01\nBOARD 0 0 64 0 "));
        assert!(text.ends_with("NPC - 1 1\nmessage;\nNPCEND\n"));
        let again = FillLevel::parse(&text).unwrap();
        assert_eq!(again, level);
        // Nothing fits on a level with no replace tile left.
        let mut full = level.clone();
        full.board.iter_mut().for_each(|t| *t = 9);
        let r = fill(
            &mut full,
            &theme,
            &mut DelphiRandom(1),
            &FillOptions { keep_inside: true },
        );
        assert!(r.placed.iter().all(|p| p.1 == 0));
    }

    #[test]
    fn makes_closed_patches_from_a_recipe() {
        let mut tm = String::from("[Tiles]\n");
        for i in 0..16 {
            tm.push_str(&format!(
                "Tile{i:02}={}\n",
                String::from_utf8_lossy(&tile_code(100 + i))
            ));
        }
        tm.push_str("[Objects]\nWidthMin=4\nWidthMax=6\nHeightMin=4\nHeightMax=5\nDensityMin=30\nDensityMax=60\nBaseObjectName=dirt\n[Theme]\n#ofObjects=3\nQtyPerObject=c(5%,1)\nDescription=Dirt on grass\n");
        let recipe = Recipe::parse(&tm).unwrap();
        let theme = make_theme(&recipe, 3).unwrap();
        assert_eq!(theme.objects.len(), 3);
        assert_eq!(theme.rules[0].object, "dirt1");
        assert_eq!(theme.rules[0].replace, Some(100));
        for o in &theme.objects {
            assert!((4..=6).contains(&o.width) && (4..=5).contains(&o.height));
            // The outer ring of corners is outside: no full tile on the edge.
            let w = o.width;
            for x in 0..w {
                assert_ne!(o.cells[x], Some(115));
                assert_ne!(o.cells[(o.height - 1) * w + x], Some(115));
            }
            assert!(o.cells.iter().any(Option::is_some));
        }
        assert_eq!(Theme::parse(&theme.write()).unwrap(), theme);
    }
}
