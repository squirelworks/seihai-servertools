//! `seihai-gif2mng` (also `seihai-servertools gif2mng`): a GIF as a PNG or
//! an MNG animation (`lat_core::anim`).

use std::path::Path;
use std::process::ExitCode;

/// Help for `seihai-gif2mng`.
pub const USAGE: &str = "\
gif2mng [-b] [--out DIR] [--force] GIF...
    Each GIF as a .png (one frame) or .mng animation beside it or in DIR;
    -b makes palette index 2 transparent. An existing file is kept unless
    --force is given.";

/// Runs one command line: the words after `seihai-gif2mng` (or after
/// `seihai-servertools gif2mng`).
pub fn run(args: &[&str]) -> ExitCode {
    if matches!(args, ["--help" | "-h"]) {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let (mut body, mut force, mut out, mut gifs) = (false, false, None, Vec::new());
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        match a {
            "-b" => body = true,
            "--force" => force = true,
            "--out" => match it.next() {
                Some(d) => out = Some(Path::new(*d)),
                None => {
                    eprintln!("gif2mng: --out needs a folder");
                    return ExitCode::from(2);
                }
            },
            s if s.starts_with('-') => {
                eprintln!("gif2mng: unknown option {s}\n{USAGE}");
                return ExitCode::from(2);
            }
            s => gifs.push(Path::new(s)),
        }
    }
    if gifs.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let mut failed = false;
    for gif in gifs {
        let result = std::fs::read(gif)
            .map_err(|e| e.to_string())
            .and_then(|bytes| lat_core::anim::gif_to_anim(&bytes, body))
            .and_then(|anim| {
                let name = gif.with_extension(anim.extension);
                let dst = match out {
                    Some(dir) => {
                        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                        dir.join(name.file_name().unwrap_or_default())
                    }
                    None => name,
                };
                if dst.exists() && !force {
                    return Err(format!(
                        "{} already exists (use --force to replace)",
                        dst.display()
                    ));
                }
                std::fs::write(&dst, &anim.bytes).map_err(|e| e.to_string())?;
                println!(
                    "{} -> {}: {} by {}, {} frame{}",
                    gif.display(),
                    dst.display(),
                    anim.width,
                    anim.height,
                    anim.frames,
                    if anim.frames == 1 { "" } else { "s" }
                );
                Ok(())
            });
        if let Err(e) = result {
            eprintln!("gif2mng: {}: {e}", gif.display());
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
