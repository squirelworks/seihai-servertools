//! NPC scripts: the GS1 check the world checker and the NPC editor page run
//! on level NPCs (`docs/NEXT.md`, LAT track).
//!
//! Two kinds of NPC:
//!
//! - **Level NPCs**: the `NPC ... NPCEND` blocks in a level. [`check_script`]
//!   parses the script with `seihai-core`'s GS1 parser and lists what it
//!   found: parse problems, names that are not GS1 commands, and the images
//!   and sounds the script names. The edit is `LevelDoc::set_npc_script`.
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
    /// (`#` codes such as `#v(...)`) are in [`Self::run_time`] instead.
    pub files: Vec<NamedFile>,
    /// Images and sounds whose name is built at run time: the name is the
    /// text as written, `#` codes and all (`bomb#v(this.n).png`). See
    /// [`RunTimeName`] for matching one to the files in a world.
    pub run_time: Vec<NamedFile>,
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
    let script = engine::parse(src, Era::V217);
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
        if text.is_empty() {
            continue;
        }
        let list = if text.contains('#') {
            &mut out.run_time
        } else {
            &mut out.files
        };
        list.push(NamedFile {
            kind,
            name: text.to_string(),
            command: name.to_string(),
            line,
        });
    }
}

/// A file name built at run time, as a pattern over file names (#38).
///
/// Each GS1 `#` code (`#v(...)`, `#s(...)`, `#c`, `#p(0)`, `#1`, ...) can
/// become any text, so it is a wildcard; the rest must match as written,
/// ignoring case. Only the file name is matched: a folder in front of it is
/// dropped, as the gserver finds files by name alone.
///
/// This says which files the name *could* be, not which ones the script
/// picks: a matched file may never be shown, and a value the script builds
/// from scratch can name a file the pattern misses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTimeName {
    /// Lower-case literal text between the codes. There is one more part
    /// than there are codes; a part may be empty.
    parts: Vec<String>,
}

impl RunTimeName {
    /// The pattern for `name`. A `#` at the very end is literal text.
    pub fn parse(name: &str) -> Self {
        let mut parts = vec![String::new()];
        let mut chars = name.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '#' || chars.peek().is_none() {
                parts.last_mut().unwrap().extend(c.to_lowercase());
                continue;
            }
            chars.next();
            if chars.peek() == Some(&'(') {
                let mut depth = 0usize;
                for c in chars.by_ref() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            parts.push(String::new());
        }
        // Keep only what follows the last folder separator in literal text.
        if let Some(i) = parts.iter().rposition(|p| p.contains(['/', '\\'])) {
            let tail = parts[i]
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or("")
                .to_string();
            parts.drain(..i);
            parts[0] = tail;
        }
        RunTimeName { parts }
    }

    /// True if the literal text, less a trailing file extension, says
    /// anything: `bomb#v(n).png` can be matched, `#s(pic)` and
    /// `#v(n).png` would match every picture and cannot.
    pub fn is_specific(&self) -> bool {
        let mut parts = self.parts.clone();
        if let Some(last) = parts.last_mut() {
            if let Some(dot) = last.rfind('.') {
                if last[dot + 1..].chars().all(|c| c.is_ascii_alphanumeric()) {
                    last.truncate(dot);
                }
            }
        }
        parts.iter().any(|p| !p.is_empty())
    }

    /// True if `file_name` (a name with no folder) could be this name.
    pub fn matches(&self, file_name: &str) -> bool {
        let name = file_name.to_lowercase();
        let (first, rest) = self.parts.split_first().expect("at least one part");
        let Some(mut left) = name.strip_prefix(first.as_str()) else {
            return false;
        };
        let Some((last, middle)) = rest.split_last() else {
            // No code at all: the whole name must match.
            return left.is_empty();
        };
        for part in middle {
            match left.find(part.as_str()) {
                Some(i) => left = &left[i + part.len()..],
                None => return false,
            }
        }
        left.ends_with(last.as_str())
    }
}

/// Which script language a script's text is written in (#38). GS1 is the
/// language every client runs; GS2 is the newer one, which only 3.153 and
/// later clients run (seihai #430).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptEra {
    /// Nothing in it is GS2: every client can run it.
    Gs1,
    /// GS2 code and no GS1 event flags.
    Gs2,
    /// GS1 event flags and GS2 code in the same script. A client without
    /// GS2 runs only the GS1 part; the GS2 lines trip its parser and are skipped.
    Both,
}

impl ScriptEra {
    pub fn label(self) -> &'static str {
        match self {
            ScriptEra::Gs1 => "GS1 only",
            ScriptEra::Gs2 => "GS2 only",
            ScriptEra::Both => "both GS1 and GS2",
        }
    }
}

/// GS1 event flags: in GS1 a script tests these in `if (...)`. GS2 has
/// event functions (`function onCreated()`) instead.
const GS1_EVENT_FLAGS: &[&str] = &[
    "created",
    "playerenters",
    "playertouchsme",
    "playerchats",
    "weaponfired",
    "actionserverside",
    "playerhurt",
    "wasshot",
    "washit",
    "wasthrown",
    "exploded",
    "playerdies",
    "playerendsreading",
    "playerlogin",
];

/// The script language of a script's text, from markers each language has
/// and the other does not. `None` for an empty script.
///
/// - **GS2**: an event function (`function on` + capital letter, such as
///   `function onCreated()`), `temp.` variables, `new Name(` objects,
///   `player.` properties and `params[`.
/// - **GS1**: event flags used as words (`if (playerenters)`) and message
///   codes outside strings (`#c`, `#p(0)`).
///
/// A script with neither is GS1: plain commands run on every client.
/// Comments and string literals are ignored. This reads the text only; it
/// does not compile anything, so a script that only calls GS2 functions
/// through GS1-looking commands counts as GS1.
pub fn script_era(src: &str) -> Option<ScriptEra> {
    let code = strip_comments_and_strings(src);
    if code.trim().is_empty() {
        return None;
    }
    let words = words(&code);
    let mut gs1 = false;
    let mut gs2 = false;
    for (i, &(start, word)) in words.iter().enumerate() {
        let before = code[..start].chars().next_back();
        let after_at = start + word.len();
        let after = code[after_at..].trim_start();
        let next = words.get(i + 1).map(|&(_, w)| w);
        let lower = word.to_ascii_lowercase();
        if before == Some('.') {
            continue;
        }
        if before == Some('#') {
            // `#c`, `#p(0)`: a GS1 message code outside a string.
            gs1 = true;
            continue;
        }
        match lower.as_str() {
            "function" => {
                if let Some(name) = next {
                    let mut c = name.chars();
                    if c.next() == Some('o')
                        && c.next() == Some('n')
                        && c.next().is_some_and(|c| c.is_ascii_uppercase())
                    {
                        gs2 = true;
                    }
                }
            }
            "temp" | "player" if after.starts_with('.') => gs2 = true,
            "params" if after.starts_with('[') => gs2 = true,
            "new" => {
                if let Some(&(nstart, name)) = words.get(i + 1) {
                    let between = &code[after_at..nstart];
                    if between.trim().is_empty()
                        && code[nstart + name.len()..].trim_start().starts_with('(')
                    {
                        gs2 = true;
                    }
                }
            }
            flag if GS1_EVENT_FLAGS.contains(&flag)
                && !after.starts_with('(')
                && !after.starts_with('=') =>
            {
                gs1 = true;
            }
            _ => {}
        }
    }
    Some(match (gs1, gs2) {
        (true, true) => ScriptEra::Both,
        (false, true) => ScriptEra::Gs2,
        _ => ScriptEra::Gs1,
    })
}

/// The script with `//` and `/* */` comments and the insides of string
/// literals blanked out (same byte length, so offsets still line up).
fn strip_comments_and_strings(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == b'/' && bytes.get(i + 1) == Some(&b'*') {
            out.push_str("  ");
            i += 2;
            while i < bytes.len() && !(bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/')) {
                out.push(if bytes[i] == b'\n' { '\n' } else { ' ' });
                i += 1;
            }
            if i < bytes.len() {
                out.push_str("  ");
                i += 2;
            }
        } else if c == b'"' {
            out.push('"');
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\n' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    out.push(' ');
                    i += 1;
                }
                out.push(' ');
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'"' {
                out.push('"');
                i += 1;
            }
        } else if c.is_ascii() {
            out.push(c as char);
            i += 1;
        } else {
            // Non-ASCII: keep the byte count with spaces.
            let len = src[i..].chars().next().map_or(1, char::len_utf8);
            for _ in 0..len {
                out.push(' ');
            }
            i += len;
        }
    }
    out
}

/// Identifier-like words and their byte offsets.
fn words(code: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in code.char_indices() {
        let word_char = c.is_ascii_alphanumeric() || c == '_';
        match (word_char, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, &code[s..i]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, &code[s..]));
    }
    out
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
    fn run_time_names_are_kept_apart() {
        let c = check_script("setimg #s(this.pic);\nplay #v(this.n).wav;\nsetimg a.png;\n");
        assert_eq!(names(&c), [(FileKind::Image, "a.png")]);
        let run_time: Vec<_> = c
            .run_time
            .iter()
            .map(|f| (f.kind, f.name.as_str(), f.line))
            .collect();
        assert_eq!(
            run_time,
            [
                (FileKind::Image, "#s(this.pic)", 1),
                (FileKind::Sound, "#v(this.n).wav", 2),
            ]
        );
    }

    #[test]
    fn run_time_names_match_as_patterns() {
        let bomb = RunTimeName::parse("bomb#v(this.n).png");
        assert!(bomb.is_specific());
        assert!(bomb.matches("bomb3.png"));
        assert!(bomb.matches("BOMB12.PNG"));
        assert!(bomb.matches("bomb.png"));
        assert!(!bomb.matches("bomb3.gif"));
        assert!(!bomb.matches("xbomb3.png"));

        // Two codes, nested brackets, and a folder in front.
        let door = RunTimeName::parse("levels/door_#s(client.c)_#v((a+1)*2).png");
        assert!(door.is_specific());
        assert!(door.matches("door_red_4.png"));
        assert!(!door.matches("door4.png"));

        // A folder inside a code is not a folder in the name.
        let sub = RunTimeName::parse("#s(dir)/tree#v(n).gif");
        assert!(sub.matches("tree2.gif"));
        assert!(!sub.matches("bush2.gif"));

        // Codes without brackets: #c, #1.
        assert!(RunTimeName::parse("head#1.png").matches("head19.png"));

        // Nothing but codes and an extension says nothing.
        assert!(!RunTimeName::parse("#s(this.pic)").is_specific());
        assert!(!RunTimeName::parse("#v(n).png").is_specific());
        assert!(RunTimeName::parse("#v(n)_hat.png").is_specific());
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

    #[test]
    fn script_era_reads_markers_outside_comments_and_strings() {
        use super::{script_era, ScriptEra::*};
        assert_eq!(script_era(""), None);
        assert_eq!(script_era("// only a comment\n"), None);
        assert_eq!(script_era("if (playerenters) { message #c; }"), Some(Gs1));
        assert_eq!(script_era("setimg tree.png;"), Some(Gs1));
        assert_eq!(script_era("this.count = 1; client.flag = 1;"), Some(Gs1));
        assert_eq!(
            script_era("function onCreated() { this.x = 1; }"),
            Some(Gs2)
        );
        assert_eq!(
            script_era("function onPlayerEnters() {\n  player.chat = \"hi\";\n}"),
            Some(Gs2)
        );
        assert_eq!(script_era("temp.a = 1;"), Some(Gs2));
        assert_eq!(script_era("x = new TStaticVar();"), Some(Gs2));
        assert_eq!(script_era("echo(params[0]);"), Some(Gs2));
        // A GS1 function with a plain name is still GS1.
        assert_eq!(
            script_era("function walk() { x += 1; }\nif (created) walk();"),
            Some(Gs1)
        );
        assert_eq!(
            script_era("if (created) { message hi; }\nfunction onPlayerChats() { echo(1); }"),
            Some(Both)
        );
        // Markers inside comments and strings do not count.
        assert_eq!(
            script_era("// function onCreated()\nmessage \"temp.a #c\";"),
            Some(Gs1)
        );
        assert_eq!(
            script_era("/* if (created) */ function onCreated() {}"),
            Some(Gs2)
        );
        // An event function's name is not a GS1 flag; neither is a call.
        assert_eq!(script_era("function onCreated() { created(); }"), Some(Gs2));
    }
}
