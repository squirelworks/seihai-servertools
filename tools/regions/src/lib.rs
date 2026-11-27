//! `seihai-regions`: a big 3D overworld as a grid of regions for
//! seihai-gserver's `regions` world line (its `docs/GRAAL3D.md`, "A world
//! built from regions").
//!
//! A region is a folder `<i>-<j>` (i east, j north, from the south-west)
//! with its piece of the streamed terrain (`heights<x>-<y>.dat`, x and y
//! from 0 to the chunks a region less one) and a `region.txt` of object
//! lines in the region's own coordinates, from its south-west corner. The
//! server stitches them into one large terrain with distance scoping, so
//! the player walks across region edges with no switch.
//!
//! - `split` cuts an existing world with a large (or plain) terrain into
//!   regions and writes the world file that uses them.
//! - `stats` counts the objects of each region and the densest spot, the
//!   numbers that decide whether a client's 4096-entry ghost table holds.
//! - `demo` builds a test overworld: hills, open fields and dense towns.

use lat_core::pw3d::shape::{Colour, Shape};
use lat_core::terrain_chunks;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub const USAGE: &str = "\
seihai-regions: a big 3D overworld as a grid of regions (seihai-gserver's
`regions` world line).

regions split WORLD.txt --root DIR --into FOLDER [--chunks N] [--out FILE] [--force]
    Cuts WORLD.txt's terrain and objects into regions of N x N terrain
    chunks (2, 4, 8 or 16; default 8): DIR/FOLDER/<i>-<j>/ with its
    heights<x>-<y>.dat and a region.txt of its object and objectfor lines in
    the region's coordinates. The terrain line (`terrain F S repeat WxH`, or
    a plain 16 x 16 one) becomes `regions FOLDER CxR N S`; every other line
    (sun, sky, spawn, water, replicators, ...) stays in the world file, which
    goes to FILE (default WORLD-regions.txt beside WORLD.txt). DIR is the
    server's world root, which the folders in the world file are under.
regions stats WORLD.txt --root DIR [--chunks N] [--reach UNITS] [--budget N]
    The objects in each region and the most within REACH units (default
    1300, the server's scope radius plus hysteresis) of any spot, against
    the BUDGET (default 3584) a client can hold. Works on a world with a
    regions line or a terrain line (cut into N-chunk regions to count).
regions demo DIR [--regions CxR] [--chunks N] [--towns N] [--town-objects N]
            [--town-radius U] [--trees N] [--seed N] [--force]
    A test overworld as a gserver folder: DIR/config/graal3dworld.txt with
    a regions line, DIR/world/3d/regions/ with hilly terrain, a grass,
    rock, sand and path heights.info and textures, houses, trees and rocks
    (.3ds) in DIR/world/3d/. Default 8x8 regions of 8 chunks (8192 units a
    side), 6 towns of 1500 houses within 200 units of their centres, and
    150 trees a region. The spawn is in the first town.

Without --force, split and demo refuse to write over files.";

pub fn run(args: &[&str]) -> ExitCode {
    let r = match args {
        [] | ["--help" | "-h", ..] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        ["split", rest @ ..] => split_cmd(rest),
        ["stats", rest @ ..] => stats_cmd(rest),
        ["demo", rest @ ..] => demo_cmd(rest),
        _ => Err(format!("unknown command\n\n{USAGE}")),
    };
    match r {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("regions: {e}");
            ExitCode::from(2)
        }
    }
}

/// Words after a command: the plain ones, and each `--name value`.
struct Args {
    plain: Vec<String>,
    named: BTreeMap<String, String>,
    force: bool,
}

fn args(words: &[&str]) -> Result<Args, String> {
    let mut a = Args {
        plain: Vec::new(),
        named: BTreeMap::new(),
        force: false,
    };
    let mut it = words.iter();
    while let Some(w) = it.next() {
        if *w == "--force" {
            a.force = true;
        } else if let Some(name) = w.strip_prefix("--") {
            let v = it.next().ok_or(format!("--{name} needs a value"))?;
            a.named.insert(name.to_string(), v.to_string());
        } else {
            a.plain.push(w.to_string());
        }
    }
    Ok(a)
}

impl Args {
    fn num<T: std::str::FromStr>(&self, name: &str, default: T) -> Result<T, String> {
        match self.named.get(name) {
            Some(v) => v.parse().map_err(|_| format!("--{name} {v}: not a number")),
            None => Ok(default),
        }
    }
}

fn grid(v: &str) -> Result<(u32, u32), String> {
    let (a, b) = v
        .split_once(['x', 'X'])
        .ok_or(format!("{v}: use CxR, like 8x8"))?;
    Ok((
        a.parse().map_err(|_| format!("{v}: use CxR"))?,
        b.parse().map_err(|_| format!("{v}: use CxR"))?,
    ))
}

/// The region layout: the same rules as the server's `regions` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub cols: u32,
    pub rows: u32,
    pub chunks: u32,
    pub square: u32,
}

impl Layout {
    pub fn new(cols: u32, rows: u32, chunks: u32, square: u32) -> Result<Layout, String> {
        if ![2, 4, 8, 16].contains(&chunks) {
            return Err(format!(
                "a region is 2, 4, 8 or 16 chunks a side, not {chunks}"
            ));
        }
        for n in [cols * chunks, rows * chunks] {
            if !(16..=256).contains(&n) {
                return Err(format!(
                    "{cols}x{rows} regions of {chunks} chunks is {} x {} chunks; the server takes 16 to 256 a side",
                    cols * chunks,
                    rows * chunks
                ));
            }
        }
        if !(1..=64).contains(&square) {
            return Err(format!("square size {square}: 1 to 64"));
        }
        Ok(Layout {
            cols,
            rows,
            chunks,
            square,
        })
    }

    /// Units a chunk and a region side.
    pub fn chunk_span(&self) -> f32 {
        (16 * self.square) as f32
    }
    pub fn span(&self) -> f32 {
        self.chunks as f32 * self.chunk_span()
    }

    /// Region (i, j)'s south-west corner: the world is centred on 0, 0.
    pub fn corner(&self, i: u32, j: u32) -> [f32; 2] {
        let half = |n: u32| -(((n * self.chunks) / 2) as f32) * self.chunk_span();
        [
            half(self.cols) + i as f32 * self.span(),
            half(self.rows) + j as f32 * self.span(),
        ]
    }

    /// The region a point is in; points off the map go to the nearest one.
    pub fn region_of(&self, x: f32, y: f32) -> (u32, u32) {
        let [x0, y0] = self.corner(0, 0);
        let f = |v: f32, o: f32, n: u32| {
            ((v - o) / self.span()).floor().clamp(0.0, (n - 1) as f32) as u32
        };
        (f(x, x0, self.cols), f(y, y0, self.rows))
    }

    pub fn line(&self, folder: &str) -> String {
        format!(
            "regions {folder} {}x{} {} {}",
            self.cols, self.rows, self.chunks, self.square
        )
    }
}

/// Where a line's x and y are, by its first word.
fn xy_at(words: &[&str]) -> Option<usize> {
    match words.first()? {
        &"object" => Some(3),
        &"objectfor" => Some(4),
        _ => None,
    }
}

fn num(v: f32) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    if r == r.trunc() {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

/// A world file's `terrain` line: folder, square size, chunks a side.
fn terrain_line(text: &str) -> Option<(String, u32, (u32, u32))> {
    for line in text.lines() {
        let w: Vec<&str> = line.split('#').next()?.split_whitespace().collect();
        if w.first() != Some(&"terrain") || w.len() < 2 {
            continue;
        }
        let mut square = 8;
        let mut size = (16, 16);
        for word in &w[2..] {
            if let Some((a, b)) = word.split_once(['x', 'X']) {
                if let (Ok(a), Ok(b)) = (a.parse(), b.parse()) {
                    size = (a, b);
                }
            } else if let Ok(n) = word.parse() {
                square = n;
            }
        }
        return Some((w[1].to_string(), square, size));
    }
    None
}

/// What `split` writes: the files under the world root, and the new world
/// file's text.
pub struct Split {
    pub files: Vec<(PathBuf, Vec<u8>)>,
    pub world: String,
    pub layout: Layout,
    pub objects: Vec<usize>,
}

/// Cuts a world into regions of `chunks` chunks. `read` reads a file
/// under the world root (`None`: missing).
pub fn split(
    text: &str,
    into: &str,
    chunks: u32,
    read: &dyn Fn(&Path) -> Option<Vec<u8>>,
) -> Result<Split, String> {
    let (folder, square, (w, h)) =
        terrain_line(text).ok_or("the world has no terrain line to cut")?;
    if w % chunks != 0 || h % chunks != 0 {
        return Err(format!(
            "the terrain is {w} x {h} chunks, which {chunks}-chunk regions do not divide; try --chunks {}",
            [16, 8, 4, 2].into_iter().find(|c| w % c == 0 && h % c == 0).unwrap_or(2)
        ));
    }
    let layout = Layout::new(w / chunks, h / chunks, chunks, square)?;
    let into_dir = PathBuf::from(into);
    let mut files = Vec::new();
    for cy in 0..h {
        for cx in 0..w {
            let Some(data) = read(&Path::new(&folder).join(format!("heights{cx}-{cy}.dat"))) else {
                continue;
            };
            let name = format!("{}-{}", cx / chunks, cy / chunks);
            files.push((
                into_dir
                    .join(name)
                    .join(format!("heights{}-{}.dat", cx % chunks, cy % chunks)),
                data,
            ));
        }
    }
    // The terrain's materials and textures go with it.
    if let Some(info) = read(&Path::new(&folder).join("heights.info")) {
        for tex in String::from_utf8_lossy(&info)
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
        {
            if let Some(d) = read(&Path::new(&folder).join(tex)) {
                files.push((into_dir.join(tex), d));
            }
        }
        files.push((into_dir.join("heights.info"), info));
    }
    let mut region_text: BTreeMap<(u32, u32), String> = BTreeMap::new();
    let mut world = format!(
        "# Cut into regions by seihai-regions split ({} x {} regions of {chunks} chunks).\n",
        layout.cols, layout.rows
    );
    for line in text.lines() {
        let body = line.split('#').next().unwrap_or("");
        let words: Vec<&str> = body.split_whitespace().collect();
        if words.first() == Some(&"terrain") {
            world.push_str(&layout.line(into));
            world.push('\n');
            continue;
        }
        let Some(at) = xy_at(&words).filter(|at| words.len() > at + 2) else {
            world.push_str(line);
            world.push('\n');
            continue;
        };
        let (Ok(x), Ok(y)) = (words[at].parse::<f32>(), words[at + 1].parse::<f32>()) else {
            world.push_str(line);
            world.push('\n');
            continue;
        };
        let (i, j) = layout.region_of(x, y);
        let [x0, y0] = layout.corner(i, j);
        let mut out: Vec<String> = words.iter().map(|s| s.to_string()).collect();
        out[at] = num(x - x0);
        out[at + 1] = num(y - y0);
        let comment = line
            .find('#')
            .map(|k| format!(" {}", &line[k..]))
            .unwrap_or_default();
        let t = region_text.entry((i, j)).or_default();
        t.push_str(&out.join(" "));
        t.push_str(&comment);
        t.push('\n');
    }
    let mut objects = vec![0; (layout.cols * layout.rows) as usize];
    for ((i, j), t) in region_text {
        objects[(j * layout.cols + i) as usize] = t.lines().count();
        files.push((
            into_dir.join(format!("{i}-{j}")).join("region.txt"),
            t.into_bytes(),
        ));
    }
    Ok(Split {
        files,
        world,
        layout,
        objects,
    })
}

fn root_reader(root: &Path) -> impl Fn(&Path) -> Option<Vec<u8>> + '_ {
    move |p: &Path| {
        p.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
            .then(|| std::fs::read(root.join(p)).ok())
            .flatten()
    }
}

fn write_all(root: &Path, files: &[(PathBuf, Vec<u8>)], force: bool) -> Result<(), String> {
    if !force {
        if let Some((p, _)) = files.iter().find(|(p, _)| root.join(p).exists()) {
            return Err(format!(
                "{} exists; --force writes over it",
                root.join(p).display()
            ));
        }
    }
    for (p, data) in files {
        let path = root.join(p);
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        std::fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

fn split_cmd(words: &[&str]) -> Result<String, String> {
    let a = args(words)?;
    let [world_file] = a.plain.as_slice() else {
        return Err("split needs one WORLD.txt".into());
    };
    let root = PathBuf::from(a.named.get("root").ok_or("split needs --root DIR")?);
    let into = a.named.get("into").ok_or("split needs --into FOLDER")?;
    let chunks = a.num("chunks", 8u32)?;
    let text = std::fs::read_to_string(world_file).map_err(|e| format!("{world_file}: {e}"))?;
    let s = split(&text, into, chunks, &root_reader(&root))?;
    let out = a.named.get("out").map(PathBuf::from).unwrap_or_else(|| {
        let p = Path::new(world_file);
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("world");
        p.with_file_name(format!("{stem}-regions.txt"))
    });
    if out.exists() && !a.force {
        return Err(format!("{} exists; --force writes over it", out.display()));
    }
    write_all(&root, &s.files, a.force)?;
    std::fs::write(&out, &s.world).map_err(|e| format!("{}: {e}", out.display()))?;
    let total: usize = s.objects.iter().sum();
    Ok(format!(
        "{} x {} regions of {} units in {}: {} files, {total} objects; world file {}\n",
        s.layout.cols,
        s.layout.rows,
        s.layout.span(),
        root.join(into).display(),
        s.files.len(),
        out.display()
    ))
}

/// The most of `points` within `reach` of one spot (tried every quarter of
/// `reach`), and where.
pub fn densest(points: &[[f32; 2]], reach: f32) -> (usize, [f32; 2]) {
    if points.is_empty() || reach.is_nan() || reach <= 0.0 {
        return (0, [0.0; 2]);
    }
    let key = |v: f32| (v / reach).floor() as i64;
    let mut cells: BTreeMap<(i64, i64), Vec<[f32; 2]>> = BTreeMap::new();
    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
    for p in points {
        cells.entry((key(p[0]), key(p[1]))).or_default().push(*p);
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let step = reach / 4.0;
    let mut best = (0, [0.0; 2]);
    let mut y = lo[1];
    while y <= hi[1] + step {
        let mut x = lo[0];
        while x <= hi[0] + step {
            let mut n = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if let Some(ps) = cells.get(&(key(x) + dx, key(y) + dy)) {
                        n += ps
                            .iter()
                            .filter(|p| (p[0] - x).powi(2) + (p[1] - y).powi(2) <= reach * reach)
                            .count();
                    }
                }
            }
            if n > best.0 {
                best = (n, [x, y]);
            }
            x += step;
        }
        y += step;
    }
    best
}

/// The world-space x, y of every object line of a world: its own lines,
/// and with a regions line every region's, moved to where it lies.
pub fn world_objects(
    text: &str,
    chunks: u32,
    read: &dyn Fn(&Path) -> Option<Vec<u8>>,
) -> Result<(Layout, Vec<[f32; 2]>), String> {
    let points = |t: &str, dx: f32, dy: f32| -> Vec<[f32; 2]> {
        t.lines()
            .filter_map(|l| {
                let w: Vec<&str> = l.split('#').next()?.split_whitespace().collect();
                let at = xy_at(&w)?;
                Some([
                    w.get(at)?.parse::<f32>().ok()? + dx,
                    w.get(at + 1)?.parse::<f32>().ok()? + dy,
                ])
            })
            .collect()
    };
    let regions = text.lines().find_map(|l| {
        let w: Vec<&str> = l.split('#').next()?.split_whitespace().collect();
        (w.first() == Some(&"regions") && w.len() >= 3).then(|| {
            (
                w[1].to_string(),
                w[2].to_string(),
                w.get(3).and_then(|v| v.parse().ok()).unwrap_or(8u32),
                w.get(4).and_then(|v| v.parse().ok()).unwrap_or(8u32),
            )
        })
    });
    let mut all = points(text, 0.0, 0.0);
    let layout = match regions {
        Some((folder, g, chunks, square)) => {
            let (c, r) = grid(&g)?;
            let l = Layout::new(c, r, chunks, square)?;
            for j in 0..r {
                for i in 0..c {
                    if let Some(t) = read(
                        &Path::new(&folder)
                            .join(format!("{i}-{j}"))
                            .join("region.txt"),
                    ) {
                        let [x0, y0] = l.corner(i, j);
                        all.extend(points(&String::from_utf8_lossy(&t), x0, y0));
                    }
                }
            }
            l
        }
        None => {
            let (_, square, (w, h)) = terrain_line(text).unwrap_or((String::new(), 8, (16, 16)));
            if w % chunks != 0 || h % chunks != 0 {
                return Err(format!(
                    "{chunks}-chunk regions do not divide the {w} x {h} terrain"
                ));
            }
            Layout::new(w / chunks, h / chunks, chunks, square)?
        }
    };
    Ok((layout, all))
}

pub fn stats(
    text: &str,
    chunks: u32,
    reach: f32,
    budget: usize,
    read: &dyn Fn(&Path) -> Option<Vec<u8>>,
) -> Result<String, String> {
    let (l, pts) = world_objects(text, chunks, read)?;
    let mut counts = vec![0usize; (l.cols * l.rows) as usize];
    for p in &pts {
        let (i, j) = l.region_of(p[0], p[1]);
        counts[(j * l.cols + i) as usize] += 1;
    }
    let mut out = format!(
        "{} x {} regions of {} units ({} chunks each), {} objects\n\nobjects a region (north at the top, i across):\n",
        l.cols,
        l.rows,
        l.span(),
        l.chunks,
        pts.len()
    );
    for j in (0..l.rows).rev() {
        out.push_str(&format!("{j:>4} |"));
        for i in 0..l.cols {
            out.push_str(&format!(" {:>5}", counts[(j * l.cols + i) as usize]));
        }
        out.push('\n');
    }
    out.push_str("     +");
    for i in 0..l.cols {
        out.push_str(&format!(" {i:>5}"));
    }
    out.push('\n');
    let (n, at) = densest(&pts, reach);
    let (i, j) = l.region_of(at[0], at[1]);
    out.push_str(&format!(
        "\ndensest spot: about {n} objects within {reach} units of {:.0},{:.0} (region {i}-{j}); a client holds {budget}: {}\n",
        at[0],
        at[1],
        if n > budget {
            "OVER: a player there gets only the nearest; spread it out or make it its own world behind a warp"
        } else {
            "fits"
        }
    ));
    Ok(out)
}

fn stats_cmd(words: &[&str]) -> Result<String, String> {
    let a = args(words)?;
    let [world_file] = a.plain.as_slice() else {
        return Err("stats needs one WORLD.txt".into());
    };
    let root = PathBuf::from(a.named.get("root").ok_or("stats needs --root DIR")?);
    let text = std::fs::read_to_string(world_file).map_err(|e| format!("{world_file}: {e}"))?;
    let read = root_reader(&root);
    stats(
        &text,
        a.num("chunks", 8u32)?,
        a.num("reach", 1300.0f32)?,
        a.num("budget", 3584usize)?,
        &read,
    )
}

/// The demo's options.
#[derive(Debug, Clone)]
pub struct Demo {
    pub layout: Layout,
    pub towns: u32,
    pub town_objects: u32,
    pub town_radius: f32,
    pub trees: u32,
    pub seed: u32,
}

/// A small, fixed random sequence (xorshift), so a seed always gives the
/// same world.
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// The demo ground's height in units at x, y: rolling hills, flattened
/// round each town.
fn ground(x: f32, y: f32, towns: &[[f32; 2]], r: f32) -> f32 {
    let hills = 40.0
        + 30.0 * (x / 700.0).sin() * (y / 900.0).cos()
        + 12.0 * (x / 230.0 + 1.3).sin() * (y / 170.0 + 0.4).sin()
        + 4.0 * (x / 61.0).cos() * (y / 47.0).sin();
    let mut z = hills;
    for t in towns {
        // 0 in the town, 1 from 250 units past its edge: a level pad.
        let d = (x - t[0]).hypot(y - t[1]);
        let k = ((d - r * 1.1) / 250.0).clamp(0.0, 1.0);
        z = 45.0 * (1.0 - k) + z * k;
    }
    z
}

/// The demo's files under the gserver folder, and a one-line summary.
pub fn demo(o: &Demo) -> Result<(Vec<(PathBuf, Vec<u8>)>, String), String> {
    let l = o.layout;
    let mut rng = Rng(o.seed.wrapping_mul(2_654_435_761) ^ 0x9E37_79B9);
    for _ in 0..8 {
        rng.next();
    }
    let [wx0, wy0] = l.corner(0, 0);
    let (ww, wh) = (l.cols as f32 * l.span(), l.rows as f32 * l.span());
    // Towns away from the map's edge and from each other.
    let mut towns: Vec<[f32; 2]> = Vec::new();
    let margin = o.town_radius + 300.0;
    let mut tries = 0;
    while towns.len() < o.towns as usize && tries < 10_000 {
        tries += 1;
        let p = [
            wx0 + margin + rng.unit() * (ww - 2.0 * margin).max(1.0),
            wy0 + margin + rng.unit() * (wh - 2.0 * margin).max(1.0),
        ];
        if towns
            .iter()
            .all(|t| (t[0] - p[0]).hypot(t[1] - p[1]) > 4.0 * o.town_radius + 1400.0)
        {
            towns.push(p);
        }
    }
    let h = |x: f32, y: f32| ground(x, y, &towns, o.town_radius);
    let root = PathBuf::from("world/3d/regions");
    let mut files = Vec::new();
    let names = [
        "g3d_grass.png",
        "g3d_rock.png",
        "g3d_sand.png",
        "g3d_path.png",
    ];
    let colours = [
        [70, 140, 60],
        [125, 120, 110],
        [200, 185, 130],
        [150, 120, 85],
    ];
    for (n, c) in names.iter().zip(colours) {
        files.push((root.join(n), lat_core::terrain3d::texture_png(c)?));
    }
    files.push((
        root.join("heights.info"),
        (names.join("\n") + "\n").into_bytes(),
    ));
    let sq = l.square as f32;
    let (cw, chh) = (l.cols * l.chunks, l.rows * l.chunks);
    for cy in 0..chh {
        for cx in 0..cw {
            let mut c = terrain_chunks::Chunk {
                heights: [[0; 16]; 16],
                materials: [[0; 16]; 16],
                holes: [[false; 16]; 16],
            };
            for y in 0..16 {
                for x in 0..16 {
                    let px = wx0 + (cx * 16 + x as u32) as f32 * sq;
                    let py = wy0 + (cy * 16 + y as u32) as f32 * sq;
                    let z = h(px, py);
                    // Wire heights are 1/8 unit.
                    c.heights[y][x] = (z * 8.0).round().clamp(0.0, 16383.0) as u16;
                    let slope = (h(px + sq, py) - z).abs().max((h(px, py + sq) - z).abs()) / sq;
                    let in_town = towns
                        .iter()
                        .any(|t| (t[0] - px).hypot(t[1] - py) < o.town_radius * 1.1);
                    c.materials[y][x] = if in_town {
                        3
                    } else if slope > 0.6 || z > 75.0 {
                        1
                    } else if z < 22.0 {
                        2
                    } else {
                        0
                    };
                }
            }
            let z = terrain_chunks::compress(&terrain_chunks::encode_settled(&c));
            if z.len() > 1023 {
                return Err(format!(
                    "chunk {cx},{cy} compresses to {} bytes; an event holds 1023",
                    z.len()
                ));
            }
            let (i, j) = (cx / l.chunks, cy / l.chunks);
            files.push((
                root.join(format!("{i}-{j}")).join(format!(
                    "heights{}-{}.dat",
                    cx % l.chunks,
                    cy % l.chunks
                )),
                z,
            ));
        }
    }
    // Objects, in region coordinates.
    let mut region_text: BTreeMap<(u32, u32), String> = BTreeMap::new();
    let mut put = |name: String, shape: &str, x: f32, y: f32, turn: f32| {
        let (i, j) = l.region_of(x, y);
        let [x0, y0] = l.corner(i, j);
        let line = format!(
            "object {name} {shape} {} {} {} 0 0 1 {}\n",
            num(x - x0),
            num(y - y0),
            num(h(x, y)),
            num(turn)
        );
        region_text.entry((i, j)).or_default().push_str(&line);
    };
    let mut houses = 0;
    for (t, c) in towns.iter().enumerate() {
        // Houses on rings of plots, from the centre out, close enough
        // that the town's houses fit in its radius (4 to 12 units apart).
        let gap = (std::f32::consts::PI * o.town_radius * o.town_radius
            / o.town_objects.max(1) as f32)
            .sqrt()
            .clamp(4.0, 12.0);
        let mut k = 0;
        let mut ring = 1;
        while k < o.town_objects {
            let r = ring as f32 * gap;
            if r > o.town_radius {
                break;
            }
            let n = ((std::f32::consts::TAU * r) / gap).floor() as u32;
            for s in 0..n {
                if k >= o.town_objects {
                    break;
                }
                let a = std::f32::consts::TAU * s as f32 / n as f32;
                put(
                    format!("t{t}h{k}"),
                    "rg_house.3ds",
                    c[0] + r * a.cos(),
                    c[1] + r * a.sin(),
                    a,
                );
                k += 1;
                houses += 1;
            }
            ring += 1;
        }
    }
    let mut trees = 0;
    for j in 0..l.rows {
        for i in 0..l.cols {
            let [x0, y0] = l.corner(i, j);
            for k in 0..o.trees {
                let (x, y) = (x0 + rng.unit() * l.span(), y0 + rng.unit() * l.span());
                if towns
                    .iter()
                    .any(|t| (t[0] - x).hypot(t[1] - y) < o.town_radius * 1.3)
                {
                    continue;
                }
                let shape = if k % 5 == 0 {
                    "rg_rock.3ds"
                } else {
                    "rg_tree.3ds"
                };
                put(format!("r{i}_{j}t{k}"), shape, x, y, rng.unit() * 6.28);
                trees += 1;
            }
        }
    }
    for ((i, j), t) in region_text {
        files.push((
            root.join(format!("{i}-{j}")).join("region.txt"),
            t.into_bytes(),
        ));
    }
    // Shapes.
    let mut house = Shape::default();
    house.block(
        [-4.0, -3.0, 0.0],
        [4.0, 3.0, 4.0],
        Colour([200, 190, 170]),
        Colour([180, 160, 130]),
    );
    house.gable(
        [-4.3, -3.3, 4.0],
        [4.3, 3.3],
        Colour([150, 50, 40]),
        Colour([180, 160, 130]),
    );
    let mut tree = Shape::default();
    tree.block(
        [-0.4, -0.4, 0.0],
        [0.4, 0.4, 3.0],
        Colour([90, 60, 30]),
        Colour([80, 50, 25]),
    );
    tree.blob(
        [0.0, 0.0, 5.0],
        [2.5, 2.5, 3.0],
        Colour([40, 120, 40]),
        Colour([30, 100, 30]),
    );
    let mut rock = Shape::default();
    rock.blob(
        [0.0, 0.0, 0.5],
        [1.5, 1.2, 1.0],
        Colour([130, 130, 125]),
        Colour([110, 110, 105]),
    );
    for (n, s) in [("rg_house", house), ("rg_tree", tree), ("rg_rock", rock)] {
        files.push((
            PathBuf::from("world/3d").join(format!("{n}.3ds")),
            s.to_3ds(n),
        ));
    }
    let spawn = towns.first().copied().unwrap_or([wx0 + 20.0, wy0 + 20.0]);
    let (sx, sy) = (spawn[0] + o.town_radius + 40.0, spawn[1]);
    let mut world = format!(
        "# A test overworld from seihai-regions demo: {} x {} regions of {} units, {} towns of up to {} houses, {} trees and rocks.\n",
        l.cols,
        l.rows,
        l.span(),
        towns.len(),
        o.town_objects,
        trees
    );
    world.push_str(&l.line("world/3d/regions"));
    world.push('\n');
    world.push_str("sun 0.57 0.57 -0.57  0.85 0.85 0.8 1  0.35 0.35 0.4 1\n");
    world.push_str("playershape rg_rock.3ds\n");
    world.push_str(&format!(
        "spawn {} {} {}\n",
        num(sx),
        num(sy),
        num(h(sx, sy) + 1.0)
    ));
    for (t, c) in towns.iter().enumerate() {
        world.push_str(&format!("# town {t} at {:.0},{:.0}\n", c[0], c[1]));
    }
    files.push((PathBuf::from("config/graal3dworld.txt"), world.into_bytes()));
    let summary = format!(
        "{} x {} regions ({} units a side), {} terrain chunks, {} towns, {houses} houses, {trees} trees and rocks\n",
        l.cols,
        l.rows,
        l.span() * l.cols as f32,
        cw * chh,
        towns.len()
    );
    Ok((files, summary))
}

fn demo_cmd(words: &[&str]) -> Result<String, String> {
    let a = args(words)?;
    let [dir] = a.plain.as_slice() else {
        return Err("demo needs one DIR".into());
    };
    let (c, r) = grid(a.named.get("regions").map_or("8x8", String::as_str))?;
    let o = Demo {
        layout: Layout::new(c, r, a.num("chunks", 8)?, 8)?,
        towns: a.num("towns", 6)?,
        town_objects: a.num("town-objects", 1500)?,
        town_radius: a.num("town-radius", 200.0)?,
        trees: a.num("trees", 150)?,
        seed: a.num("seed", 1)?,
    };
    let (files, summary) = demo(&o)?;
    write_all(Path::new(dir), &files, a.force)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_the_server() {
        let l = Layout::new(4, 2, 8, 8).unwrap();
        assert_eq!(l.span(), 1024.0);
        assert_eq!(l.corner(0, 0), [-2048.0, -1024.0]);
        assert_eq!(l.corner(3, 1), [1024.0, 0.0]);
        assert_eq!(l.region_of(10.0, 10.0), (2, 1));
        assert_eq!(l.region_of(-99999.0, 99999.0), (0, 1));
        assert!(Layout::new(1, 4, 8, 8).is_err());
        assert!(Layout::new(4, 4, 3, 8).is_err());
    }

    #[test]
    fn the_densest_spot() {
        let mut p: Vec<[f32; 2]> = (0..50).map(|k| [k as f32, 0.0]).collect();
        p.push([5000.0, 0.0]);
        assert_eq!(densest(&p, 100.0).0, 50);
    }
}
