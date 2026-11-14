//! `seihai-servertools level fill` and `theme make`: LevelFiller and
//! LFThemeMaker (`lat_core::levelfill`).

use lat_core::generate::DelphiRandom;
use lat_core::levelfill::{fill_area, make_theme, Area, FillLevel, FillOptions, Recipe, Theme};
use lat_core::worldmap::Gmap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub const FILL_USAGE: &str = "\
level fill THEME.nwt LEVEL... (--out DIR | --in-place) [--seed N] [--edges]
           [--log FILE] [--across GMAP]
    LevelFiller: scatters the theme's objects over each GLEVNW01 level, rule
    by rule in theme order, only where the whole object covers the rule's
    replace tile (|tile, else repbase, else base). Every line but the
    layer-0 BOARD rows is kept as it was. --seed makes a fill repeatable
    (default 0; each level continues the same sequence). --edges also tries
    positions over the level's edge (they never fit; LevelFiller's
    \"keep inside\" off). --log writes what each rule placed.
    Complex objects run their script (setsize, add, return, if, while,
    for, r, c, round); their debug and msg lines are printed and logged.
    Border objects (.top=, .left=, .bttm=, .rght= as percent,object) are
    placed along the outer edges of each rule's patches, where the whole
    border fits on the replace tile. --across GMAP lets objects (with
    --edges) and borders run into the neighbouring levels that the gmap
    places next to a level, when those levels are in the same batch; all
    levels are written after the whole batch is filled.";

pub const THEME_USAGE: &str = "\
theme make RECIPE.tm OUT.nwt [--seed N] [--force]
    LFThemeMaker: #ofObjects random patches of Tile01..Tile15 on Tile00,
    sized and as dense as the recipe says, each with the rule
    QtyPerObject|Tile00.";

/// Options as `(name, value)`; a switch has no value.
type Flags = Vec<(String, Option<String>)>;

fn take_opts(args: &[&str]) -> Result<(Vec<String>, Flags), String> {
    let mut plain = Vec::new();
    let mut flags = Vec::new();
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        match a {
            "--out" | "--seed" | "--log" | "--across" => {
                let v = it.next().ok_or(format!("{a} needs a value"))?;
                flags.push((a.to_string(), Some(v.to_string())));
            }
            "--in-place" | "--edges" | "--force" => flags.push((a.to_string(), None)),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s => plain.push(s.to_string()),
        }
    }
    Ok((plain, flags))
}

fn flag<'a>(flags: &'a Flags, name: &str) -> Option<Option<&'a str>> {
    flags
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_deref())
}

fn seed(flags: &Flags) -> Result<u32, String> {
    match flag(flags, "--seed").flatten() {
        Some(v) => v.parse().map_err(|_| format!("--seed {v}: not a number")),
        None => Ok(0),
    }
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read(path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .map_err(|e| format!("{}: {e}", path.display()))
}

pub fn run_fill(args: &[&str]) -> ExitCode {
    match fill_levels(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("level fill: {e}\n{FILL_USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn fill_levels(args: &[&str]) -> Result<(), String> {
    let (plain, flags) = take_opts(args)?;
    let [theme, levels @ ..] = plain.as_slice() else {
        return Err("name a theme and at least one level".into());
    };
    if levels.is_empty() {
        return Err("name at least one level".into());
    }
    let out = flag(&flags, "--out").flatten().map(PathBuf::from);
    let in_place = flag(&flags, "--in-place").is_some();
    if out.is_some() == in_place {
        return Err("use one of --out DIR or --in-place".into());
    }
    let theme = Theme::parse(&read(Path::new(theme))?).map_err(|e| format!("{theme}: {e}"))?;
    let opts = FillOptions {
        keep_inside: flag(&flags, "--edges").is_none(),
    };
    let mut rng = DelphiRandom(seed(&flags)?);
    let mut log = String::new();
    if let Some(dir) = &out {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut batch = Vec::new();
    for name in levels {
        let src = Path::new(name);
        let level = FillLevel::parse(&read(src)?).map_err(|e| format!("{name}: {e}"))?;
        batch.push((src, level));
    }
    let neighbours = match flag(&flags, "--across").flatten() {
        Some(gmap) => neighbours(Path::new(gmap), &batch)?,
        None => vec![[None; 9]; batch.len()],
    };
    let mut reports = Vec::new();
    for (i, around) in neighbours.iter().enumerate() {
        let mut area = Area::default();
        for (slot, at) in around.iter().enumerate() {
            if let Some(j) = at {
                area.boards[slot] = Some(std::mem::take(&mut batch[*j].1.board));
            }
        }
        area.boards[4] = Some(std::mem::take(&mut batch[i].1.board));
        reports.push(fill_area(&mut area, &theme, &mut rng, &opts));
        batch[i].1.board = area.boards[4].take().unwrap_or_default();
        for (slot, at) in around.iter().enumerate() {
            if let Some(j) = at {
                batch[*j].1.board = area.boards[slot].take().unwrap_or_default();
            }
        }
    }
    for ((src, level), report) in batch.iter().zip(reports) {
        let dst = match &out {
            Some(dir) => dir.join(src.file_name().unwrap_or_default()),
            None => src.to_path_buf(),
        };
        std::fs::write(&dst, level.write()).map_err(|e| format!("{}: {e}", dst.display()))?;
        let placed: u32 = report.placed.iter().map(|p| p.1).sum();
        println!(
            "{} -> {}: {placed} objects placed",
            src.display(),
            dst.display()
        );
        log.push_str(&format!("{}\n", src.display()));
        for (obj, n, tried) in &report.placed {
            log.push_str(&format!("  {obj}: {n} of {tried} tries\n"));
        }
        for line in &report.log {
            println!("  script: {line}");
            log.push_str(&format!("  script: {line}\n"));
        }
        for note in &report.notes {
            println!("  {note}");
            log.push_str(&format!("  {note}\n"));
        }
    }
    if let Some(Some(path)) = flag(&flags, "--log") {
        std::fs::write(path, log).map_err(|e| format!("{path}: {e}"))?;
    }
    Ok(())
}

/// For each level of the batch, the batch levels around it on `gmap`
/// (row by row from the top left, itself at 4, which stays `None`). Levels
/// are matched by file name in any letter case.
fn neighbours(
    gmap: &Path,
    batch: &[(&Path, FillLevel)],
) -> Result<Vec<[Option<usize>; 9]>, String> {
    let g = Gmap::parse(&read(gmap)?).ok_or_else(|| format!("{}: not a gmap", gmap.display()))?;
    let base = |name: &str| {
        name.rsplit(['/', '\\'])
            .next()
            .unwrap_or(name)
            .to_ascii_lowercase()
    };
    let by_name: HashMap<String, usize> = batch
        .iter()
        .enumerate()
        .filter_map(|(i, (p, _))| Some((p.file_name()?.to_string_lossy().to_ascii_lowercase(), i)))
        .collect();
    let width = g.width.max(1) as i64;
    let mut cell = vec![None; batch.len()];
    let mut at: HashMap<(i64, i64), usize> = HashMap::new();
    for (k, name) in g.names.iter().enumerate() {
        if let Some(&i) = by_name.get(&base(name)) {
            let pos = (k as i64 % width, k as i64 / width);
            cell[i].get_or_insert(pos);
            at.entry(pos).or_insert(i);
        }
    }
    let mut out = Vec::new();
    for (i, pos) in cell.iter().enumerate() {
        let mut around = [None; 9];
        match pos {
            Some((x, y)) => {
                for (slot, a) in around.iter_mut().enumerate() {
                    let (dx, dy) = (slot as i64 % 3 - 1, slot as i64 / 3 - 1);
                    if slot != 4 {
                        *a = at.get(&(x + dx, y + dy)).copied();
                    }
                }
            }
            None => eprintln!(
                "level fill: warning: {} is not on {}; filled alone",
                batch[i].0.display(),
                gmap.display()
            ),
        }
        out.push(around);
    }
    Ok(out)
}

pub fn run_theme(args: &[&str]) -> ExitCode {
    let result = (|| -> Result<(), String> {
        let ["make", rest @ ..] = args else {
            return Err("use theme make RECIPE.tm OUT.nwt".into());
        };
        let (plain, flags) = take_opts(rest)?;
        let [recipe, out] = plain.as_slice() else {
            return Err("name the recipe and the theme to write".into());
        };
        let out = Path::new(out);
        if out.exists() && flag(&flags, "--force").is_none() {
            return Err(format!(
                "{} already exists (use --force to replace)",
                out.display()
            ));
        }
        let recipe =
            Recipe::parse(&read(Path::new(recipe))?).map_err(|e| format!("{recipe}: {e}"))?;
        let theme = make_theme(&recipe, seed(&flags)?)?;
        std::fs::write(out, theme.write()).map_err(|e| format!("{}: {e}", out.display()))?;
        println!("{}: {} objects", out.display(), theme.objects.len());
        Ok(())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("theme: {e}\n{THEME_USAGE}");
            ExitCode::FAILURE
        }
    }
}
