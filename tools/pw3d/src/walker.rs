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
//! * moves a quarter tile a step unless the tile ahead is a wall, and sets
//!   `player.position` (5.12 sends its own object's transform on);
//! * keeps a free camera behind and above the player;
//! * on a link, switches world (`g3dworld`) when the link leaves it and puts
//!   the player where the link lands;
//! * shows a sign's text while the player stands below it.
//!
//! The grid is one run-length string a row: a count, then `.` floor, `~`
//! water, `^` lava or `#` wall (and gmap holes).

use crate::convert::{Ground, World};

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

/// The whole weapon for these worlds.
pub fn script(worlds: &[World], wall: f32) -> String {
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
        "function pw3dKind(x, y) {{\n\
         \x20 if (x < 0 || y < 0 || x >= this.w || y >= this.h) return \"#\";\n\
         \x20 temp.row = this.rows[y];\n\
         \x20 temp.n = 0;\n\
         \x20 temp.at = 0;\n\
         \x20 for (temp.i = 0; temp.i < temp.row.length(); temp.i++) {{\n\
         \x20   temp.c = temp.row.charat(temp.i);\n\
         \x20   if (temp.c == \".\" || temp.c == \"~\" || temp.c == \"^\" || temp.c == \"#\") {{\n\
         \x20     temp.at += temp.n;\n\
         \x20     if (x < temp.at) return temp.c;\n\
         \x20     temp.n = 0;\n\
         \x20   }} else temp.n = temp.n * 10 + temp.c;\n\
         \x20 }}\n\
         \x20 return \"#\";\n\
         }}\n\n\
         function pw3dZ(k) {{\n\
         \x20 if (k == \"~\") return {water} + 1;\n\
         \x20 if (k == \"^\") return {lava} + 1;\n\
         \x20 return 1;\n\
         }}\n\n\
         function pw3dStep(dx, dy) {{\n\
         \x20 temp.nx = this.px + dx * this.step;\n\
         \x20 temp.ny = this.py + dy * this.step;\n\
         \x20 if (temp.nx < 0 || temp.ny < 0) return 0;\n\
         \x20 if (pw3dKind(int(temp.nx), int(temp.ny)) == \"#\") return 0;\n\
         \x20 this.px = temp.nx;\n\
         \x20 this.py = temp.ny;\n\
         \x20 return 1;\n\
         }}\n\n\
         function pw3dPlace() {{\n\
         \x20 temp.z = pw3dZ(pw3dKind(int(this.px), int(this.py)));\n\
         \x20 player.position = {{this.px, 0 - this.py, temp.z}};\n\
         \x20 pw3dCamera(this.px, 0 - this.py, temp.z);\n\
         }}\n\n\
         // The camera 5 behind (south of) the player and 3 above, looking\n\
         // north and down (the lab #458 walker's camera).\n\
         function pw3dCamera(x, y, z) {{\n\
         \x20 temp.cam = ServerConnection.camera;\n\
         \x20 if (temp.cam == NULL) return;\n\
         \x20 temp.cam.cameramode = \"free\";\n\
         \x20 temp.cam.position = format(\"%f %f %f\", x, y - 5, z + 3);\n\
         \x20 temp.cam.eulerrotation = \"0.5 0 0\";\n\
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
         // l: x, y, w, h, world, dest origin x, y, dest x, y (\"k\" keeps the\n\
         // coordinate), keep offset x, y, origin x, y of the link's level.\n\
         function pw3dTake(l) {{\n\
         \x20 if (l[7] == \"k\") temp.nx = this.px - l[11] + l[9] + l[5];\n\
         \x20 else temp.nx = l[7] + l[5];\n\
         \x20 if (l[8] == \"k\") temp.ny = this.py - l[12] + l[10] + l[6];\n\
         \x20 else temp.ny = l[8] + l[6];\n\
         \x20 if (l[4] != this.world) {{\n\
         \x20   triggerserver(\"gui\", \"{WEAPON}\", \"g3dworld\", l[4]);\n\
         \x20   pw3dLoad(l[4]);\n\
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
         \x20 if (temp.text == \"\") hideimg(200);\n\
         \x20 else showtext(200, 20, 20, \"Arial\", \"b\", temp.text);\n\
         }}\n\n",
        water = num(Ground::Water.height(wall)),
        lava = num(Ground::Lava.height(wall)),
    ));
    // The worlds' data.
    t.push_str("function pw3dLoad(name) {\n  this.world = name;\n");
    for (i, w) in worlds.iter().enumerate() {
        t.push_str(&format!(
            "  {}if (name == {}) {{\n    this.w = {};\n    this.h = {};\n    this.rows = {{\n",
            if i == 0 { "" } else { "else " },
            lit(&w.name),
            w.tiles_w(),
            w.tiles_h()
        ));
        let rows: Vec<String> = (0..w.tiles_h() as i32)
            .map(|y| {
                let row = rle_row(
                    (0..w.tiles_w() as i32).map(|x| w.at(x, y).map_or('#', Ground::letter)),
                );
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
            .map(|s| format!("      {{{}, {}, {}}}", s.x, s.y, lit(&s.text)))
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
    fn literals_are_escaped() {
        assert_eq!(lit("say \"hi\"\nnow"), "\"say \\\"hi\\\" now\"");
    }
}
