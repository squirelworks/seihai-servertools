//! NPC scripts: the GS1 check the world checker runs on level NPCs, and
//! what the NPC editor will build on (`docs/NEXT.md`, LAT track).
//!
//! Two kinds of NPC:
//!
//! - **Level NPCs**: the `NPC ... NPCEND` blocks in a level. [`check_script`]
//!   parses the script with `seihai-core`'s GS1 parser and lists what it
//!   found: parse problems, names that are not GS1 commands, and the images
//!   and sounds the script names. The editor itself is still planned.
//! - **Database NPCs, classes and weapons** on the NPC-server: these need an
//!   NC login (RC type 3). That is `seihai-rc` stage 5; the editor uses its
//!   library, pinned by tag, once it exists.
//!
//! The parser never fails: a bad statement is a diagnostic and is skipped,
//! and the script still runs in the client. So a diagnostic means "this
//! part does nothing", not "the level will not load".
//!
//! The vocabulary is the newest era (2.1 and later), the widest command
//! set, so a command only older clients lack is not reported.

use seihai_core::scripting::engine::ast::{Arg, Stmt};
use seihai_core::scripting::engine::{self, vocab, Era};

/// What kind of file a script names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Image,
    Sound,
}

/// A file a script names with a fixed name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedFile {
    pub kind: FileKind,
    pub name: String,
    /// The command that names it (`setimg`, `play`, ...).
    pub command: String,
    /// Line in the script, from 1.
    pub line: u32,
}

/// What [`check_script`] found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptCheck {
    /// Parse problems: `line N: ...`. That part of the script is skipped.
    pub problems: Vec<String>,
    /// Names used like commands that the parser's vocabulary does not have:
    /// `line N: name (reason)`. The vocabulary is the client's commands, so
    /// real server-side commands (`addweapon`, `putnpc2`) land here too; the
    /// world checker does not report these.
    pub unknown: Vec<String>,
    /// Images and sounds named with a fixed name. Names built at run time
    /// (`#` codes such as `#v(...)`) are left out.
    pub files: Vec<NamedFile>,
}

/// The file kind each file-naming command's file is. Commands not listed
/// name files that live elsewhere (heads, bodies, maps, palettes, letter
/// sets) and are not checked here.
fn file_kind(command: &str) -> Option<FileKind> {
    match command {
        "setimg" | "setgif" | "setimgpart" | "setgifpart" | "showimg" => Some(FileKind::Image),
        "play" | "playlooped" | "play2" => Some(FileKind::Sound),
        _ => None,
    }
}

/// Parse a level NPC's script and list what is wrong with it and the files
/// it names.
pub fn check_script(src: &str) -> ScriptCheck {
    let script = engine::parse(src, Era::V21);
    let mut out = ScriptCheck {
        problems: script.diagnostics.iter().map(|d| d.to_string()).collect(),
        ..ScriptCheck::default()
    };
    for stmt in &script.body {
        walk(stmt, &mut out);
    }
    out
}

fn walk(stmt: &Stmt, out: &mut ScriptCheck) {
    match stmt {
        Stmt::Block(body) | Stmt::Function { body, .. } => {
            for s in body {
                walk(s, out);
            }
        }
        Stmt::If { then, els, .. } => {
            walk(then, out);
            if let Some(e) = els {
                walk(e, out);
            }
        }
        Stmt::While { body, .. } | Stmt::With { body, .. } => walk(body, out),
        Stmt::For {
            init, step, body, ..
        } => {
            for s in [init, step].into_iter().flatten() {
                walk(s, out);
            }
            walk(body, out);
        }
        Stmt::Command { name, args, line } => note_files(name, args, *line, out),
        Stmt::Unknown {
            name, reason, line, ..
        } => out.unknown.push(format!("line {line}: {name} ({reason})")),
        _ => {}
    }
}

fn note_files(name: &str, args: &[Arg], line: u32, out: &mut ScriptCheck) {
    let Some(kind) = file_kind(name) else {
        return;
    };
    let Some(cmd) = vocab::command(name) else {
        return;
    };
    for (i, sig) in cmd.sig.chars().enumerate() {
        if sig != 'F' {
            continue;
        }
        let Some(Arg::Text(text)) = args.get(i) else {
            continue;
        };
        let text = text.trim();
        if text.is_empty() || text.contains('#') {
            continue;
        }
        out.files.push(NamedFile {
            kind,
            name: text.to_string(),
            command: name.to_string(),
            line,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(c: &ScriptCheck) -> Vec<(FileKind, &str)> {
        c.files.iter().map(|f| (f.kind, f.name.as_str())).collect()
    }

    #[test]
    fn a_clean_script_names_its_files() {
        let c = check_script(
            "if (playerenters) {\n  setimg door.png;\n  showimg 1,light.png,30,30;\n}\nif (playertouchsme) play chest.wav;\n",
        );
        assert!(c.problems.is_empty(), "{:?}", c.problems);
        assert!(c.unknown.is_empty(), "{:?}", c.unknown);
        assert_eq!(
            names(&c),
            [
                (FileKind::Image, "door.png"),
                (FileKind::Image, "light.png"),
                (FileKind::Sound, "chest.wav"),
            ]
        );
        assert_eq!(c.files[2].line, 5);
    }

    #[test]
    fn run_time_names_are_left_out() {
        let c = check_script("setimg #s(this.pic);\nplay #v(this.n).wav;\n");
        assert!(c.files.is_empty(), "{:?}", c.files);
    }

    #[test]
    fn a_broken_script_is_reported() {
        let c = check_script("if (playerenters) {\n  setimg a.png;\n");
        assert!(!c.problems.is_empty());
        assert!(c.problems[0].starts_with("line "), "{:?}", c.problems);
    }

    #[test]
    fn files_in_nested_blocks_and_functions() {
        let c = check_script(
            "function open() {\n  while (this.n < 2) { this.n++; play open.wav; }\n}\nif (created) { if (x > 1) setgifpart a.gif,0,0,16,16; else setgif b.gif; }\n",
        );
        assert_eq!(
            names(&c),
            [
                (FileKind::Sound, "open.wav"),
                (FileKind::Image, "a.gif"),
                (FileKind::Image, "b.gif"),
            ]
        );
    }

    #[test]
    fn clientside_marker_is_a_comment() {
        let c = check_script("//#CLIENTSIDE\nif (playertouchsme) { message hi; }\n");
        assert!(c.problems.is_empty(), "{:?}", c.problems);
        assert!(c.unknown.is_empty(), "{:?}", c.unknown);
    }
}
