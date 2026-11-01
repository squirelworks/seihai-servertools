//! The LAT gani editor: open a gani, change one sprite frame, write it back.
//!
//! Parsed with `seihai-core` (`Gani::parse`). There is no second gani parser
//! here. `seihai-core` keeps the original text but does not write a gani
//! from the parsed frames, so [`encode`] writes the text from the struct.
//! Comments and a line the parser does not store are not kept. Sprite
//! definitions, the frame list, waits, sounds and the header flags are.

use crate::preview::{paint_gani, FrameView, GaniPics, Rgba};

pub use seihai_core::assets::Gani;

/// Parse a gani's text. The bytes need not be UTF-8.
pub fn open(bytes: &[u8]) -> Result<Gani, String> {
    Gani::parse(&String::from_utf8_lossy(bytes))
}

/// A gani open for editing.
#[derive(Debug, Clone)]
pub struct GaniDoc {
    gani: Gani,
    dirty: bool,
}

impl GaniDoc {
    /// Parse `bytes`.
    pub fn open(bytes: &[u8]) -> Result<Self, String> {
        Ok(Self {
            gani: open(bytes)?,
            dirty: false,
        })
    }

    /// The parsed gani.
    pub fn gani(&self) -> &Gani {
        &self.gani
    }

    /// True after a change until the next [`Self::open`].
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// How many animation steps the file has.
    pub fn frame_count(&self) -> usize {
        self.gani.frame_count()
    }

    /// The source rectangle of sprite `id`: `(x, y, width, height)` on its
    /// sheet. That rectangle is the sprite's frame.
    pub fn sprite_frame(&self, id: u32) -> Option<(i32, i32, i32, i32)> {
        self.gani
            .sprites
            .iter()
            .find(|s| s.id == id)
            .map(|s| (s.x, s.y, s.w, s.h))
    }

    /// Change the source rectangle of sprite `id`.
    ///
    /// Every animation step that places this sprite draws the new
    /// rectangle. Width and height must be positive. An unknown id leaves
    /// the gani as it was.
    pub fn set_sprite_frame(
        &mut self,
        id: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Result<(), String> {
        if w <= 0 || h <= 0 {
            return Err("a sprite frame needs a positive width and height".into());
        }
        let Some(sprite) = self.gani.sprites.iter_mut().find(|s| s.id == id) else {
            return Err(format!("no sprite {id}"));
        };
        sprite.x = x;
        sprite.y = y;
        sprite.w = w;
        sprite.h = h;
        self.dirty = true;
        Ok(())
    }

    /// Move one placed sprite inside one animation step.
    ///
    /// `frame` is the step, `row` the direction row (0 up, 1 left, 2 down,
    /// 3 right, or 0 for a single-direction gani), `index` the placement
    /// in that row. `(x, y)` is where that sprite sits, in pixels from the
    /// gani origin.
    pub fn set_placement(
        &mut self,
        frame: usize,
        row: usize,
        index: usize,
        x: i32,
        y: i32,
    ) -> Result<(), String> {
        let Some(step) = self.gani.frames.get_mut(frame) else {
            return Err(format!("no frame {frame}"));
        };
        let Some(row) = step.rows.get_mut(row) else {
            return Err(format!("frame {frame} has no direction row {row}"));
        };
        let Some(place) = row.get_mut(index) else {
            return Err(format!("that row has no sprite {index}"));
        };
        place.x = x;
        place.y = y;
        self.dirty = true;
        Ok(())
    }

    /// Gani text for the animation as it is now.
    pub fn encode(&self) -> Vec<u8> {
        encode(&self.gani)
    }

    /// One frame, painted by `seihai-draw`.
    pub fn paint(&self, pics: &GaniPics<'_>, view: FrameView) -> Rgba {
        paint_gani(&self.gani, pics, view)
    }
}

/// Write `gani` as text that [`open`] can read back.
pub fn encode(gani: &Gani) -> Vec<u8> {
    let mut out = String::new();
    for sprite in &gani.sprites {
        out.push_str(&format!(
            "SPRITE {} {} {} {} {} {}",
            sprite.id, sprite.sheet, sprite.x, sprite.y, sprite.w, sprite.h
        ));
        if !sprite.desc.is_empty() {
            out.push(' ');
            out.push_str(&sprite.desc.replace(['\n', '\r'], " "));
        }
        out.push('\n');
    }
    for (i, attr) in gani.default_attrs.iter().enumerate() {
        if let Some(name) = attr {
            if !name.is_empty() {
                out.push_str(&format!("DEFAULTATTR{} {name}\n", i + 1));
            }
        }
    }
    if let Some(name) = &gani.default_head {
        if !name.is_empty() {
            out.push_str(&format!("DEFAULTHEAD {name}\n"));
        }
    }
    if let Some(name) = &gani.default_body {
        if !name.is_empty() {
            out.push_str(&format!("DEFAULTBODY {name}\n"));
        }
    }
    if gani.single_direction {
        out.push_str("SINGLEDIRECTION\n");
    }
    if let Some(name) = &gani.setback_to {
        if !name.is_empty() {
            out.push_str(&format!("SETBACKTO {name}\n"));
        }
    }
    if gani.looped {
        out.push_str("LOOP\n");
    }
    out.push_str("ANI\n");
    for frame in &gani.frames {
        if frame.rows.is_empty() {
            continue;
        }
        for row in &frame.rows {
            let parts: Vec<String> = row
                .iter()
                .map(|p| format!("{} {} {}", p.sprite_id, p.x, p.y))
                .collect();
            out.push_str(&parts.join(", "));
            out.push('\n');
        }
        if frame.wait != 0 {
            out.push_str(&format!("WAIT {}\n", frame.wait));
        }
        for sound in &frame.sounds {
            out.push_str(&format!(
                "PLAYSOUND {} {} {}\n",
                sound.file,
                float_text(sound.x),
                float_text(sound.y)
            ));
        }
        out.push('\n');
    }
    out.push_str("ANIEND\n");
    out.into_bytes()
}

fn float_text(n: f32) -> String {
    if n.is_finite() && n.fract() == 0.0 {
        format!("{}", n as i32)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::{FrameView, GaniPics, Sheet};

    const GANI: &str = "\
SPRITE 0 BODY 0 0 1 1
SPRITE 1 HEAD 2 3 4 5 hat
DEFAULTHEAD head.png
SINGLEDIRECTION
SETBACKTO idle
LOOP
ANI
0 4 2
WAIT 2
PLAYSOUND hit 1.5 2
1 0 0
ANIEND
";

    #[test]
    fn changing_one_sprite_frame_round_trips_through_the_parser() {
        let mut doc = GaniDoc::open(GANI.as_bytes()).unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(doc.sprite_frame(0), Some((0, 0, 1, 1)));
        assert_eq!(doc.frame_count(), 2);
        doc.set_sprite_frame(0, 1, 0, 1, 1).unwrap();
        assert!(doc.is_dirty());
        assert!(doc.set_sprite_frame(9, 0, 0, 1, 1).is_err());
        assert!(doc.set_sprite_frame(0, 0, 0, 0, 1).is_err());
        assert_eq!(
            doc.sprite_frame(0),
            Some((1, 0, 1, 1)),
            "a refused edit must not replace a good one"
        );

        let again = GaniDoc::open(&doc.encode()).unwrap();
        assert_eq!(again.sprite_frame(0), Some((1, 0, 1, 1)));
        assert_eq!(again.sprite_frame(1), Some((2, 3, 4, 5)));
        assert!(again.gani().single_direction);
        assert!(again.gani().looped);
        assert_eq!(again.gani().setback_to.as_deref(), Some("idle"));
        assert_eq!(again.gani().default_head.as_deref(), Some("head.png"));
        assert_eq!(again.frame_count(), 2);
        let first = &again.gani().frames[0];
        assert_eq!(first.rows.len(), 1);
        assert_eq!(first.rows[0][0].sprite_id, 0);
        assert_eq!((first.rows[0][0].x, first.rows[0][0].y), (4, 2));
        assert_eq!(first.wait, 2);
        assert_eq!(first.sounds.len(), 1);
        assert_eq!(first.sounds[0].file, "hit");
        assert_eq!(first.sounds[0].x, 1.5);
        assert_eq!(first.sounds[0].y, 2.0);
        assert_eq!(again.gani().frames[1].rows[0][0].sprite_id, 1);
    }

    #[test]
    fn moving_one_placement_round_trips() {
        let mut doc = GaniDoc::open(GANI.as_bytes()).unwrap();
        doc.set_placement(0, 0, 0, 8, 9).unwrap();
        let again = GaniDoc::open(&doc.encode()).unwrap();
        let place = &again.gani().frames[0].rows[0][0];
        assert_eq!((place.x, place.y), (8, 9));
        assert!(doc.set_placement(3, 0, 0, 0, 0).is_err());
    }

    #[test]
    fn painting_uses_the_shared_drawer_and_shows_the_new_sprite_frame() {
        let body = [255, 0, 0, 0, 255, 0];
        let pics = GaniPics {
            body: Some(Sheet {
                rgb: &body,
                width: 2,
                height: 1,
            }),
            ..GaniPics::default()
        };
        let view = FrameView {
            frame: 0,
            dir: 2,
            x: 0,
            y: 0,
            width: 8,
            height: 4,
        };
        let mut doc = GaniDoc::open(GANI.as_bytes()).unwrap();
        let before = doc.paint(&pics, view);
        assert_eq!(before.pixel(4, 2), Some([255, 0, 0, 255]));
        doc.set_sprite_frame(0, 1, 0, 1, 1).unwrap();
        let after = doc.paint(&pics, view);
        assert_eq!(
            after.pixel(4, 2),
            Some([0, 255, 0, 255]),
            "the placement is unchanged; the sprite frame now reads the green pixel"
        );
    }
}
