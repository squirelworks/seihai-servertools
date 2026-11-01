//! The LAT level editor: open a level, change a tile, write it back.
//!
//! The bytes are parsed with `seihai-core` (`Level::from_bytes`). There is
//! no second parser here. `seihai-core` does not write a level back, and the
//! client's offline editor (`tools/level-editor` in the Seihai repo) is not
//! a library, so [`encode`] writes a `GLEVNW01` text level from the parsed
//! value. An unchanged file is not byte-for-byte the same: comments go,
//! and a binary level (`.graal`) comes back as text. The tiles, NPCs,
//! links, signs, baddies and chests are kept.
//!
//! This is the live-server editor. The client's offline editor stays in
//! the Seihai repo.

use seihai_core::assets::{tile_code, BoardRun};

use crate::preview::{paint_level, BoardView, Rgba, Sheet};

pub use seihai_core::assets::Level;

/// Parse a `.nw` (`GLEVNW01`) or `.graal` level.
pub fn open(bytes: &[u8]) -> Result<Level, String> {
    Level::from_bytes(bytes)
}

/// A level open for editing.
#[derive(Debug, Clone)]
pub struct LevelDoc {
    level: Level,
    dirty: bool,
}

impl LevelDoc {
    /// Parse `bytes`.
    pub fn open(bytes: &[u8]) -> Result<Self, String> {
        Ok(Self {
            level: open(bytes)?,
            dirty: false,
        })
    }

    /// The parsed level.
    pub fn level(&self) -> &Level {
        &self.level
    }

    /// True after [`Self::set_tile`] until the next [`Self::open`].
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Layer-0 tile id at `(x, y)`, if the level has a tile there.
    pub fn tile(&self, x: i32, y: i32) -> Option<u16> {
        self.level.tile_id_at(x, y)
    }

    /// Put tile `id` at `(x, y)` on layer 0.
    ///
    /// A cell the level does not define yet gains a one-tile run. `id`
    /// above 4095, and a cell outside the 64 by 64 board, are refused and
    /// the level is left as it was.
    pub fn set_tile(&mut self, x: i32, y: i32, id: u16) -> Result<(), String> {
        set_tile(&mut self.level, x, y, id)?;
        self.dirty = true;
        Ok(())
    }

    /// `GLEVNW01` text for the level as it is now.
    pub fn encode(&self) -> Vec<u8> {
        encode(&self.level)
    }

    /// The board, painted by `seihai-draw`.
    pub fn paint(&self, sheet: Sheet<'_>, view: BoardView) -> Rgba {
        paint_level(&self.level, sheet, view)
    }
}

/// Put tile `id` at `(x, y)` on layer 0. See [`LevelDoc::set_tile`].
pub fn set_tile(level: &mut Level, x: i32, y: i32, id: u16) -> Result<(), String> {
    if !(0..64).contains(&x) || !(0..64).contains(&y) {
        return Err(format!("tile {x},{y} is outside the 64 by 64 board"));
    }
    if id > 4095 {
        return Err(format!("tile id {id} is above 4095"));
    }
    if level.set_tile_id(x, y, id) {
        return Ok(());
    }
    level.boards.push(BoardRun {
        x,
        y,
        width: 1,
        layer: 0,
        tiles: vec![tile_code(id)],
    });
    Ok(())
}

/// Write `level` as `GLEVNW01` text that [`open`] can read back.
pub fn encode(level: &Level) -> Vec<u8> {
    let mut out = Vec::new();
    push(&mut out, "GLEVNW01\n");
    for run in &level.boards {
        if run.tiles.is_empty() {
            continue;
        }
        push(
            &mut out,
            &format!(
                "BOARD {} {} {} {} ",
                run.x,
                run.y,
                run.tiles.len(),
                run.layer
            ),
        );
        for tile in &run.tiles {
            out.push(tile[0]);
            out.push(tile[1]);
        }
        out.push(b'\n');
    }
    for npc in &level.npcs {
        let image = if npc.image.is_empty() {
            "-"
        } else {
            npc.image.as_str()
        };
        push(
            &mut out,
            &format!("NPC {image} {} {}\n", float_text(npc.x), float_text(npc.y)),
        );
        if !npc.script.is_empty() {
            out.extend_from_slice(npc.script.as_bytes());
            if !npc.script.ends_with('\n') {
                out.push(b'\n');
            }
        }
        push(&mut out, "NPCEND\n");
    }
    for link in &level.links {
        push(
            &mut out,
            &format!(
                "LINK {} {} {} {} {} {} {}\n",
                link.dest,
                link.x,
                link.y,
                link.width,
                link.height,
                link_coord(link.new_x, link.x_offset, "playerx"),
                link_coord(link.new_y, link.y_offset, "playery"),
            ),
        );
    }
    for sign in &level.signs {
        push(&mut out, &format!("SIGN {} {}\n", sign.x, sign.y));
        if !sign.text.is_empty() {
            out.extend_from_slice(sign.text.as_bytes());
            if !sign.text.ends_with('\n') {
                out.push(b'\n');
            }
        }
        push(&mut out, "SIGNEND\n");
    }
    for baddy in &level.baddies {
        push(
            &mut out,
            &format!(
                "BADDY {} {} {}\n",
                float_text(baddy.x),
                float_text(baddy.y),
                baddy.kind
            ),
        );
        if !baddy.verses.is_empty() {
            out.extend_from_slice(baddy.verses.as_bytes());
            if !baddy.verses.ends_with('\n') {
                out.push(b'\n');
            }
        }
        push(&mut out, "BADDYEND\n");
    }
    for chest in &level.chests {
        push(
            &mut out,
            &format!(
                "CHEST {} {} {} {}\n",
                chest.x, chest.y, chest.item, chest.sign_index
            ),
        );
    }
    out
}

fn push(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(text.as_bytes());
}

fn float_text(n: f32) -> String {
    if n.is_finite() && n.fract() == 0.0 {
        format!("{}", n as i32)
    } else {
        n.to_string()
    }
}

fn link_coord(abs: Option<f32>, offset: f32, keep: &str) -> String {
    match abs {
        Some(n) => float_text(n),
        None if offset == 0.0 => keep.to_string(),
        None => {
            let number = float_text(offset);
            if offset > 0.0 {
                format!("{keep}+{number}")
            } else {
                format!("{keep}{number}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::{coloured_sheet, BoardView, Sheet};

    const LEVEL: &str = "\
GLEVNW01
BOARD 0 0 2 0 AAAA
NPC bush.png 3 4
setimg bush.png
NPCEND
LINK other.nw 0 10 1 4 62 playery
SIGN 2 3
hello
SIGNEND
BADDY 5 6 1
verse
BADDYEND
CHEST 7 8 greenrupee 0
";

    #[test]
    fn changing_one_tile_round_trips_through_the_parser() {
        let mut doc = LevelDoc::open(LEVEL.as_bytes()).unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(doc.tile(0, 0), Some(0));
        assert_eq!(doc.tile(1, 0), Some(0));
        doc.set_tile(1, 0, 1).unwrap();
        assert!(doc.is_dirty());
        assert!(doc.set_tile(-1, 0, 1).is_err());
        assert!(doc.set_tile(0, 0, 4096).is_err());
        assert_eq!(doc.tile(0, 0), Some(0), "a refused edit must not land");

        let encoded = doc.encode();
        let again = LevelDoc::open(&encoded).unwrap();
        assert_eq!(again.tile(0, 0), Some(0));
        assert_eq!(again.tile(1, 0), Some(1));
        assert_eq!(again.level().npcs.len(), 1);
        assert_eq!(again.level().npcs[0].image, "bush.png");
        assert!(again.level().npcs[0].script.contains("setimg"));
        assert_eq!(again.level().links.len(), 1);
        assert_eq!(again.level().links[0].dest, "other.nw");
        assert_eq!(again.level().links[0].new_x, Some(62.0));
        assert_eq!(again.level().links[0].new_y, None);
        assert_eq!(again.level().signs[0].text, "hello");
        assert_eq!(again.level().baddies[0].kind, "1");
        assert_eq!(again.level().baddies[0].verses, "verse");
        assert_eq!(again.level().chests[0].item, "greenrupee");
    }

    #[test]
    fn painting_uses_the_shared_drawer_and_shows_the_new_tile() {
        let (rgb, width, height) = coloured_sheet(2);
        let sheet = Sheet {
            rgb: &rgb,
            width,
            height,
        };
        let view = BoardView {
            origin_x: 0,
            origin_y: 0,
            width: 16,
            height: 16,
        };
        let mut doc = LevelDoc::open(b"GLEVNW01\nBOARD 0 0 1 0 AA\n").unwrap();
        let before = doc.paint(sheet, view);
        assert_eq!(before.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(before.pixel(15, 15), Some([255, 0, 0, 255]));
        doc.set_tile(0, 0, 1).unwrap();
        let after = doc.paint(sheet, view);
        assert_eq!(after.pixel(0, 0), Some([0, 255, 0, 255]));
        assert_eq!(after.pixel(15, 15), Some([0, 255, 0, 255]));
    }
}
