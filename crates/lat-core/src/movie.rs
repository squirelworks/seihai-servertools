//! GraalShop's movie mode (`GANI0002`): what each actor is at a timeline
//! frame, and a picture of the frame. The file and the keyframe model are
//! seihai-core's (`assets::gani_movie`); this module only paints.
//!
//! Painted: SPRITE actors, as the sprite-table rectangle their `sprite`
//! key names (also read as `sprite #` or `sprite#`), at `dx`, `dy`, in
//! `layer` order (lower first, then file order), unless `visible` is off.
//! `dx` and `dy` are taken as pixels from the picture's top-left: the unit
//! GraalShop used is not known (no saved `GANI0002` file has been found).
//! Not painted yet: CHAR actors (a whole player through a gani) and TEXT
//! actors (fonts); SOUND actors have nothing to draw. They are listed in
//! [`MovieFrame::skipped`].

use crate::preview::Rgba;
use seihai_core::assets::gani_movie::truthy;
use seihai_core::assets::{ActorKind, Gani, GaniMovie};
use seihai_draw::{Image, PixelBuffer};

/// One painted timeline frame.
#[derive(Debug, Clone)]
pub struct MovieFrame {
    pub image: Rgba,
    /// Sprite sheets `load` did not return, once each.
    pub missing: Vec<String>,
    /// Actors not drawn, with why: `"name (CHAR): not drawn yet"`.
    pub skipped: Vec<String>,
}

/// An actor's keys and values at one frame.
type State = Vec<(String, String)>;

fn key<'a>(state: &'a [(String, String)], names: &[&str]) -> Option<&'a str> {
    names.iter().find_map(|n| {
        state
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(n))
            .map(|(_, v)| v.as_str())
    })
}

fn num(state: &[(String, String)], name: &str) -> f64 {
    key(state, &[name])
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0.0)
}

/// Paint timeline frame `t` on a `width` by `height` picture. `load`
/// returns a sprite sheet by the name the sprite table gives.
pub fn paint_frame(
    gani: &Gani,
    movie: &GaniMovie,
    t: u32,
    (width, height): (u32, u32),
    mut load: impl FnMut(&str) -> Option<Rgba>,
) -> MovieFrame {
    let mut buf = PixelBuffer::new(width, height);
    let mut missing: Vec<String> = Vec::new();
    let mut skipped = Vec::new();
    let mut sheets: Vec<(String, Option<Rgba>)> = Vec::new();
    let mut drawn: Vec<(f64, usize, State)> = Vec::new();
    for (i, actor) in movie.actors.iter().enumerate() {
        let state = actor.state_at(t);
        if state.is_empty() {
            continue;
        }
        if key(&state, &["visible"]).is_some_and(|v| !truthy(v)) {
            continue;
        }
        match actor.kind {
            ActorKind::Sprite => drawn.push((num(&state, "layer"), i, state)),
            ActorKind::Sound => {}
            kind => skipped.push(format!("{} ({}): not drawn yet", actor.name, kind.word())),
        }
    }
    drawn.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    for (_, i, state) in drawn {
        let name = &movie.actors[i].name;
        let Some(id) = key(&state, &["sprite", "sprite #", "sprite#"])
            .and_then(|v| v.trim().parse::<u32>().ok())
        else {
            skipped.push(format!("{name} (SPRITE): no sprite number"));
            continue;
        };
        let Some(sprite) = gani.sprites.iter().find(|s| s.id == id) else {
            skipped.push(format!("{name} (SPRITE): sprite {id} is not in the table"));
            continue;
        };
        let sheet = match sheets.iter().position(|(n, _)| *n == sprite.sheet) {
            Some(p) => p,
            None => {
                let pic = load(&sprite.sheet);
                if pic.is_none() {
                    missing.push(sprite.sheet.clone());
                }
                sheets.push((sprite.sheet.clone(), pic));
                sheets.len() - 1
            }
        };
        let Some(pic) = &sheets[sheet].1 else {
            continue;
        };
        seihai_draw::draw_sprite(
            &mut buf,
            Image::rgba(&pic.pixels, pic.width, pic.height),
            (sprite.x, sprite.y, sprite.w, sprite.h),
            num(&state, "dx").round() as i32,
            num(&state, "dy").round() as i32,
        );
    }
    MovieFrame {
        image: Rgba {
            width: buf.width,
            height: buf.height,
            pixels: buf.rgba,
        },
        missing,
        skipped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "GANI0002
SPRITE 0 rocks.png 0 0 2 2 rock
SPRITE 1 rocks.png 2 0 2 2 gem
FRAMES 10
MOVIE
  ACTOR SPRITE a
    FRAME 0 sprite=0,dx=0,dy=0,layer=1
    FRAME 4 dx=8
  ACTOREND

  ACTOR SPRITE b
    FRAME 0 sprite=1,dx=1,dy=0,layer=0
  ACTOREND

  ACTOR CHAR hero
    FRAME 0 dx=3
  ACTOREND

  ACTOR SPRITE ghost
    FRAME 0 sprite=0,dx=5,visible=0
  ACTOREND
MOVIEEND
";

    /// A 4 x 2 sheet: red on the left half, blue on the right.
    fn rocks(name: &str) -> Option<Rgba> {
        (name == "rocks.png").then(|| {
            let mut pixels = Vec::new();
            for _ in 0..2 {
                for x in 0..4 {
                    pixels.extend(if x < 2 {
                        [255, 0, 0, 255]
                    } else {
                        [0, 0, 255, 255]
                    });
                }
            }
            Rgba {
                width: 4,
                height: 2,
                pixels,
            }
        })
    }

    #[test]
    fn paints_sprite_actors_in_layer_order_and_moves_them() {
        let gani = Gani::parse(FILE).unwrap();
        let movie = GaniMovie::parse(FILE).unwrap().unwrap();
        let f = paint_frame(&gani, &movie, 0, (12, 4), rocks);
        // b (blue, layer 0) at x 1..3, a (red, layer 1) over it at 0..2.
        assert_eq!(f.image.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(f.image.pixel(1, 0), Some([255, 0, 0, 255]));
        assert_eq!(f.image.pixel(2, 0), Some([0, 0, 255, 255]));
        assert_eq!(f.image.pixel(5, 0).map(|p| p[3]), Some(0), "ghost hidden");
        assert_eq!(f.skipped, ["hero (CHAR): not drawn yet"]);
        assert!(f.missing.is_empty());
        // Half way to frame 4, a has moved 4 pixels.
        let f = paint_frame(&gani, &movie, 2, (12, 4), rocks);
        assert_eq!(f.image.pixel(4, 0), Some([255, 0, 0, 255]));
        let f = paint_frame(&gani, &movie, 0, (12, 4), |_| None);
        assert_eq!(f.missing, ["rocks.png"]);
    }

    #[test]
    fn a_movie_gani_saves_with_its_movie() {
        let doc = crate::gani::GaniDoc::open(FILE.as_bytes()).unwrap();
        let text = String::from_utf8(doc.encode()).unwrap();
        assert!(text.starts_with("GANI0002\nSPRITE 0 rocks.png 0 0 2 2 rock\n"));
        assert!(text.contains("FRAMES 10\nMOVIE\n  ACTOR SPRITE a\n"));
        assert!(!text.contains("ANI\n"));
        let again = crate::gani::GaniDoc::open(text.as_bytes()).unwrap();
        assert_eq!(again.movie(), doc.movie());
    }
}
