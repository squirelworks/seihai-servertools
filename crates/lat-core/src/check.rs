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
//!   still loads), including the fixed names a level NPC's script gives
//!   `setimg`, `setgif`, `setimgpart`, `setgifpart`, `showimg`, `play`,
//!   `playlooped` and `play2`;
//! - level NPC scripts: GS1 parse problems (warnings: the client skips that
//!   part and runs the rest; at most three per NPC, then a count). Names the
//!   parser does not know as commands are not reported: its vocabulary is
//!   the client's, so server-side commands such as `addweapon` would show
//!   up. See [`crate::npc::check_script`].
//!
//! - resource packages in each gserver folder (`servers/<name>/`, or the
//!   world itself when it has a `config/` folder): a `.gpake` with no key
//!   in `config/gpakkeys.txt` (the gserver will not send it), a key file
//!   where the gserver serves files (anyone could download the keys), and
//!   a plain `.gpak` when `config/allowedversions.txt` may let in 4.20r8,
//!   which saves a `.gpak` but never loads it (`docs/design/gpak.md`).
//!
//! The server settings (`serveroptions.txt`, `seihai-listserver.toml`) are
//! checked by `setup_core::folder::check_world_config`, which
//! `seihai-servertools check` runs next to this.
//!
//! `seihai-servertools check --json` prints this report and the settings
//! findings as one JSON object.

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
                        check_npc_script(&mut report, &mut seen, &file, npc, &assets);
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
    report.findings.extend(check_packages(root));
    report
}

/// The folders a gserver serves files from, relative to its folder:
/// `world/` (seihai-gserver-v2's `servers/<name>/world/`, with packages in
/// `world/gpaks/`) and `files/` (seihai-gserver's world folder).
const SERVED: [&str; 2] = ["world", "files"];

/// The package checks for every gserver folder in a world. See the module
/// note; all are warnings, since the server still starts.
pub fn check_packages(root: &Path) -> Vec<Finding> {
    let mut servers = vec![root.to_path_buf()];
    if let Ok(rd) = fs::read_dir(root.join("servers")) {
        let mut more: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        more.sort();
        servers.extend(more);
    }
    let mut out = Vec::new();
    for server in servers.iter().filter(|s| s.join("config").is_dir()) {
        let config = server.join("config");
        let keys = fs::read_to_string(crate::gpak::keys_path(server)).unwrap_or_default();
        let lets_in_420 = allows_420(server);
        let mut files = Vec::new();
        for dir in SERVED {
            walk(&server.join(dir), &mut files);
        }
        files.sort();
        for path in &files {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let file = rel(root, path);
            if name.starts_with("gpakkeys.txt") {
                out.push(Finding {
                    severity: Severity::Warn,
                    file,
                    text: "a package key file in a folder the gserver serves: anyone can download it and unpack every .gpake; keep keys only in config/gpakkeys.txt".into(),
                });
            } else if crate::gpak::is_encrypted(&name) {
                if crate::gpak::key_in(&keys, &name).is_none() {
                    out.push(Finding {
                        severity: Severity::Warn,
                        file,
                        text: format!(
                            "{name} has no key in {}: the gserver will not send it (gpak pack --server writes the key)",
                            rel(root, &config.join("gpakkeys.txt"))
                        ),
                    });
                }
            } else if name.ends_with(".gpak") {
                if let Some(why) = &lets_in_420 {
                    out.push(Finding {
                        severity: Severity::Warn,
                        file,
                        text: format!(
                            "a plain .gpak, and {why}: 4.20r8 saves a .gpak but never loads it; use a .gpake (or .zip) when 4.20 players matter"
                        ),
                    });
                }
            }
        }
    }
    out
}

/// Whether a gserver folder's `config/allowedversions.txt` may let in
/// 4.20r8 (`G3D20126`), and the line that does. Each line is a token, a
/// range `A:B`, `*`, or `name = entries` for one server generation; when
/// `serveroptions.txt` names a `generation`, only that one's named lines
/// count. A range counts when 4.20r8 lies between its ends in
/// `seihai-core`'s version order (oldest first). Mac 4.20r6 (`G3D27076`)
/// is not in the fork's version table, so it is not looked for. `None`
/// when there is no such file, or nothing in it reaches 4.20r8.
fn allows_420(server: &Path) -> Option<String> {
    let text = fs::read_to_string(server.join("config/allowedversions.txt")).ok()?;
    let options = fs::read_to_string(server.join("config/serveroptions.txt")).unwrap_or_default();
    let generation = options.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim().eq_ignore_ascii_case("generation")).then(|| v.trim().to_ascii_lowercase())
    });
    let order: Vec<&str> = ClientVersion::all_versions()
        .iter()
        .map(|v| v.token())
        .collect();
    let target = ClientVersion::V420R8.token();
    let at = |t: &str| order.iter().position(|o| o.eq_ignore_ascii_case(t));
    let target_at = at(target)?;
    for raw in text.lines() {
        let line = raw.split("//").next().unwrap_or("");
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        let entries = match line.split_once('=') {
            Some((name, value)) => {
                let name = name.trim().to_ascii_lowercase();
                if generation.as_ref().is_some_and(|g| *g != name) {
                    continue;
                }
                value
            }
            None => line,
        };
        for entry in entries.split([',', ' ', '\t']).filter(|e| !e.is_empty()) {
            let hit = if entry == "*" {
                true
            } else if let Some((a, b)) = entry.split_once(':') {
                matches!((at(a.trim()), at(b.trim())), (Some(a), Some(b)) if a <= target_at && target_at <= b)
            } else {
                entry.eq_ignore_ascii_case(target)
            };
            if hit {
                return Some(format!(
                    "config/allowedversions.txt lets in 4.20r8 (`{}`)",
                    raw.trim()
                ));
            }
        }
    }
    None
}

/// At most this many script problems are listed for one NPC; the rest are
/// counted. A script written for the newer language trips the GS1 parser on
/// most lines, and one NPC should not bury the report.
const MAX_SCRIPT_PROBLEMS: usize = 3;

/// Inventory PNG references without GIF counterparts (level NPCs and
/// weapon files, see `images_named_by`) and parsed gani files needing
/// fallback review for shared profiles without `ui.ganis` (#38). Presence does not prove use or original-client rendering.
///
/// An image name a script builds at run time (`bomb#v(this.n).png`) is
/// matched against the PNG and GIF names in the world (see
/// [`crate::npc::RunTimeName`]); each PNG it could be is checked like a
/// fixed name. A run-time name that matches no file, or says too little to
/// match, is reported so the owner checks it by hand. This never writes or
/// converts files.
pub fn check_image_reach(root: &Path) -> Vec<Finding> {
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let names: BTreeSet<String> = files
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .collect();
    let mut findings = Vec::new();
    let without_ganis = ClientVersion::all_versions()
        .iter()
        .filter(|v| !v.profile().ui.ganis)
        .map(|v| v.label())
        .collect::<Vec<_>>()
        .join(", ");
    for path in &files {
        let lower = path.to_string_lossy().to_ascii_lowercase();
        if lower.ends_with(".gani") {
            let Ok(bytes) = fs::read(path) else {
                continue;
            };
            // Reuse the shared parser. Ordinary check_world reports parse
            // errors and missing literal images/sounds; do not duplicate them.
            if crate::gani::open(&bytes).is_ok() && !without_ganis.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warn,
                    file: rel(root, path),
                    text: format!(
                        "client reach: gani file present; shared profiles without gani support [{without_ganis}] need server animation fallback review if it is used; file presence does not prove use, and no nearest built-in animation is inferred"
                    ),
                });
            }
            continue;
        }
        let Some(images) = images_named_by(path, &lower, &names) else {
            continue;
        };
        for (image, stem) in gif_wanted_in(&images.fixed, &names) {
            let via = match images.via.get(&image) {
                Some(template) => format!(" (a run-time name, {template}, can be this file)"),
                None => String::new(),
            };
            findings.push(Finding {
                severity: Severity::Warn,
                file: rel(root, path),
                text: format!("client reach: image {image}{via} has PNG art but no {stem}.gif counterpart for pre-2.1 clients"),
            });
        }
        for (template, why) in images.unresolved {
            findings.push(Finding {
                severity: Severity::Warn,
                file: rel(root, path),
                text: format!("client reach: image name {template} is built at run time and {why}; check by hand that pre-2.1 clients have a GIF for it"),
            });
        }
    }
    findings
}

/// The shared profiles (oldest first) for which `lacks` is true, as labels.
fn profiles_lacking(lacks: impl Fn(&seihai_core::protocol::Levels) -> bool) -> String {
    ClientVersion::all_versions()
        .iter()
        .filter(|v| lacks(&v.profile().levels))
        .map(|v| v.label())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Level and gmap reach (#38): every parsed `.gmap`, with the shared
/// profiles whose `levels.gmap` is false (they need the linked-level
/// fallback, if a player can reach the gmap), and every level whose format
/// some profiles cannot open. The format is read from the level's magic
/// (`GLEVNW01`, `GR-V1`, `Z3-V1`), as the clients do; the extension decides
/// nothing. An inventory: presence does not prove a player reaches the
/// file, and no fallback or conversion is inferred or written.
pub fn check_level_reach(root: &Path) -> Vec<Finding> {
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let mut findings = Vec::new();
    let no_gmap = profiles_lacking(|l| !l.gmap);
    let formats: [(&str, &str, String); 3] = [
        ("GLEVNW01", "NW", profiles_lacking(|l| !l.nw)),
        ("GR-V1", "GR-V1 (.graal)", profiles_lacking(|l| !l.graal)),
        ("Z3-V1", "Z3-V1 (.zelda)", profiles_lacking(|l| !l.zelda)),
    ];
    for path in &files {
        let lower = path.to_string_lossy().to_ascii_lowercase();
        let gmap = lower.ends_with(".gmap");
        if !gmap && !is_level(&lower) {
            continue;
        }
        let Ok(bytes) = fs::read(path) else {
            continue;
        };
        if gmap {
            if Gmap::parse(&String::from_utf8_lossy(&bytes)).is_some() && !no_gmap.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warn,
                    file: rel(root, path),
                    text: format!(
                        "client reach: gmap; shared profiles without gmap support [{no_gmap}] need linked single levels if players reach it; matching level names do not prove the server sends the links"
                    ),
                });
            }
            continue;
        }
        if crate::level::open(&bytes).is_err() {
            // check_world reports levels that do not parse.
            continue;
        }
        let Some((_, name, lacking)) = formats
            .iter()
            .find(|(magic, _, _)| bytes.starts_with(magic.as_bytes()))
        else {
            continue;
        };
        if !lacking.is_empty() {
            findings.push(Finding {
                severity: Severity::Warn,
                file: rel(root, path),
                text: format!(
                    "client reach: {name} level; shared profiles that cannot open it [{lacking}] get nothing if they reach it"
                ),
            });
        }
    }
    findings
}

/// A gserver weapon file (`weapons/*.txt`, first line `GRAWP001`): its
/// name, `IMAGE` and the script between `SCRIPT` and `SCRIPTEND`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeaponFile {
    pub name: String,
    pub image: String,
    pub script: String,
}

/// Read a weapon file, or `None` when `path` is not a `.txt` file in a
/// `weapons` folder that starts with `GRAWP001`.
pub fn read_weapon(path: &Path) -> Option<WeaponFile> {
    let in_weapons = path
        .parent()
        .and_then(|p| p.file_name())
        .is_some_and(|n| n.eq_ignore_ascii_case("weapons"));
    let txt = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("txt"));
    if !in_weapons || !txt {
        return None;
    }
    let text = String::from_utf8_lossy(&fs::read(path).ok()?).into_owned();
    let mut lines = text.lines();
    if lines.next()?.trim() != "GRAWP001" {
        return None;
    }
    let mut weapon = WeaponFile::default();
    let mut in_script = false;
    for line in lines {
        let trimmed = line.trim_end_matches('\r');
        if in_script {
            if trimmed.trim() == "SCRIPTEND" {
                in_script = false;
            } else {
                weapon.script.push_str(trimmed);
                weapon.script.push('\n');
            }
            continue;
        }
        let (key, value) = trimmed
            .split_once(' ')
            .map_or((trimmed.trim(), ""), |(k, v)| (k.trim(), v.trim()));
        match key {
            "REALNAME" => weapon.name = value.to_string(),
            "IMAGE" => weapon.image = value.to_string(),
            "SCRIPT" => in_script = true,
            _ => {}
        }
    }
    Some(weapon)
}

/// The shared profiles (oldest first) whose `scripts.gs2` is no, and those
/// where it is unknown, as labels. seihai-core sets the field from whether
/// the client's binary handles `PLO_NPCBYTECODE` (131): 3.153 on run GS2;
/// unknown means there is no binary to read.
fn profiles_without_gs2() -> (String, String) {
    let list = |want: &str| {
        ClientVersion::all_versions()
            .iter()
            .filter(|v| v.profile().scripts.gs2.as_str() == want)
            .map(|v| v.label())
            .collect::<Vec<_>>()
            .join(", ")
    };
    (list("no"), list("unknown"))
}

/// Script era reach (#38): every level NPC and weapon whose script is GS2
/// only, or mixes GS1 and GS2 ([`crate::npc::script_era`]), with the
/// shared profiles that do not run GS2 (and those not known either way;
/// see [`profiles_without_gs2`]). GS1-only scripts run on
/// every client and are not listed. An inventory from the script text:
/// presence does not prove a player reaches the level or gets the weapon,
/// and nothing is compiled or written.
pub fn check_script_reach(root: &Path) -> Vec<Finding> {
    use crate::npc::{script_era, ScriptEra};
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let (old, unknown) = profiles_without_gs2();
    let mut findings = Vec::new();
    let note = |what: &str, era: ScriptEra| {
        match era {
        ScriptEra::Gs1 => None,
        ScriptEra::Gs2 => Some(format!(
            "client reach: {what} script is GS2 only; shared profiles without GS2 [{old}] do not run it (not known for [{unknown}])"
        )),
        ScriptEra::Both => Some(format!(
            "client reach: {what} script is both GS1 and GS2; shared profiles without GS2 [{old}] run only the GS1 part and skip the GS2 lines (not known for [{unknown}])"
        )),
    }
    };
    for path in &files {
        let lower = path.to_string_lossy().to_ascii_lowercase();
        let mut texts = Vec::new();
        if is_level(&lower) {
            let Ok(bytes) = fs::read(path) else {
                continue;
            };
            let Ok(level) = crate::level::open(&bytes) else {
                continue;
            };
            for npc in &level.npcs {
                if let Some(era) = script_era(&npc.script) {
                    texts.extend(note(&format!("NPC at {},{}", npc.x, npc.y), era));
                }
            }
        } else if let Some(weapon) = read_weapon(path) {
            if let Some(era) = script_era(&weapon.script) {
                let what = if weapon.name.is_empty() {
                    "weapon".to_string()
                } else {
                    format!("weapon {}", weapon.name)
                };
                texts.extend(note(&what, era));
            }
        }
        for text in texts {
            findings.push(Finding {
                severity: Severity::Warn,
                file: rel(root, path),
                text,
            });
        }
    }
    findings
}

/// Columns of 16-pixel tiles on the `pics1.png` the 1.28 beta 2 to 1.41
/// and 2.00 to 2.05 clients ship: 1856×512. The 1.42 NewMain sheet and
/// later is 2048×512 (128 columns).
const OLD_SHEET_COLUMNS: u16 = 116;

/// The column of tile `id` on a `pics1.png` laid out as eight blocks of 16
/// columns by 32 rows (512 tiles per block): `id % 16 + (id / 512) * 16`.
fn tile_column(id: u16) -> u16 {
    id % 16 + (id / 512) * 16
}

/// A board tile's 12-bit id from its two base64 characters (high six bits
/// first), the code `GLEVNW01` boards use and seihai-core's parser keeps.
fn tile_id(tile: [u8; 2]) -> u16 {
    fn val(c: u8) -> u16 {
        match c {
            b'A'..=b'Z' => (c - b'A') as u16,
            b'a'..=b'z' => (c - b'a') as u16 + 26,
            b'0'..=b'9' => (c - b'0') as u16 + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
    (val(tile[0]) << 6) | val(tile[1])
}

/// The shared profiles that ship the 1856-wide sheet, as labels.
///
/// Evidence (Graal_analysis `version-profiles/matrix/README.md` and the
/// shipped files): the `pics1` of 1.28 beta 2, 1.31, 1.33 to 1.39, 1.41
/// rev 1 and 2.05 rev 2 is 1856×512, and its 116 columns match the left
/// 116 columns of the 1.42 NewMain sheet cell for cell (3,570 to 3,654 of
/// 3,712 cells identical; the rest are redrawn tiles), so tile ids map the
/// same way and the old sheet just ends at column 116. The 1.3 test, 1.32,
/// 1.40 and 2.00 to 2.04 rows ship no sheet in the archives and are taken
/// to be the same as their neighbours (inferred). 1.21's 1280-wide sheet
/// is left out: its layout is not established. The 2.1x rows are left out
/// too: no sheet of theirs is in the archives.
fn profiles_with_old_sheet() -> String {
    use ClientVersion::*;
    [
        V128B2, V13T, V131, V132, V133, V134, V135, V136, V137, V138, V139, V141, V200, V201, V202,
        V202R2, V203, V204, V205,
    ]
    .iter()
    .map(|v| v.label())
    .collect::<Vec<_>>()
    .join(", ")
}

/// Tile reach (#38): every parsed level that uses tiles past column 116 of
/// the default `pics1.png`, which the 1856-wide sheet of the 1.28 beta 2 to
/// 2.05 clients does not have ([`profiles_with_old_sheet`]). Those clients
/// draw nothing in that cell. Only the default sheet is considered: a
/// level drawn with a `tiledefs` sheet is not matched to it here. An
/// inventory; nothing is written.
pub fn check_tile_reach(root: &Path) -> Vec<Finding> {
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let old = profiles_with_old_sheet();
    let mut findings = Vec::new();
    for path in &files {
        let lower = path.to_string_lossy().to_ascii_lowercase();
        if !is_level(&lower) {
            continue;
        }
        let Ok(bytes) = fs::read(path) else {
            continue;
        };
        let Ok(level) = crate::level::open(&bytes) else {
            continue;
        };
        let mut cells = 0usize;
        let mut ids = BTreeSet::new();
        for run in &level.boards {
            for tile in &run.tiles {
                let id = tile_id(*tile);
                if tile_column(id) >= OLD_SHEET_COLUMNS {
                    cells += 1;
                    ids.insert(id);
                }
            }
        }
        if cells == 0 {
            continue;
        }
        let mut shown: Vec<String> = ids.iter().take(5).map(u16::to_string).collect();
        if ids.len() > 5 {
            shown.push(format!("and {} more", ids.len() - 5));
        }
        let word = if cells == 1 { "cell uses" } else { "cells use" };
        findings.push(Finding {
            severity: Severity::Warn,
            file: rel(root, path),
            text: format!(
                "client reach: {cells} {word} tiles past column {OLD_SHEET_COLUMNS} of pics1.png (ids {}); shared profiles with the 1856-wide sheet [{old}] draw nothing there",
                shown.join(", ")
            ),
        });
    }
    findings
}

/// The images one file names, for the GIF inventory.
#[derive(Default)]
struct NamedImages {
    /// Fixed names, then the world files that run-time names can be.
    fixed: Vec<String>,
    /// For a file name in `fixed` that came from a run-time name: that name
    /// as written. A file two run-time names can be keeps the first.
    via: std::collections::BTreeMap<String, String>,
    /// Run-time names that could not be matched to a file, with why.
    unresolved: Vec<(String, &'static str)>,
}

impl NamedImages {
    fn add_script(&mut self, script: &str, names: &BTreeSet<String>) {
        let check = crate::npc::check_script(script);
        let images = |f: &crate::npc::NamedFile| f.kind == crate::npc::FileKind::Image;
        self.fixed.extend(
            check
                .files
                .iter()
                .filter(|f| images(f))
                .map(|f| f.name.clone()),
        );
        for named in check.run_time.iter().filter(|f| images(f)) {
            let pattern = crate::npc::RunTimeName::parse(&named.name);
            if !pattern.is_specific() {
                self.unresolved
                    .push((named.name.clone(), "says too little to match to files"));
                continue;
            }
            let mut found = false;
            for name in names {
                let picture = name.ends_with(".png") || name.ends_with(".gif");
                if picture && pattern.matches(name) {
                    found = true;
                    if !self.via.contains_key(name) {
                        self.via.insert(name.clone(), named.name.clone());
                        self.fixed.push(name.clone());
                    }
                }
            }
            if !found {
                self.unresolved
                    .push((named.name.clone(), "matches no PNG or GIF in the world"));
            }
        }
    }
}

/// The images a file names, for the GIF inventory: a level's NPC pictures
/// and the names their scripts give `setimg` and friends, or a weapon
/// file's `IMAGE` and script names. Run-time script names are matched
/// against `names` (every file name in the world, lower case). `None` for
/// any other file, or a level that does not parse.
fn images_named_by(path: &Path, lower: &str, names: &BTreeSet<String>) -> Option<NamedImages> {
    let mut out = NamedImages::default();
    if is_level(lower) {
        let level = crate::level::open(&fs::read(path).ok()?).ok()?;
        for npc in &level.npcs {
            out.fixed.push(npc.image.clone());
            out.add_script(&npc.script, names);
        }
    } else {
        let weapon = read_weapon(path)?;
        out.fixed.push(weapon.image);
        if !weapon.script.trim().is_empty() {
            out.add_script(&weapon.script, names);
        }
    }
    Some(out)
}

/// Of `images`, those with PNG art in the world but no GIF of the same
/// name, which pre-2.1 clients ask for: `(name as written, lower-case
/// stem)`, each stem once. `names` holds every file name in the world,
/// lower case.
fn gif_wanted_in(images: &[String], names: &BTreeSet<String>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for image in images {
        let image = image.trim();
        if !picture_file(image) {
            continue;
        }
        let name = image
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(image)
            .to_ascii_lowercase();
        let Some(stem) = name
            .strip_suffix(".png")
            .or_else(|| name.strip_suffix(".gif"))
        else {
            continue;
        };
        if names.contains(&format!("{stem}.png"))
            && !names.contains(&format!("{stem}.gif"))
            && seen.insert(stem.to_string())
        {
            out.push((image.to_string(), stem.to_string()));
        }
    }
    out
}

/// The PNG files `check --client-reach` reports as missing a GIF copy, and
/// the GIF path each would get (same folder, same stem, `.gif`), in path
/// order. Every PNG with a wanted stem is listed, wherever it lies in the
/// world, since the gserver finds files by name alone.
pub fn gif_copies_wanted(root: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let names: BTreeSet<String> = files
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .collect();
    let mut stems = BTreeSet::new();
    for path in &files {
        let lower = path.to_string_lossy().to_ascii_lowercase();
        if let Some(images) = images_named_by(path, &lower, &names) {
            stems.extend(
                gif_wanted_in(&images.fixed, &names)
                    .into_iter()
                    .map(|(_, s)| s),
            );
        }
    }
    files
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_string_lossy().to_ascii_lowercase();
            let stem = name.strip_suffix(".png")?;
            stems
                .contains(stem)
                .then(|| (p.clone(), p.with_file_name(format!("{stem}.gif"))))
        })
        .collect()
}

fn check_npc_script(
    report: &mut Report,
    seen: &mut BTreeSet<String>,
    file: &str,
    npc: &seihai_core::assets::Npc,
    assets: &AssetResolver,
) {
    if npc.script.trim().is_empty() {
        return;
    }
    let check = crate::npc::check_script(&npc.script);
    let at = format!("NPC at {},{}", npc.x, npc.y);
    let mut lines: Vec<String> = check
        .problems
        .iter()
        .map(|p| format!("{at}: GS1 script {p}; that part of the script is skipped"))
        .collect();
    let more = lines.len().saturating_sub(MAX_SCRIPT_PROBLEMS);
    lines.truncate(MAX_SCRIPT_PROBLEMS);
    if more > 0 {
        let word = if more == 1 { "problem" } else { "problems" };
        lines.push(format!("{at}: {more} more GS1 script {word}"));
    }
    for text in lines {
        report.findings.push(Finding {
            severity: Severity::Warn,
            file: file.to_string(),
            text,
        });
    }
    for named in &check.files {
        let role = format!("{} in an NPC script", named.command);
        match named.kind {
            crate::npc::FileKind::Image => {
                note_image(report, seen, file, &named.name, &role, assets)
            }
            crate::npc::FileKind::Sound => {
                note_sound(report, seen, file, &named.name, &role, assets)
            }
        }
    }
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
    role: &str,
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
            text: format!("sound {name} ({role}) is not in the world"),
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
            note_sound(
                report,
                &mut seen,
                file,
                &sound.file,
                "gani PLAYSOUND",
                assets,
            );
        }
    }
}
