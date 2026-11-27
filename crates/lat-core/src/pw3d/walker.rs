//! The `PW3DWalk` weapon: a client-side GS2 script that walks the player
//! through the converted worlds.
//!
//! A 5.x client in 3D mode never moves its own player and has no collision
//! (Graal_analysis `torque/3d-scoping-interiors-limits-512-2026-11-26.md`),
//! and it runs no level links. So the weapon carries each world's walk grid
//! and links, and:
//!
//! * reads the arrow keys every tick (`keydown2`), as the lab #458 walker
//!   did on a real 5.12;
//! * moves about 5 tiles a second (by the clock, so a slow tick rate does
//!   not slow the walk) unless the tile ahead is a wall, and sets
//!   `player.position` (5.12 sends its own object's transform on);
//! * keeps a free camera behind and above the player, in front of any wall
//!   behind it;
//! * on a link, loads the walk grid of the level or gmap it leads to and
//!   puts the player where the link lands; an interior placed inside the
//!   world it is entered from (`convert::place_interiors`) is only a move,
//!   and `g3dworld` is sent only when the server world changes;
//! * shows a sign's text while the player stands below it.
//!
//! The grid is one run-length string a row: a count, then `#` for a tile
//! the player cannot enter (walls, gmap holes) or a letter for the height
//! it stands at there (`a` is -1, each letter a quarter unit higher).

use super::convert::World;

/// Walkable heights as letters, a quarter unit each from -1.
const ALPHA: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// The letter for standing height `z`.
pub fn letter(z: f32) -> char {
    let k = ((z + 1.0) / 0.25).round().clamp(0.0, 51.0) as usize;
    ALPHA.as_bytes()[k] as char
}

pub const WEAPON: &str = "PW3DWalk";
/// Relative to the server's world folder (`weapon3d` files are).
pub const FILE: &str = "weapons/pw3dwalk.gs2";

/// One grid row as runs.
pub fn rle_row(cells: impl Iterator<Item = char>) -> String {
    let mut out = String::new();
    let mut cur: Option<(char, usize)> = None;
    for c in cells {
        match &mut cur {
            Some((k, n)) if *k == c => *n += 1,
            _ => {
                if let Some((k, n)) = cur {
                    out.push_str(&format!("{n}{k}"));
                }
                cur = Some((c, 1));
            }
        }
    }
    if let Some((k, n)) = cur {
        out.push_str(&format!("{n}{k}"));
    }
    out
}

/// A GS2 string literal.
fn lit(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' | '\t' => out.push(' '),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            _ => out.push('?'),
        }
    }
    out.push('"');
    out
}

fn num(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn opt(v: Option<f32>) -> String {
    v.map_or_else(|| "\"k\"".to_string(), num)
}

/// A sign's text for one line: GS1's `#b` line breaks become spaces.
fn sign_text(t: &str) -> String {
    t.replace("#b", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The whole weapon for these worlds.
pub fn script(worlds: &[World]) -> String {
    let start = worlds
        .iter()
        .find(|w| w.name == "default")
        .unwrap_or(&worlds[0]);
    let mut t = String::new();
    t.push_str(&format!(
        "// {WEAPON}: walks the player through a playerworld converted by seihai-pw3d.\n\
         // Generated; edit the converter, not this file.\n\
         //#CLIENTSIDE\n\
         function onCreated() {{\n\
         \x20 this.step = 0.25;\n\
         \x20 this.signshown = \"\";\n\
         \x20 pw3dLoad(\"default\");\n\
         \x20 this.px = {sx};\n\
         \x20 this.py = {sy};\n\
         \x20 this.away = 0;\n\
         \x20 this.settle = 8;\n\
         \x20 this.ticks = 0;\n\
         \x20 this.wantworld = \"\";\n\
         \x20 setTimer(0.05);\n\
         }}\n\n\
         // The arrows, read each tick: the 3D key bindings end up as keys too\n\
         // (lab #458, real 5.12), and nothing in the client moves the player.\n\
         function onTimeout() {{\n\
         \x20 temp.dx = 0;\n\
         \x20 temp.dy = 0;\n\
         \x20 if (keydown2(37, true)) temp.dx = temp.dx - 1;\n\
         \x20 if (keydown2(39, true)) temp.dx = temp.dx + 1;\n\
         \x20 if (keydown2(38, true)) temp.dy = temp.dy - 1;\n\
         \x20 if (keydown2(40, true)) temp.dy = temp.dy + 1;\n\
         \x20 // About 5 tiles a second whatever the tick rate (a loaded 5.12\n\
         \x20 // ticks at 7 to 13 Hz, not 20); a quarter tile a tick if the\n\
         \x20 // clock is not there.\n\
         \x20 temp.now = timevar2;\n\
         \x20 this.step = 0.25;\n\
         \x20 if (temp.now > 0 && this.last > 0) this.step = (temp.now - this.last) * 5;\n\
         \x20 this.last = temp.now;\n\
         \x20 if (this.step > 0.5) this.step = 0.5;\n\
         \x20 if (this.step < 0.05) this.step = 0.05;\n\
         \x20 // A real 5.12 drops triggerserver in its first ticks (lab), so a\n\
         \x20 // world change made that early is told to the server later.\n\
         \x20 if (this.ticks <= 64) this.ticks++;\n\
         \x20 if (this.ticks > 64 && this.wantworld != \"\") {{\n\
         \x20   triggerserver(\"gui\", \"{WEAPON}\", \"g3dworld\", this.wantworld @ \"\");\n\
         \x20   this.wantworld = \"\";\n\
         \x20 }}\n\
         \x20 temp.moved = 0;\n\
         \x20 if (temp.dx != 0) temp.moved = temp.moved + pw3dStep(temp.dx, 0);\n\
         \x20 if (temp.dy != 0) temp.moved = temp.moved + pw3dStep(0, temp.dy);\n\
         \x20 if (temp.moved > 0 || this.settle > 0) {{\n\
         \x20   if (this.settle > 0) this.settle--;\n\
         \x20   pw3dPlace();\n\
         \x20 }}\n\
         \x20 if (temp.moved > 0) {{\n\
         \x20   pw3dLinks();\n\
         \x20   pw3dSigns();\n\
         \x20 }}\n\
         \x20 setTimer(0.05);\n\
         }}\n\n",
        sx = num(start.spawn.0),
        sy = num(start.spawn.1)
    ));
    t.push_str(&format!(
        "// A real 5.12 running seihai-core's bytecode misreads a variable or\n\
         // parameter given straight to .pos(), .charat() or an index (lab,\n\
         // 2026-11-26): each goes through a temp or an expression.\n\
         function pw3dKind(x, y) {{\n\
         \x20 temp.x = x + 0;\n\
         \x20 temp.y = y + 0;\n\
         \x20 if (temp.x < 0 || temp.y < 0 || temp.x >= this.w || temp.y >= this.h) return \"#\";\n\
         \x20 temp.row = this.rows[temp.y + 0];\n\
         \x20 temp.n = 0;\n\
         \x20 temp.at = 0;\n\
         \x20 for (temp.i = 0; temp.i < temp.row.length(); temp.i++) {{\n\
         \x20   temp.c = temp.row.charat(temp.i + 0);\n\
         \x20   if (\"0123456789\".pos(temp.c @ \"\") < 0) {{\n\
         \x20     temp.at += temp.n;\n\
         \x20     if (temp.x < temp.at) return temp.c;\n\
         \x20     temp.n = 0;\n\
         \x20   }} else temp.n = temp.n * 10 + temp.c;\n\
         \x20 }}\n\
         \x20 return \"#\";\n\
         }}\n\n\
         // A tile's letter is its height: {alpha}, a quarter unit\n\
         // each from -1; the player's origin is 1 above its feet.\n\
         function pw3dZ(k) {{\n\
         \x20 return \"{alpha}\".pos(k @ \"\") * 0.25;\n\
         }}\n\n\
         function pw3dStep(dx, dy) {{\n\
         \x20 temp.dx = dx + 0;\n\
         \x20 temp.dy = dy + 0;\n\
         \x20 temp.nx = this.px + temp.dx * this.step;\n\
         \x20 temp.ny = this.py + temp.dy * this.step;\n\
         \x20 if (temp.nx < 0 || temp.ny < 0) return 0;\n\
         \x20 if (pw3dKind(int(temp.nx), int(temp.ny)) == \"#\") return 0;\n\
         \x20 this.px = temp.nx;\n\
         \x20 this.py = temp.ny;\n\
         \x20 return 1;\n\
         }}\n\n\
         function pw3dPlace() {{\n\
         \x20 temp.z = pw3dZ(pw3dKind(int(this.px), int(this.py)));\n\
         \x20 // An interior's grid is its own; it stands at (ax, ay) in the world.\n\
         \x20 temp.wx = this.px + this.ax;\n\
         \x20 temp.wy = this.py + this.ay;\n\
         \x20 player.position = {{temp.wx, 0 - temp.wy, temp.z}};\n\
         \x20 // The camera comes in front of the first wall (or ground over\n\
         \x20 // 2 higher) behind the player, so it is never inside one.\n\
         \x20 temp.back = 6;\n\
         \x20 for (temp.d = 1; temp.d <= 6; temp.d++) {{\n\
         \x20   temp.k = pw3dKind(int(this.px), int(this.py + temp.d));\n\
         \x20   if (temp.k == \"#\" || pw3dZ(temp.k) > temp.z + 2) {{\n\
         \x20     temp.back = int(this.py) + temp.d - this.py - 0.2;\n\
         \x20     break;\n\
         \x20   }}\n\
         \x20 }}\n\
         \x20 if (temp.back < 0.8) temp.back = 0.8;\n\
         \x20 pw3dCamera(temp.wx, 0 - temp.wy, temp.z, temp.back);\n\
         }}\n\n\
         // The camera up to 6 behind (south of) the player and 2.5 above, looking\n\
         // north and a little down: a steeper look made upright things\n\
         // lean outward on a real 5.12 (lab, 2026-11-26).\n\
         function pw3dCamera(x, y, z, back) {{\n\
         \x20 temp.x = x + 0;\n\
         \x20 temp.y = y + 0;\n\
         \x20 temp.z = z + 0;\n\
         \x20 temp.back = back + 0;\n\
         \x20 temp.up = 1.2 + temp.back * 0.22;\n\
         \x20 temp.cam = ServerConnection.camera;\n\
         \x20 if (temp.cam == NULL) return;\n\
         \x20 temp.cam.cameramode = \"free\";\n\
         \x20 temp.cam.position = format(\"%f %f %f\", temp.x, temp.y - temp.back, temp.z + temp.up);\n\
         \x20 temp.cam.eulerrotation = \"0.3 0 0\";\n\
         \x20 temp.cam.updatePosition();\n\
         }}\n\n\
         function pw3dLinks() {{\n\
         \x20 temp.inany = 0;\n\
         \x20 for (temp.l: this.links) {{\n\
         \x20   if (this.px >= temp.l[0] && this.px < temp.l[0] + temp.l[2] && this.py >= temp.l[1] && this.py < temp.l[1] + temp.l[3]) {{\n\
         \x20     temp.inany = 1;\n\
         \x20     if (this.away) {{\n\
         \x20       pw3dTake(temp.l);\n\
         \x20       return;\n\
         \x20     }}\n\
         \x20   }}\n\
         \x20 }}\n\
         \x20 if (!temp.inany) this.away = 1;\n\
         }}\n\n\
         // l: x, y, w, h, the level or gmap it leads to, dest origin x, y, dest x, y (\"k\" keeps the\n\
         // coordinate), keep offset x, y, origin x, y of the link's level.\n\
         function pw3dTake(l) {{\n\
         \x20 temp.l = l;\n\
         \x20 if (temp.l[7] == \"k\") temp.nx = this.px - temp.l[11] + temp.l[9] + temp.l[5];\n\
         \x20 else temp.nx = temp.l[7] + temp.l[5];\n\
         \x20 if (temp.l[8] == \"k\") temp.ny = this.py - temp.l[12] + temp.l[10] + temp.l[6];\n\
         \x20 else temp.ny = temp.l[8] + temp.l[6];\n\
         \x20 temp.was = this.world;\n\
         \x20 if (temp.l[4] != this.area) pw3dLoad(temp.l[4]);\n\
         \x20 // Into an interior of this world or out again is a move; only a\n\
         \x20 // different server world needs the server.\n\
         \x20 if (this.world != temp.was) {{\n\
         \x20   if (this.ticks > 64) triggerserver(\"gui\", \"{WEAPON}\", \"g3dworld\", this.world @ \"\");\n\
         \x20   else this.wantworld = this.world @ \"\";\n\
         \x20 }}\n\
         \x20 this.px = temp.nx;\n\
         \x20 this.py = temp.ny;\n\
         \x20 this.away = 0;\n\
         \x20 this.settle = 8;\n\
         }}\n\n\
         function pw3dSigns() {{\n\
         \x20 temp.tx = int(this.px);\n\
         \x20 temp.ty = int(this.py);\n\
         \x20 temp.text = \"\";\n\
         \x20 for (temp.s: this.signs) {{\n\
         \x20   if (temp.tx >= temp.s[0] - 1 && temp.tx <= temp.s[0] + 1 && temp.ty >= temp.s[1] && temp.ty <= temp.s[1] + 1) temp.text = temp.s[2];\n\
         \x20 }}\n\
         \x20 if (temp.text == this.signshown) return;\n\
         \x20 this.signshown = temp.text;\n\
         \x20 // showtext draws nothing in 5.12's 3D view (lab, 2026-11-26); the\n\
         \x20 // player's chat line is tried as well.\n\
         \x20 player.chat = temp.text;\n\
         \x20 if (temp.text == \"\") hideimg(200);\n\
         \x20 else showtext(200, 20, 20, \"Arial\", \"b\", temp.text);\n\
         }}\n\n",
        alpha = ALPHA,
    ));
    // The worlds' data.
    t.push_str(
        // Not temp.name: on a real 5.12 that reads back as the function's
        // own name (lab, 2026-11-26).
        "function pw3dLoad(name) {\n  temp.wname = name @ \"\";\n  this.area = temp.wname;\n",
    );
    for (i, w) in worlds.iter().enumerate() {
        t.push_str(&format!(
            "  {}if (temp.wname == {}) {{\n    this.world = {};\n    this.ax = {};\n    this.ay = {};\n    this.w = {};\n    this.h = {};\n    this.rows = {{\n",
            if i == 0 { "" } else { "else " },
            lit(&w.name),
            lit(w.server_world()),
            w.offset().0,
            w.offset().1,
            w.tiles_w(),
            w.tiles_h()
        ));
        let rows: Vec<String> = (0..w.tiles_h() as i32)
            .map(|y| {
                let row =
                    rle_row((0..w.tiles_w() as i32).map(|x| w.walk_at(x, y).map_or('#', letter)));
                format!("      \"{row}\"")
            })
            .collect();
        t.push_str(&rows.join(",\n"));
        t.push_str("\n    };\n    this.links = {\n");
        let links: Vec<String> = w
            .links
            .iter()
            .map(|l| {
                format!(
                    "      {{{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}}}",
                    l.x,
                    l.y,
                    l.w,
                    l.h,
                    lit(&l.world),
                    l.dest_origin.0,
                    l.dest_origin.1,
                    opt(l.dest_x),
                    opt(l.dest_y),
                    num(l.keep_offset.0),
                    num(l.keep_offset.1),
                    l.from_origin.0,
                    l.from_origin.1
                )
            })
            .collect();
        t.push_str(&links.join(",\n"));
        t.push_str("\n    };\n    this.signs = {\n");
        let signs: Vec<String> = w
            .signs
            .iter()
            .map(|s| format!("      {{{}, {}, {}}}", s.x, s.y, lit(&sign_text(&s.text))))
            .collect();
        t.push_str(&signs.join(",\n"));
        t.push_str("\n    };\n  }\n");
    }
    t.push_str("}\n");
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_runs() {
        assert_eq!(rle_row("...##.".chars()), "3.2#1.");
        assert_eq!(rle_row("".chars()), "");
    }

    #[test]
    fn sign_lines_join() {
        assert_eq!(sign_text("Hello#bthere #b  you"), "Hello there you");
    }

    #[test]
    fn literals_are_escaped() {
        assert_eq!(lit("say \"hi\"\nnow"), "\"say \\\"hi\\\" now\"");
    }
}
