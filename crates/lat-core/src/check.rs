//! The world and config checker: walks a world folder and reports what a
//! server would trip over.
//!
//! **Today:**
//!
//! - levels (`.nw`, `.graal`, `.zelda`) that `seihai-core` cannot parse;
//! - level links whose destination level is not in the world;
//! - gmaps that do not parse, and gmap cells naming a level that is missing;
//! - ganis that do not parse.
//!
//! **Planned** (`docs/NEXT.md`): missing images and sounds that NPCs and
//! ganis name (with `seihai-core`'s asset resolver), GS1 syntax errors,
//! `serveroptions.txt` and folder config checks (`setup_core::checks`), a
//! `--json` report for CI.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use seihai_core::gmap::Gmap;

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
            if let Err(e) = crate::gani::open(&bytes) {
                report.findings.push(Finding {
                    severity: Severity::Error,
                    file,
                    text: format!("gani does not parse: {e}"),
                });
            }
        }
    }
    report
}
