//! Sign text as 3D letters.
//!
//! A 3D client draws neither 2D signs nor chat bubbles over its own player
//! (5.12 lab re-run, 2026-11-26), so a sign's words stand on a board above
//! the sign as small square pixels, in our own 3×5 capital font, facing the
//! camera (south).

use super::shape::{Colour, Shape};

/// One pixel's size, in tiles.
const PX: f32 = 0.06;
/// Letters on one line at most, before wrapping at a space.
const WIDTH: usize = 22;
/// Lines shown; longer text ends with "...".
const LINES: usize = 8;

const BOARD: Colour = Colour([225, 205, 150]);
const INK: Colour = Colour([40, 30, 20]);

/// A glyph: five rows of three bits, the top row first, the left column
/// the high bit.
fn glyph(c: char) -> Option<[u8; 5]> {
    Some(match c.to_ascii_uppercase() {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b101, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b110, 0b001, 0b010, 0b100, 0b111],
        '3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        '6' => [0b011, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b110],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
        '!' => [0b010, 0b010, 0b010, 0b000, 0b010],
        '?' => [0b110, 0b001, 0b010, 0b000, 0b010],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '\'' => [0b010, 0b010, 0b000, 0b000, 0b000],
        '"' => [0b101, 0b101, 0b000, 0b000, 0b000],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '(' => [0b001, 0b010, 0b010, 0b010, 0b001],
        ')' => [0b100, 0b010, 0b010, 0b010, 0b100],
        '<' => [0b001, 0b010, 0b100, 0b010, 0b001],
        '>' => [0b100, 0b010, 0b001, 0b010, 0b100],
        '=' => [0b000, 0b111, 0b000, 0b111, 0b000],
        _ => return None,
    })
}

/// The sign's words in lines: `#b` and line breaks become spaces, `#` codes
/// with arguments are dropped, letters the font lacks become spaces.
pub fn lines(text: &str, width: usize) -> Vec<String> {
    let width = width.clamp(4, WIDTH);
    let mut clean = String::new();
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        if c == '#' {
            match it.next() {
                Some('#') => clean.push('#'),
                Some(_) => {
                    clean.push(' ');
                    if it.peek() == Some(&'(') {
                        for d in it.by_ref() {
                            if d == ')' {
                                break;
                            }
                        }
                    }
                }
                None => {}
            }
        } else if glyph(c).is_some() {
            clean.push(c.to_ascii_uppercase());
        } else {
            clean.push(' ');
        }
    }
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in clean.split_whitespace() {
        let mut word = word.to_string();
        while word.len() > width {
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            let rest = word.split_off(width);
            out.push(word);
            word = rest;
        }
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    if out.len() > LINES {
        out.truncate(LINES);
        let last = &mut out[LINES - 1];
        while last.len() + 3 > width {
            last.pop();
        }
        last.push_str("...");
    }
    out
}

/// A board with the sign's words, its bottom centre at `(cx, y, z)`, its
/// face at world `y` looking towards -y, at most `max_w` wide (so boards of
/// signs side by side do not overlap). Nothing for a sign with no words.
pub fn board(text: &str, cx: f32, y: f32, z: f32, max_w: f32) -> Shape {
    let mut s = Shape::default();
    let fit = ((max_w / PX - 1.0) / 4.0).floor().max(0.0) as usize;
    let lines = lines(text, fit);
    if lines.is_empty() {
        return s;
    }
    let cols = lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let w = (cols * 4 + 1) as f32 * PX;
    let h = (lines.len() * 6 + 1) as f32 * PX;
    let x0 = cx - w / 2.0;
    s.block([x0, y, z], [x0 + w, y + 0.06, z + h], BOARD, BOARD);
    let fy = y - 0.01;
    for (li, line) in lines.iter().enumerate() {
        // The top pixel row of this line, counted down from the board's top.
        let top = z + h - PX * (1 + li * 6) as f32;
        for (ci, c) in line.chars().enumerate() {
            let Some(g) = glyph(c) else { continue };
            let lx = x0 + PX * (1 + ci * 4) as f32;
            for (r, bits) in g.iter().enumerate() {
                let z1 = top - PX * r as f32;
                let z0 = z1 - PX;
                // Runs of lit pixels in the row, one quad each.
                let mut col = 0;
                while col < 3 {
                    if bits & (0b100 >> col) == 0 {
                        col += 1;
                        continue;
                    }
                    let start = col;
                    while col < 3 && bits & (0b100 >> col) != 0 {
                        col += 1;
                    }
                    let a = lx + PX * start as f32;
                    let b = lx + PX * col as f32;
                    s.quad([[a, fy, z0], [b, fy, z0], [b, fy, z1], [a, fy, z1]], INK);
                }
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_wrapping() {
        assert_eq!(
            lines("Welcome to#bArynala!", WIDTH),
            vec!["WELCOME TO ARYNALA!"]
        );
        assert_eq!(lines("a #c(red)b", WIDTH), vec!["A B"]);
        let l = lines("one two three four five six seven eight nine", WIDTH);
        assert!(l.len() > 1 && l.iter().all(|x| x.len() <= WIDTH));
        assert!(lines("", WIDTH).is_empty());
        let long = "word ".repeat(100);
        let l = lines(&long, WIDTH);
        assert_eq!(l.len(), LINES);
        assert!(l[LINES - 1].ends_with("..."));
    }

    #[test]
    fn board_has_letters() {
        let s = board("HI", 5.0, -3.0, 1.0, 9.0);
        // The board's 5 faces as 10 triangles, then the ink.
        assert!(s.faces.len() > 10);
        assert!(s.faces.iter().any(|f| f.1 == INK));
        assert!(board("   ", 0.0, 0.0, 0.0, 9.0).faces.is_empty());
        let w: Vec<f32> = s.points.iter().map(|p| p[0]).collect();
        let (lo, hi) = w
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &x| (a.min(x), b.max(x)));
        assert!((lo + hi) / 2.0 - 5.0 < 0.01);
        // Narrow room: the board stays inside it and wraps instead.
        let t = "a long sign that has to wrap into a narrow board";
        let n = board(t, 0.0, 0.0, 0.0, 2.0);
        assert!(n.points.iter().all(|p| p[0].abs() <= 1.0 + 1e-4));
        assert!(lines(t, 7).iter().all(|l| l.len() <= 7));
    }
}
