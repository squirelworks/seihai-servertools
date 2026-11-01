//! RGBA previews for the level and gani editors.
//!
//! Painting is [`seihai_draw`]: a level's tiles and one gani frame go into
//! a buffer. This module only chooses the rectangle and hands over the
//! `seihai-core` value. It does not draw tiles itself and it does not parse
//! a level or a gani.

use seihai_core::assets::{Gani, Level};
use seihai_draw::{Image, PixelBuffer, SpriteSheets};

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

/// Paint `level` with `seihai-draw`.
pub fn paint_level(level: &Level, sheet: Sheet<'_>, view: BoardView) -> Rgba {
    let mut buf = PixelBuffer::new(view.width, view.height);
    seihai_draw::draw_level_tiles(&mut buf, level, image(sheet), view.origin_x, view.origin_y);
    Rgba {
        width: buf.width,
        height: buf.height,
        pixels: buf.rgba,
    }
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
