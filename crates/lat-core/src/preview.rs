//! RGBA previews for the level and gani editors.
//!
//! Painting is [`seihai_draw`]: a level's tiles and one gani frame go into
//! a buffer. This module only chooses the rectangle and hands over the
//! `seihai-core` value. It does not draw tiles itself and it does not parse
//! a level or a gani.

use seihai_core::assets::{Gani, Level};
use seihai_core::npcart;
use seihai_draw::ab_glyph::FontVec;
use seihai_draw::{Align, Image, PixelBuffer, SpriteSheets, TextStyle};

/// One tile, in pixels. The same size [`seihai_draw`] blits.
pub const TILE_PX: u32 = seihai_draw::TILE_PX;

/// Row-major RGBA8. `pixels.len()` is `width * height * 4`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    /// `R, G, B, A`, left to right, top to bottom.
    pub pixels: Vec<u8>,
}

impl Rgba {
    /// The pixel at `(x, y)`, or `None` outside the buffer.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels.get(i..i + 4).map(|p| [p[0], p[1], p[2], p[3]])
    }
}

/// An RGB picture in the layout `seihai_core`'s PNG reader returns
/// (3 bytes a pixel). Not a file parser: the caller already has the bytes.
#[derive(Debug, Clone, Copy)]
pub struct Sheet<'a> {
    pub rgb: &'a [u8],
    pub width: u32,
    pub height: u32,
}

/// Which part of a level to paint. `origin_x` and `origin_y` are level
/// pixels: the buffer's top-left is that point.
#[derive(Debug, Clone, Copy)]
pub struct BoardView {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u32,
    pub height: u32,
}

/// Which gani step to paint. `(x, y)` is where the gani's origin sits.
#[derive(Debug, Clone, Copy)]
pub struct FrameView {
    pub frame: usize,
    /// 0 up, 1 left, 2 down, 3 right.
    pub dir: u8,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Pictures a gani's `SPRITE` line can name. A sheet left empty skips
/// those sprites. [`GaniPics::all`] points every standard name at one
/// stand-in, which is enough to see a frame change without the world's
/// own images.
#[derive(Debug, Clone, Copy, Default)]
pub struct GaniPics<'a> {
    pub body: Option<Sheet<'a>>,
    pub head: Option<Sheet<'a>>,
    pub sprites: Option<Sheet<'a>>,
    pub sword: Option<Sheet<'a>>,
    pub shield: Option<Sheet<'a>>,
    /// Index 0 is `ATTR1`.
    pub attrs: [Option<Sheet<'a>>; 5],
}

impl<'a> GaniPics<'a> {
    /// Every standard sheet name uses `sheet`.
    pub fn all(sheet: Sheet<'a>) -> Self {
        Self {
            body: Some(sheet),
            head: Some(sheet),
            sprites: Some(sheet),
            sword: Some(sheet),
            shield: Some(sheet),
            attrs: [Some(sheet); 5],
        }
    }
}

fn image(sheet: Sheet<'_>) -> Image<'_> {
    Image::rgb(sheet.rgb, sheet.width, sheet.height)
}

fn sheets<'a>(pics: &'a GaniPics<'a>) -> SpriteSheets<'a> {
    SpriteSheets {
        body: pics.body.map(image),
        head: pics.head.map(image),
        sprites: pics.sprites.map(image),
        sword: pics.sword.map(image),
        shield: pics.shield.map(image),
        attrs: pics.attrs.map(|s| s.map(image)),
    }
}

/// An image by the name a level NPC gives. `None` skips image NPCs.
pub type NpcImages<'a, 'f> = Option<&'a mut (dyn FnMut(&str) -> Option<Rgba> + 'f)>;

/// A parsed gani by file name (`idle.gani`). `None` skips gani NPCs.
pub type GaniFiles<'a, 'f> = Option<&'a mut (dyn FnMut(&str) -> Option<Gani> + 'f)>;

/// Paint `level`'s tiles with `seihai-draw`. Chests, signs and NPCs are
/// [`paint_scene`].
pub fn paint_level(level: &Level, sheet: Sheet<'_>, view: BoardView) -> Rgba {
    let mut buf = PixelBuffer::new(view.width, view.height);
    seihai_draw::draw_level_tiles(&mut buf, level, image(sheet), view.origin_x, view.origin_y);
    Rgba {
        width: buf.width,
        height: buf.height,
        pixels: buf.rgba,
    }
}

/// The four tiles of a chest record: top-left, top-right, bottom-left,
/// bottom-right. A closed chest uses the first set and an opened chest the
/// second. The picture blits them from the record and leaves magenta out.
const CHEST_CLOSED: [u16; 4] = [1784, 1785, 1800, 1801];
const CHEST_OPEN: [u16; 4] = [829, 830, 845, 846];

/// A sign's mark: 2 tiles wide and 1 tall, the same size the level editor
/// uses. Yellow outline, so the board still shows. A sign at (0, 0) is not
/// drawn; that one is used by index.
const SIGN_MARK: [u8; 4] = [255, 230, 60, 255];

/// [`paint_level`], then chests, NPC images and signs. `load` returns an
/// image by the name the NPC gives; the names it could not load come back,
/// once each. An NPC shown as a gani is left out here (see [`paint_scene`]).
pub fn paint_level_npcs(
    level: &Level,
    sheet: Sheet<'_>,
    view: BoardView,
    mut load: impl FnMut(&str) -> Option<Rgba>,
) -> (Rgba, Vec<String>) {
    paint_scene(
        level,
        sheet,
        view,
        Some(&mut load),
        None,
        GaniPics::default(),
    )
}

/// The board, then chest records, then visible NPCs, then signs.
///
/// Chests are a 2×2 blit. A chest whose x or y is outside 0 to 62 is
/// skipped. Signs are a 2×1 outline with the first line of text, except a
/// sign at (0, 0). `images` draws an NPC's picture. `ganis` draws an NPC
/// whose image is a `.gani`, or whose script text names `showcharacter`,
/// `setcharani` or `setani` (the script is not run; `idle` when
/// `showcharacter` names nothing). Names `images` or `ganis` could not
/// load come back, once each.
pub fn paint_scene<'i, 'il, 'g, 'gl>(
    level: &Level,
    sheet: Sheet<'_>,
    view: BoardView,
    images: NpcImages<'i, 'il>,
    ganis: GaniFiles<'g, 'gl>,
    pics: GaniPics<'_>,
) -> (Rgba, Vec<String>) {
    let mut buf = PixelBuffer::new(view.width, view.height);
    seihai_draw::draw_level_tiles(&mut buf, level, image(sheet), view.origin_x, view.origin_y);
    draw_chests(&mut buf, level, sheet, view);
    let missing = draw_npcs(&mut buf, level, view, images, ganis, pics);
    draw_signs(&mut buf, level, view);
    (
        Rgba {
            width: buf.width,
            height: buf.height,
            pixels: buf.rgba,
        },
        missing,
    )
}

fn draw_chests(buf: &mut PixelBuffer, level: &Level, sheet: Sheet<'_>, view: BoardView) {
    let tile = TILE_PX as i32;
    for chest in &level.chests {
        if !(0..=62).contains(&chest.x) || !(0..=62).contains(&chest.y) {
            continue;
        }
        let tiles = if chest.opened {
            CHEST_OPEN
        } else {
            CHEST_CLOSED
        };
        for (i, id) in tiles.into_iter().enumerate() {
            let (dx, dy) = ((i % 2) as i32, (i / 2) as i32);
            blit_keyed_tile(
                buf,
                sheet,
                id,
                (chest.x + dx) * tile - view.origin_x,
                (chest.y + dy) * tile - view.origin_y,
            );
        }
    }
}

/// One tile of `sheet`, magenta left out, the same way a sprite is copied.
fn blit_keyed_tile(buf: &mut PixelBuffer, sheet: Sheet<'_>, id: u16, dx: i32, dy: i32) {
    let (col, row) = seihai_draw::tile_cell(id);
    let sx = col * TILE_PX;
    let sy = row * TILE_PX;
    if sx + TILE_PX > sheet.width || sy + TILE_PX > sheet.height {
        return;
    }
    seihai_draw::draw_sprite(
        buf,
        image(sheet),
        (sx as i32, sy as i32, TILE_PX as i32, TILE_PX as i32),
        dx,
        dy,
    );
}

fn draw_signs(buf: &mut PixelBuffer, level: &Level, view: BoardView) {
    let font = FontVec::try_from_vec(epaint_default_fonts::UBUNTU_LIGHT.to_vec())
        .expect("the bundled font parses");
    let style = TextStyle {
        size: 8.0,
        color: [20, 20, 20, 255],
        bold: false,
        italic: false,
        underline: false,
        strikeout: false,
        shaded: true,
        align: Align::Left,
        wrap: Some(30),
    };
    let tile = TILE_PX as i32;
    for sign in &level.signs {
        if sign.x == 0 && sign.y == 0 {
            continue;
        }
        let x = sign.x * tile - view.origin_x;
        let y = sign.y * tile - view.origin_y;
        buf.stroke_rect(x, y, tile * 2, tile, SIGN_MARK);
        let line = sign.text.lines().next().unwrap_or("").trim();
        if !line.is_empty() {
            seihai_draw::draw_text(buf, &font, line, x + 1, y + 1, &style);
        }
    }
}

fn draw_npcs<'i, 'il, 'g, 'gl>(
    buf: &mut PixelBuffer,
    level: &Level,
    view: BoardView,
    mut images: NpcImages<'i, 'il>,
    mut ganis: GaniFiles<'g, 'gl>,
    pics: GaniPics<'_>,
) -> Vec<String> {
    let mut image_cache: std::collections::HashMap<String, Option<Rgba>> = Default::default();
    let mut gani_cache: std::collections::HashMap<String, Option<Gani>> = Default::default();
    let mut missing = Vec::new();
    let sheets = sheets(&pics);
    let px = TILE_PX as f32;
    for npc in &level.npcs {
        if !npc.visible {
            continue;
        }
        let x = (npc.x * px).round() as i32 - view.origin_x;
        let y = (npc.y * px).round() as i32 - view.origin_y;
        if let Some(name) = gani_file(npc) {
            let Some(load) = ganis.as_deref_mut() else {
                continue;
            };
            let gani = gani_cache.entry(name.clone()).or_insert_with(|| {
                let gani = load(&name);
                if gani.is_none() {
                    missing.push(name.clone());
                }
                gani
            });
            if let Some(gani) = gani {
                seihai_draw::draw_gani_frame(buf, gani, npc.ani_frame, npc.dir, &sheets, x, y);
            }
            continue;
        }
        // `-` is GLEVNW01's "no image".
        if matches!(npc.image.trim(), "" | "-") {
            continue;
        }
        let Some(load) = images.as_deref_mut() else {
            continue;
        };
        let name = npc.image.trim().to_string();
        let pic = image_cache.entry(name.clone()).or_insert_with(|| {
            let pic = load(&name);
            if pic.is_none() {
                missing.push(name.clone());
            }
            pic
        });
        let Some(pic) = pic else { continue };
        let rect = npc
            .image_part
            .unwrap_or((0, 0, pic.width as i32, pic.height as i32));
        seihai_draw::draw_sprite(
            buf,
            Image::rgba(&pic.pixels, pic.width, pic.height),
            rect,
            x,
            y,
        );
    }
    missing
}

/// The gani file an NPC is shown as, when it has one.
///
/// `ani` wins (a script that already ran). Otherwise the first
/// `setcharani` or `setani` word in the script text, then an image whose
/// name ends in `.gani`, then `idle.gani` when the script says
/// `showcharacter` and names nothing. Words after `//` are ignored. This
/// does not run the script.
fn gani_file(npc: &seihai_core::assets::Npc) -> Option<String> {
    if let Some(ani) = npc.ani.as_deref().and_then(npcart::ani_name) {
        return Some(format!("{}.gani", npcart::ani_file(&ani)));
    }
    let tokens = script_words(&npc.script);
    let mut named = None;
    let mut character = false;
    for (i, word) in tokens.iter().enumerate() {
        if word.eq_ignore_ascii_case("showcharacter") {
            character = true;
        } else if word.eq_ignore_ascii_case("setcharani") || word.eq_ignore_ascii_case("setani") {
            character = true;
            if named.is_none() {
                named = tokens.get(i + 1).and_then(|s| npcart::ani_name(s));
            }
        }
    }
    if let Some(name) = named {
        return Some(format!("{}.gani", npcart::ani_file(&name)));
    }
    let image = npc.image.trim();
    if image.len() > 5 && image.to_ascii_lowercase().ends_with(".gani") {
        return Some(image.to_string());
    }
    character.then(|| "idle.gani".to_string())
}

/// Words of a script, with `//` comments and the punctuation around a
/// command removed. Not a GS1 parse.
fn script_words(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in script.lines() {
        let line = line.split("//").next().unwrap_or("");
        for word in line
            .split(|c: char| c.is_whitespace() || matches!(c, '{' | '}' | ';' | ',' | '(' | ')'))
        {
            if !word.is_empty() {
                out.push(word.to_string());
            }
        }
    }
    out
}

/// Paint one gani frame with `seihai-draw`.
pub fn paint_gani(gani: &Gani, pics: &GaniPics<'_>, view: FrameView) -> Rgba {
    let mut buf = PixelBuffer::new(view.width, view.height);
    let pics = sheets(pics);
    seihai_draw::draw_gani_frame(&mut buf, gani, view.frame, view.dir, &pics, view.x, view.y);
    Rgba {
        width: buf.width,
        height: buf.height,
        pixels: buf.rgba,
    }
}

/// A tile sheet big enough for tile ids `0..count`, each tile one flat
/// colour. Tile 0 is red and tile 1 is green, so a one-tile edit is obvious.
/// Ids past `count` are not on the sheet and [`seihai_draw`] skips them.
pub fn coloured_sheet(count: u16) -> (Vec<u8>, u32, u32) {
    let count = count.max(1);
    let (last_col, last_row) = seihai_draw::tile_cell(count - 1);
    let width = (last_col + 1) * TILE_PX;
    let height = (last_row + 1) * TILE_PX;
    let mut rgb = vec![0u8; width as usize * height as usize * 3];
    for id in 0..count {
        let (col, row) = seihai_draw::tile_cell(id);
        fill_tile(&mut rgb, width, col, row, tile_colour(id));
    }
    (rgb, width, height)
}

/// A square RGB picture whose colour changes with x and y, so two sprite
/// frames taken from different places do not look the same.
pub fn pattern_image(size: u32) -> Vec<u8> {
    let mut rgb = vec![0u8; size as usize * size as usize * 3];
    for y in 0..size {
        for x in 0..size {
            let i = (y as usize * size as usize + x as usize) * 3;
            rgb[i] = (x.wrapping_mul(8)) as u8;
            rgb[i + 1] = (y.wrapping_mul(4)) as u8;
            rgb[i + 2] = 96;
        }
    }
    rgb
}

fn tile_colour(id: u16) -> [u8; 3] {
    match id {
        0 => [255, 0, 0],
        1 => [0, 255, 0],
        2 => [0, 0, 255],
        n => {
            let n = n as u8;
            [n.wrapping_mul(40), n.wrapping_mul(90), 160]
        }
    }
}

fn fill_tile(rgb: &mut [u8], width: u32, col: u32, row: u32, colour: [u8; 3]) {
    for y in 0..TILE_PX {
        for x in 0..TILE_PX {
            let xx = col * TILE_PX + x;
            let yy = row * TILE_PX + y;
            let i = (yy as usize * width as usize + xx as usize) * 3;
            if let Some(slot) = rgb.get_mut(i..i + 3) {
                slot.copy_from_slice(&colour);
            }
        }
    }
}
