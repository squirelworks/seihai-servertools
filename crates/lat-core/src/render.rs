//! Levels and whole gmaps as PNG pictures: the job of NW2PNG, graalmapview
//! and the classic level viewers.
//!
//! Tiles are painted by `seihai-draw` (through [`crate::preview`]), the
//! same painter as the editors, on every board layer. NPC images, signs and
//! chests are not drawn: the picture is the board. A gmap is painted one
//! level at a time and each level is shrunk by `scale` before it is put in
//! place, so a large map does not need the full-size picture in memory.

use seihai_core::assets::read_png_rgb;
use seihai_core::assets::Level;
use seihai_core::gmap::Gmap;

use crate::preview::{paint_level, BoardView, Rgba, Sheet, TILE_PX};

/// A level is 64 tiles a side.
pub const LEVEL_TILES: u32 = 64;
/// One level's side in pixels at scale 1.
pub const LEVEL_PX: u32 = LEVEL_TILES * TILE_PX;
/// The largest picture side [`gmap_png`] will make, in pixels.
pub const MAX_SIDE_PX: u64 = 32_768;

/// A tile sheet read from a PNG file's bytes.
#[derive(Debug, Clone)]
pub struct OwnedSheet {
    pub rgb: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl OwnedSheet {
    /// Read `pics1.png`-style bytes.
    pub fn from_png(bytes: &[u8]) -> Result<Self, String> {
        let (width, height, rgb) = read_png_rgb(bytes).map_err(|e| format!("tile sheet: {e}"))?;
        Ok(Self { rgb, width, height })
    }

    /// The `pics1.png` built into `seihai-core` (the original default art).
    pub fn builtin() -> Result<Self, String> {
        let bytes = seihai_core::assets::builtin::bytes("pics1.png")
            .ok_or("seihai-core has no built-in pics1.png")?;
        Self::from_png(bytes)
    }

    pub fn sheet(&self) -> Sheet<'_> {
        Sheet {
            rgb: &self.rgb,
            width: self.width,
            height: self.height,
        }
    }
}

/// Check a scale: 1, 2, 4, 8 or 16 (pixels per output pixel, each way).
pub fn check_scale(scale: u32) -> Result<(), String> {
    if matches!(scale, 1 | 2 | 4 | 8 | 16) {
        Ok(())
    } else {
        Err(format!("scale {scale}: use 1, 2, 4, 8 or 16"))
    }
}

/// The whole level, `LEVEL_PX / scale` a side.
pub fn level_rgba(level: &Level, sheet: Sheet<'_>, scale: u32) -> Result<Rgba, String> {
    check_scale(scale)?;
    let full = paint_level(
        level,
        sheet,
        BoardView {
            origin_x: 0,
            origin_y: 0,
            width: LEVEL_PX,
            height: LEVEL_PX,
        },
    );
    Ok(shrink(&full, scale))
}

/// Average `scale` by `scale` blocks. Alpha is averaged too, so a block
/// with no tiles stays transparent.
pub fn shrink(img: &Rgba, scale: u32) -> Rgba {
    if scale <= 1 {
        return img.clone();
    }
    let (w, h) = (img.width / scale, img.height / scale);
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    let n = scale * scale;
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0u32; 4];
            for dy in 0..scale {
                let row = ((y * scale + dy) * img.width + x * scale) as usize * 4;
                for dx in 0..scale as usize {
                    for (c, s) in sum.iter_mut().enumerate() {
                        *s += u32::from(img.pixels[row + dx * 4 + c]);
                    }
                }
            }
            let at = ((y * w + x) * 4) as usize;
            for c in 0..4 {
                pixels[at + c] = ((sum[c] + n / 2) / n) as u8;
            }
        }
    }
    Rgba {
        width: w,
        height: h,
        pixels,
    }
}

/// What [`gmap_rgba`] could not paint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GmapReport {
    pub painted: usize,
    /// Named cells whose level `load` did not return.
    pub missing: Vec<String>,
}

/// The whole gmap, each level `LEVEL_PX / scale` a side, holes and missing
/// levels transparent. `load` returns a level by the name the gmap gives.
pub fn gmap_rgba(
    gmap: &Gmap,
    sheet: Sheet<'_>,
    scale: u32,
    mut load: impl FnMut(&str) -> Option<Level>,
) -> Result<(Rgba, GmapReport), String> {
    check_scale(scale)?;
    let cell = LEVEL_PX / scale;
    let (w, h) = (
        gmap.width as u64 * u64::from(cell),
        gmap.height as u64 * u64::from(cell),
    );
    if w > MAX_SIDE_PX || h > MAX_SIDE_PX {
        return Err(format!(
            "a {w} by {h} picture is too large; use a larger --scale"
        ));
    }
    let (w, h) = (w as u32, h as u32);
    let mut out = Rgba {
        width: w,
        height: h,
        pixels: vec![0; (w as usize) * (h as usize) * 4],
    };
    let mut report = GmapReport::default();
    for (i, name) in gmap.names.iter().enumerate() {
        if name.is_empty() {
            continue;
        }
        let Some(level) = load(name) else {
            report.missing.push(name.clone());
            continue;
        };
        let tile = level_rgba(&level, sheet, scale)?;
        let (cx, cy) = (
            (i % gmap.width) as u32 * cell,
            (i / gmap.width) as u32 * cell,
        );
        for row in 0..cell {
            let src = (row * cell * 4) as usize;
            let dst = (((cy + row) * w + cx) * 4) as usize;
            let len = (cell * 4) as usize;
            out.pixels[dst..dst + len].copy_from_slice(&tile.pixels[src..src + len]);
        }
        report.painted += 1;
    }
    Ok((out, report))
}

/// PNG bytes (8-bit RGBA) of `img`.
pub fn png_bytes(img: &Rgba) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&img.pixels).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::coloured_sheet;

    fn sheet_and_level() -> ((Vec<u8>, u32, u32), Level) {
        // Tile 1 (green) everywhere, tile 0 (red) in the top-left corner.
        let mut text = String::from("GLEVNW01\n");
        for y in 0..64 {
            text.push_str(&format!("BOARD 0 {y} 64 0 "));
            for x in 0..64 {
                text.push_str(if x == 0 && y == 0 { "AA" } else { "AB" });
            }
            text.push('\n');
        }
        (coloured_sheet(2), Level::parse(&text).unwrap())
    }

    #[test]
    fn a_level_is_painted_full_size_and_shrunk() {
        let ((rgb, sw, sh), level) = sheet_and_level();
        let sheet = Sheet {
            rgb: &rgb,
            width: sw,
            height: sh,
        };
        let full = level_rgba(&level, sheet, 1).unwrap();
        assert_eq!((full.width, full.height), (1024, 1024));
        let red = full.pixel(0, 0).unwrap();
        let green = full.pixel(500, 500).unwrap();
        assert_ne!(red, green);
        let small = level_rgba(&level, sheet, 16).unwrap();
        assert_eq!((small.width, small.height), (64, 64));
        assert_eq!(small.pixel(0, 0), Some(red));
        assert_eq!(small.pixel(1, 0), Some(green));
        assert!(level_rgba(&level, sheet, 3).is_err());
    }

    #[test]
    fn a_gmap_puts_each_level_in_its_cell_and_reports_missing_ones() {
        let ((rgb, sw, sh), level) = sheet_and_level();
        let sheet = Sheet {
            rgb: &rgb,
            width: sw,
            height: sh,
        };
        let gmap = Gmap::parse(
            "GRMAP001\nWIDTH 2\nHEIGHT 2\nLEVELNAMES\n\"a.nw\",\"\",\n\"gone.nw\",\"a.nw\",\nLEVELNAMESEND\n",
        )
        .unwrap();
        let (img, report) =
            gmap_rgba(&gmap, sheet, 16, |n| (n == "a.nw").then(|| level.clone())).unwrap();
        assert_eq!((img.width, img.height), (128, 128));
        assert_eq!(report.painted, 2);
        assert_eq!(report.missing, vec!["gone.nw".to_string()]);
        let red = img.pixel(0, 0).unwrap();
        assert_eq!(img.pixel(64, 64), Some(red), "the second a.nw");
        assert_eq!(img.pixel(64, 0).unwrap()[3], 0, "the hole is transparent");
        assert_eq!(img.pixel(0, 64).unwrap()[3], 0, "the missing level too");
        let png = png_bytes(&img).unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }

    #[test]
    fn the_builtin_sheet_reads() {
        let s = OwnedSheet::builtin().unwrap();
        assert!(s.width >= 512 && s.height >= 512);
        assert_eq!(s.rgb.len(), (s.width * s.height * 3) as usize);
    }
}
