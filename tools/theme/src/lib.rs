//! `seihai-theme make` (also `seihai-servertools theme`): LFThemeMaker's
//! patch themes (`lat_core::levelfill`).

use lat_core::levelfill::{make_theme, Recipe};
use std::path::Path;
use std::process::ExitCode;

/// Help for `seihai-theme`.
pub const USAGE: &str = "\
theme make RECIPE.tm OUT.nwt [--seed N] [--force]
    LFThemeMaker: #ofObjects random patches of Tile01..Tile15 on Tile00,
    sized and as dense as the recipe says, each with the rule
    QtyPerObject|Tile00.";

struct Opts {
    plain: Vec<String>,
    seed: u32,
    force: bool,
}

fn parse(args: &[&str]) -> Result<Opts, String> {
    let mut o = Opts {
        plain: Vec::new(),
        seed: 0,
        force: false,
    };
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        match a {
            "--seed" => {
                let v = it.next().ok_or("--seed needs a value")?;
                o.seed = v.parse().map_err(|_| format!("--seed {v}: not a number"))?;
            }
            "--force" => o.force = true,
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s => o.plain.push(s.to_string()),
        }
    }
    Ok(o)
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read(path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Runs one command line: the words after `seihai-theme` (or after
/// `seihai-servertools theme`): `make RECIPE.tm OUT.nwt`.
pub fn run(args: &[&str]) -> ExitCode {
    if matches!(args, ["--help" | "-h"]) {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let result = (|| -> Result<(), String> {
        let ["make", rest @ ..] = args else {
            return Err("use theme make RECIPE.tm OUT.nwt".into());
        };
        let o = parse(rest)?;
        let [recipe, out] = o.plain.as_slice() else {
            return Err("name the recipe and the theme to write".into());
        };
        let out = Path::new(out);
        if out.exists() && !o.force {
            return Err(format!(
                "{} already exists (use --force to replace)",
                out.display()
            ));
        }
        let recipe =
            Recipe::parse(&read(Path::new(recipe))?).map_err(|e| format!("{recipe}: {e}"))?;
        let theme = make_theme(&recipe, o.seed)?;
        std::fs::write(out, theme.write()).map_err(|e| format!("{}: {e}", out.display()))?;
        println!("{}: {} objects", out.display(), theme.objects.len());
        Ok(())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("theme: {e}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}
