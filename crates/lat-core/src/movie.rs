//! GraalShop's movie mode (`GANI0002`): what each actor is at a timeline
//! frame, and a picture of the frame. The file and the keyframe model are
//! seihai-core's (`assets::gani_movie`); this module only paints.
//!
//! Painted: SPRITE actors, as the sprite-table rectangle their `sprite`
//! key names (also read as `sprite #` or `sprite#`), at `dx`, `dy`, in
//! `layer` order (lower first, then file order), unless `visible` is off.
//! `dx` and `dy` are taken as pixels from the picture's top-left: the unit
//! GraalShop used is not known (no saved `GANI0002` file has been found).
//! CHAR actors are painted as a player through their `ani` gani (default
//! `idle`) at the step the timeline frame gives (wrapping), facing `dir`,
//! with `head`, `body`, `sword`, `shield`, `attr1` … `attr5` naming the
//! pictures (classic defaults `head0.png`, `body.png`, `sword1.png`,
//! `shield1.png`), on the same layers as sprites. Colours, `chat`,
//! `horse`, params and `playerlook` are not drawn yet.
//! TEXT actors are painted with seihai-draw's text renderer: `text` in
//! `font` (a font file found by name, else Ubuntu Light), `fontsize`
//! pixels times `zoom`, `color` (hex), and the style flags. GraalShop used
//! Windows fonts by face name and point size; neither is known for a saved
//! file, so the size is taken as pixels. SOUND actors have nothing to draw.
//! Actors that could not be drawn are listed in [`MovieFrame::skipped`].

use crate::preview::Rgba;
use seihai_core::assets::gani_movie::truthy;
use seihai_core::assets::{ActorKind, Gani, GaniMovie};
use seihai_draw::ab_glyph::FontVec;
use seihai_draw::{Align, Image, PixelBuffer, SpriteSheets, TextStyle};

/// One painted timeline frame.
#[derive(Debug, Clone)]
pub struct MovieFrame {
    pub image: Rgba,
    /// Sprite sheets `load` did not return, once each.
    pub missing: Vec<String>,
    /// Actors not drawn, with why: `"name (CHAR): gani walk.gani not found"`.
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

/// Where [`paint_frame`] finds what a movie names.
pub trait MovieFiles {
    /// A picture by file name: sprite sheets, and a CHAR actor's head,
    /// body, sword, shield and attrs.
    fn image(&mut self, name: &str) -> Option<Rgba>;
    /// A gani by file name (a CHAR actor's `ani`, `.gani` added).
    fn gani(&mut self, name: &str) -> Option<Gani>;
    /// A font file's bytes by a TEXT actor's `font` name; `None` uses the
    /// default font (Ubuntu Light).
    fn font(&mut self, _name: &str) -> Option<Vec<u8>> {
        None
    }
}

/// Pictures, ganis and fonts by name, loaded once each.
struct Cache<'l, M: MovieFiles> {
    files: &'l mut M,
    images: Vec<(String, Option<Rgba>)>,
    ganis: Vec<(String, Option<Gani>)>,
    fonts: Vec<(String, FontVec)>,
    missing: Vec<String>,
}

impl<M: MovieFiles> Cache<'_, M> {
    fn image(&mut self, name: &str) -> Option<usize> {
        let at = match self
            .images
            .iter()
            .position(|(n, _)| n.eq_ignore_ascii_case(name))
        {
            Some(p) => p,
            None => {
                let pic = self.files.image(name);
                if pic.is_none() {
                    self.missing.push(name.to_string());
                }
                self.images.push((name.to_string(), pic));
                self.images.len() - 1
            }
        };
        self.images[at].1.is_some().then_some(at)
    }

    fn gani(&mut self, name: &str) -> Option<usize> {
        let at = match self
            .ganis
            .iter()
            .position(|(n, _)| n.eq_ignore_ascii_case(name))
        {
            Some(p) => p,
            None => {
                let g = self.files.gani(name);
                if g.is_none() {
                    self.missing.push(name.to_string());
                }
                self.ganis.push((name.to_string(), g));
                self.ganis.len() - 1
            }
        };
        self.ganis[at].1.is_some().then_some(at)
    }

    /// The font `name` names, or the default when it is empty or not
    /// found (reported once as missing).
    fn font(&mut self, name: &str) -> usize {
        if let Some(p) = self
            .fonts
            .iter()
            .position(|(n, _)| n.eq_ignore_ascii_case(name))
        {
            return p;
        }
        let loaded = if name.is_empty() {
            None
        } else {
            let f = self
                .files
                .font(name)
                .and_then(|b| FontVec::try_from_vec(b).ok());
            if f.is_none() {
                self.missing.push(format!("font {name}"));
            }
            f
        };
        let font = loaded.unwrap_or_else(|| {
            FontVec::try_from_vec(epaint_default_fonts::UBUNTU_LIGHT.to_vec())
                .expect("the bundled font parses")
        });
        self.fonts.push((name.to_string(), font));
        self.fonts.len() - 1
    }

    fn rgba(&self, at: Option<usize>) -> Option<Image<'_>> {
        let pic = self.images[at?].1.as_ref()?;
        Some(Image::rgba(&pic.pixels, pic.width, pic.height))
    }
}

/// A CHAR actor's pictures: its key, or the classic default.
const CHAR_SHEETS: [(&str, &str); 5] = [
    ("head", "head0.png"),
    ("body", "body.png"),
    ("sword", "sword1.png"),
    ("shield", "shield1.png"),
    ("sprites", "sprites.png"),
];

/// Paint timeline frame `t` on a `width` by `height` picture, with what
/// the movie names found through `files`.
pub fn paint_frame(
    gani: &Gani,
    movie: &GaniMovie,
    t: u32,
    (width, height): (u32, u32),
    files: &mut impl MovieFiles,
) -> MovieFrame {
    let mut buf = PixelBuffer::new(width, height);
    let mut skipped = Vec::new();
    let mut cache = Cache {
        files,
        images: Vec::new(),
        ganis: Vec::new(),
        fonts: Vec::new(),
        missing: Vec::new(),
    };
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
            ActorKind::Sound => {}
            _ => drawn.push((num(&state, "layer"), i, state)),
        }
    }
    drawn.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    for (_, i, state) in drawn {
        let actor = &movie.actors[i];
        let at = (
            num(&state, "dx").round() as i32,
            num(&state, "dy").round() as i32,
        );
        let result = match actor.kind {
            ActorKind::Char => paint_char(&mut buf, &mut cache, &state, t, at),
            ActorKind::Text => paint_text(&mut buf, &mut cache, &state, at),
            _ => paint_sprite(&mut buf, &mut cache, gani, &state, at),
        };
        if let Err(why) = result {
            skipped.push(format!("{} ({}): {why}", actor.name, actor.kind.word()));
        }
    }
    MovieFrame {
        image: Rgba {
            width: buf.width,
            height: buf.height,
            pixels: buf.rgba,
        },
        missing: cache.missing,
        skipped,
    }
}

fn paint_sprite(
    buf: &mut PixelBuffer,
    cache: &mut Cache<'_, impl MovieFiles>,
    gani: &Gani,
    state: &[(String, String)],
    (x, y): (i32, i32),
) -> Result<(), String> {
    let id = key(state, &["sprite", "sprite #", "sprite#"])
        .and_then(|v| v.trim().parse::<u32>().ok())
        .ok_or("no sprite number")?;
    let sprite = gani
        .sprites
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("sprite {id} is not in the table"))?;
    let at = cache.image(&sprite.sheet);
    if let Some(image) = cache.rgba(at) {
        seihai_draw::draw_sprite(buf, image, (sprite.x, sprite.y, sprite.w, sprite.h), x, y);
    }
    Ok(())
}

/// A CHAR actor: a player drawn through its `ani` gani (default `idle`),
/// facing `dir` (0 up, 1 left, 2 down (default), 3 right), at the gani's
/// step `t` (wrapping), with the pictures its keys name.
fn paint_char(
    buf: &mut PixelBuffer,
    cache: &mut Cache<'_, impl MovieFiles>,
    state: &[(String, String)],
    t: u32,
    (x, y): (i32, i32),
) -> Result<(), String> {
    let ani = key(state, &["ani"])
        .map(str::trim)
        .filter(|a| !a.is_empty());
    let ani = ani.unwrap_or("idle");
    let file = if ani.to_ascii_lowercase().ends_with(".gani") {
        ani.to_string()
    } else {
        format!("{ani}.gani")
    };
    let g = cache
        .gani(&file)
        .ok_or_else(|| format!("gani {file} not found"))?;
    let dir = key(state, &["dir"])
        .and_then(|v| v.trim().parse::<u8>().ok())
        .unwrap_or(2);
    let mut at = [None; 10];
    for (slot, (k, default)) in CHAR_SHEETS.iter().enumerate() {
        let name = key(state, &[k]).map(str::trim).filter(|v| !v.is_empty());
        at[slot] = cache.image(name.unwrap_or(default));
    }
    for n in 0..5 {
        let k = format!("attr{}", n + 1);
        let k2 = format!("attr {}", n + 1);
        if let Some(name) = key(state, &[&k, &k2])
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            at[5 + n] = cache.image(name);
        }
    }
    let sheets = SpriteSheets {
        head: cache.rgba(at[0]),
        body: cache.rgba(at[1]),
        sword: cache.rgba(at[2]),
        shield: cache.rgba(at[3]),
        sprites: cache.rgba(at[4]),
        attrs: [5, 6, 7, 8, 9].map(|i| cache.rgba(at[i])),
    };
    let g = cache.ganis[g].1.as_ref().expect("loaded");
    let steps = g.frames.len().max(1);
    seihai_draw::draw_gani_frame(buf, g, t as usize % steps, dir, &sheets, x, y);
    Ok(())
}

/// `#rrggbb`, `rrggbb`, `#rrggbbaa` or a whole number `0xRRGGBB` / decimal.
fn colour(value: &str) -> Option<[u8; 4]> {
    let v = value.trim();
    let hex = v
        .strip_prefix('#')
        .or_else(|| v.strip_prefix("0x"))
        .or_else(|| v.strip_prefix("0X"));
    let (n, digits) = match hex {
        Some(h) => (u32::from_str_radix(h, 16).ok()?, h.len()),
        None if v.len() == 6 && v.chars().all(|c| c.is_ascii_hexdigit()) => {
            (u32::from_str_radix(v, 16).ok()?, 6)
        }
        None => (v.parse().ok()?, 6),
    };
    Some(if digits == 8 {
        n.to_be_bytes()
    } else {
        let [_, r, g, b] = n.to_be_bytes();
        [r, g, b, 255]
    })
}

/// A TEXT actor: `text` in `font` (a font file the files give, else Ubuntu
/// Light) at `fontsize` pixels (default 12) times `zoom`, in `color`
/// (default black), with `bold`, `italic`, `underline`, `strikeout`,
/// `shaded`, `centered` / `rightaligned` and `wordwrap` (wrapping at the
/// picture's right edge).
fn paint_text(
    buf: &mut PixelBuffer,
    cache: &mut Cache<'_, impl MovieFiles>,
    state: &[(String, String)],
    (x, y): (i32, i32),
) -> Result<(), String> {
    let text = key(state, &["text"]).unwrap_or("");
    if text.is_empty() {
        return Ok(());
    }
    let flag = |k: &str| key(state, &[k]).is_some_and(truthy);
    let size = key(state, &["fontsize", "font size"])
        .and_then(|v| v.trim().parse::<f32>().ok())
        .filter(|s| *s > 0.0)
        .unwrap_or(12.0);
    let zoom = key(state, &["zoom"])
        .and_then(|v| v.trim().parse::<f32>().ok())
        .filter(|z| *z > 0.0)
        .unwrap_or(1.0);
    let color = match key(state, &["color"]) {
        Some(v) => colour(v).ok_or_else(|| format!("color {v} is not a colour"))?,
        None => [0, 0, 0, 255],
    };
    let align = if flag("centered") {
        Align::Center
    } else if flag("rightaligned") {
        Align::Right
    } else {
        Align::Left
    };
    let style = TextStyle {
        size: (size * zoom).min(512.0),
        color,
        bold: flag("bold"),
        italic: flag("italic"),
        underline: flag("underline"),
        strikeout: flag("strikeout"),
        shaded: flag("shaded"),
        align,
        wrap: flag("wordwrap").then(|| (buf.width as i32 - x).max(1) as u32),
    };
    let font = cache.font(key(state, &["font"]).map(str::trim).unwrap_or(""));
    seihai_draw::draw_text(buf, &cache.fonts[font].1, text, x, y, &style);
    Ok(())
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
    /// Files from two closures.
    struct Files<I, G>(I, G);

    impl<I, G> MovieFiles for Files<I, G>
    where
        I: FnMut(&str) -> Option<Rgba>,
        G: FnMut(&str) -> Option<Gani>,
    {
        fn image(&mut self, name: &str) -> Option<Rgba> {
            (self.0)(name)
        }
        fn gani(&mut self, name: &str) -> Option<Gani> {
            (self.1)(name)
        }
    }

    fn none(_: &str) -> Option<Gani> {
        None
    }

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
        let f = paint_frame(&gani, &movie, 0, (12, 4), &mut Files(rocks, none));
        // b (blue, layer 0) at x 1..3, a (red, layer 1) over it at 0..2.
        assert_eq!(f.image.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(f.image.pixel(1, 0), Some([255, 0, 0, 255]));
        assert_eq!(f.image.pixel(2, 0), Some([0, 0, 255, 255]));
        assert_eq!(f.image.pixel(5, 0).map(|p| p[3]), Some(0), "ghost hidden");
        assert_eq!(f.skipped, ["hero (CHAR): gani idle.gani not found"]);
        assert_eq!(f.missing, ["idle.gani"]);
        // Half way to frame 4, a has moved 4 pixels.
        let f = paint_frame(&gani, &movie, 2, (12, 4), &mut Files(rocks, none));
        assert_eq!(f.image.pixel(4, 0), Some([255, 0, 0, 255]));
        let f = paint_frame(&gani, &movie, 0, (12, 4), &mut Files(|_: &str| None, none));
        assert_eq!(f.missing, ["rocks.png", "idle.gani"]);
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

    #[test]
    fn paints_char_actors_through_their_gani() {
        const MOVIE: &str = "GANI0002
FRAMES 4
MOVIE
  ACTOR CHAR hero
    FRAME 0 ani=walk,dir=3,body=green.png,dx=2,dy=1
  ACTOREND
MOVIEEND
";
        // walk: step 0 draws body cell 0 at (0,0) facing right; step 1 at (1,0).
        let walk = Gani::parse(
            "GANI0001\nSPRITE 0 BODY 0 0 1 1 b\nANI\n0 0 0\n0 0 0\n0 0 0\n0 0 0\n\n0 1 0\n0 1 0\n0 1 0\n0 1 0\nANIEND\n",
        )
        .unwrap();
        let gani = Gani::parse(MOVIE).unwrap();
        let movie = GaniMovie::parse(MOVIE).unwrap().unwrap();
        let green = |name: &str| {
            (name == "green.png").then(|| Rgba {
                width: 1,
                height: 1,
                pixels: vec![0, 255, 0, 255],
            })
        };
        let mut asked = Vec::new();
        let mut ganis = |name: &str| {
            asked.push(name.to_string());
            (name == "walk.gani").then(|| walk.clone())
        };
        let f = paint_frame(&gani, &movie, 0, (6, 3), &mut Files(green, &mut ganis));
        assert!(f.skipped.is_empty(), "{:?}", f.skipped);
        assert_eq!(f.image.pixel(2, 1), Some([0, 255, 0, 255]));
        // The defaults that are not there are reported, once each.
        assert!(f.missing.contains(&"head0.png".to_string()));
        let f = paint_frame(&gani, &movie, 1, (6, 3), &mut Files(green, &mut ganis));
        assert_eq!(f.image.pixel(3, 1), Some([0, 255, 0, 255]), "step 1");
        let f = paint_frame(&gani, &movie, 2, (6, 3), &mut Files(green, &mut ganis));
        assert_eq!(f.image.pixel(2, 1), Some([0, 255, 0, 255]), "wraps");
        assert_eq!(asked, ["walk.gani"; 3]);
    }

    #[test]
    fn paints_text_actors_in_the_default_font() {
        const MOVIE: &str = "GANI0002
FRAMES 2
MOVIE
  ACTOR TEXT title
    FRAME 0 text=Hello,fontsize=20,color=#ff0000,bold=1,dx=2,dy=2
    FRAME 1 visible=0
  ACTOREND

  ACTOR TEXT other
    FRAME 0 text=x,font=Fancy,color=pink
  ACTOREND
MOVIEEND
";
        let gani = Gani::parse(MOVIE).unwrap();
        let movie = GaniMovie::parse(MOVIE).unwrap().unwrap();
        let f = paint_frame(&gani, &movie, 0, (80, 30), &mut Files(|_: &str| None, none));
        let red = (0..80)
            .flat_map(|x| (0..30).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                f.image
                    .pixel(x, y)
                    .is_some_and(|p| p[0] > 150 && p[1] < 80 && p[3] > 150)
            })
            .count();
        assert!(red > 40, "Hello is drawn in red ({red} pixels)");
        assert!(f.image.pixel(0, 0).is_some_and(|p| p[3] == 0));
        assert_eq!(f.skipped, ["other (TEXT): color pink is not a colour"]);
        let f = paint_frame(&gani, &movie, 1, (80, 30), &mut Files(|_: &str| None, none));
        assert!(f.image.pixels.chunks(4).all(|p| p[3] == 0), "hidden");
        assert_eq!(colour("#00ff00"), Some([0, 255, 0, 255]));
        assert_eq!(colour("0000ff"), Some([0, 0, 255, 255]));
        assert_eq!(colour("#11223344"), Some([0x11, 0x22, 0x33, 0x44]));
    }
}
