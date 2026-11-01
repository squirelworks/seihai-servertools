//! The world and config checker: walks a world folder and reports what a
//! server would trip over.
//!
//! **Today:**
//!
//! - levels (`.nw`, `.graal`, `.zelda`) that `seihai-core` cannot parse;
//! - level links whose destination level is not in the world;
//! - gmaps that do not parse, and gmap cells naming a level that is missing;
//! - ganis that do not parse;
//! - images and sounds a level NPC or a gani names, when `seihai-core`'s
//!   asset resolver cannot find them in this folder (a warning: the world
//!   still loads).
//!
//! **Planned** (`docs/NEXT.md`): GS1 syntax errors in level NPCs (and the
//! images and sounds those scripts name), `serveroptions.txt` checks
//! (`setup_core::checks`), a `--json` report for CI.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use seihai_core::assets::{AssetResolver, AssetRoot};
use seihai_core::gmap::Gmap;
use seihai_core::protocol::ClientVersion;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warn,
    Error,
}

/// One problem in the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    /// The file, relative to the world root, `/`-separated.
    pub file: String,
    pub text: String,
}

/// Everything the checker found, and what it looked at.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub levels: usize,
    pub gmaps: usize,
    pub ganis: usize,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }
}

fn is_level(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".nw") || n.ends_with(".graal") || n.ends_with(".zelda")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Check a world folder. Level names are matched without case and without
/// folders, the way a gserver finds a level by file name.
pub fn check_world(root: &Path) -> Report {
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();

    let names: BTreeSet<String> = files
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .collect();
    let exists = |name: &str| names.contains(&name.to_ascii_lowercase());

    let mut report = Report::default();
    // 2.22's profile accepts a `.png` where a `.gif` was named, and the
    // other way around. The question here is whether the file is in the
    // folder, not whether one older build can draw it.
    let assets = AssetResolver::new(AssetRoot::new(root), ClientVersion::V222.profile());
    for path in &files {
        let file = rel(root, path);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let lower = name.to_ascii_lowercase();
        let Ok(bytes) = fs::read(path) else {
            continue;
        };
        if is_level(&lower) {
            report.levels += 1;
            match crate::level::open(&bytes) {
                Err(e) => report.findings.push(Finding {
                    severity: Severity::Error,
                    file,
                    text: format!("level does not parse: {e}"),
                }),
                Ok(level) => {
                    for link in &level.links {
                        if !link.dest.is_empty() && !exists(&link.dest) {
                            report.findings.push(Finding {
                                severity: Severity::Warn,
                                file: file.clone(),
                                text: format!(
                                    "link at {},{} goes to {}, which is not in the world",
                                    link.x, link.y, link.dest
                                ),
                            });
                        }
                    }
                    let mut seen = BTreeSet::new();
                    for npc in &level.npcs {
                        note_image(
                            &mut report,
                            &mut seen,
                            &file,
                            &npc.image,
                            "NPC image",
                            &assets,
                        );
                    }
                }
            }
        } else if lower.ends_with(".gmap") {
            report.gmaps += 1;
            match Gmap::parse(&String::from_utf8_lossy(&bytes)) {
                None => report.findings.push(Finding {
                    severity: Severity::Error,
                    file,
                    text: "gmap does not parse (GRMAP001 header and a level grid expected)".into(),
                }),
                Some(gmap) => {
                    for (i, cell) in gmap.names.iter().enumerate() {
                        if !cell.is_empty() && !exists(cell) {
                            report.findings.push(Finding {
                                severity: Severity::Error,
                                file: file.clone(),
                                text: format!(
                                    "cell {},{} names {}, which is not in the world",
                                    i % gmap.width.max(1),
                                    i / gmap.width.max(1),
                                    cell
                                ),
                            });
                        }
                    }
                }
            }
        } else if lower.ends_with(".gani") {
            report.ganis += 1;
            match crate::gani::open(&bytes) {
                Err(e) => report.findings.push(Finding {
                    severity: Severity::Error,
                    file,
                    text: format!("gani does not parse: {e}"),
                }),
                Ok(gani) => check_gani(&mut report, &file, &gani, &assets),
            }
        }
    }
    report
}

/// A gani `SPRITE` sheet that means one of the player's own pictures, not a
/// file in the world. Any other sheet is a file name.
const SYMBOLIC_SHEETS: &[&str] = &[
    "BODY", "HEAD", "SPRITES", "SWORD", "SHIELD", "ATTR1", "ATTR2", "ATTR3", "ATTR4", "ATTR5",
];

fn symbolic_sheet(name: &str) -> bool {
    SYMBOLIC_SHEETS.iter().any(|s| s.eq_ignore_ascii_case(name))
}

/// An image name that is a file. `-` is the empty NPC picture (`NPC - x y`
/// in seihai-core's level sample). A name starting with `@` is a text
/// picture (`@font@style@text`): the client draws it and does not load a file.
fn picture_file(name: &str) -> bool {
    !name.is_empty() && name != "-" && !name.starts_with('@')
}

fn note_image(
    report: &mut Report,
    seen: &mut BTreeSet<String>,
    file: &str,
    name: &str,
    role: &str,
    assets: &AssetResolver,
) {
    let name = name.trim();
    if !picture_file(name) {
        return;
    }
    if !seen.insert(format!("image:{}", name.to_ascii_lowercase())) {
        return;
    }
    if assets.image(name).is_none() {
        report.findings.push(Finding {
            severity: Severity::Warn,
            file: file.to_string(),
            text: format!("image {name} ({role}) is not in the world"),
        });
    }
}

fn note_sound(
    report: &mut Report,
    seen: &mut BTreeSet<String>,
    file: &str,
    name: &str,
    assets: &AssetResolver,
) {
    let name = name.trim();
    let key = name.trim_start_matches('+');
    if key.is_empty() {
        return;
    }
    if !seen.insert(format!("sound:{}", key.to_ascii_lowercase())) {
        return;
    }
    if assets.sound(name).is_none() {
        report.findings.push(Finding {
            severity: Severity::Warn,
            file: file.to_string(),
            text: format!("sound {name} (gani PLAYSOUND) is not in the world"),
        });
    }
}

fn check_gani(report: &mut Report, file: &str, gani: &crate::gani::Gani, assets: &AssetResolver) {
    let mut seen = BTreeSet::new();
    for sprite in &gani.sprites {
        if symbolic_sheet(sprite.sheet.trim()) {
            continue;
        }
        note_image(
            report,
            &mut seen,
            file,
            &sprite.sheet,
            "gani sprite",
            assets,
        );
    }
    if let Some(name) = &gani.default_head {
        note_image(report, &mut seen, file, name, "gani default head", assets);
    }
    if let Some(name) = &gani.default_body {
        note_image(report, &mut seen, file, name, "gani default body", assets);
    }
    for (i, name) in gani.default_attrs.iter().enumerate() {
        if let Some(name) = name {
            note_image(
                report,
                &mut seen,
                file,
                name,
                &format!("gani attribute {}", i + 1),
                assets,
            );
        }
    }
    for frame in &gani.frames {
        for sound in &frame.sounds {
            note_sound(report, &mut seen, file, &sound.file, assets);
        }
    }
}
