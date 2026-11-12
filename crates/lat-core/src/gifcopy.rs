//! GIF copies of PNG art for clients before 2.1, which ask for `.gif`
//! names (#38, `check --make-gif`). Opt-in: nothing here runs unless the
//! owner asks for it, and it only ever adds files.
//!
//! A copy keeps every pixel's colour exactly. A PNG with more than 256
//! colours (255 when some pixels are see-through) is skipped with the
//! reason, not dithered: picking a palette for the owner's art is their
//! call. Pixels with alpha below 128 become the GIF's transparent colour;
//! the rest are drawn opaque. That threshold is this tool's choice, not
//! something read from a client.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The most colours a GIF palette holds.
const MAX_COLOURS: usize = 256;

/// Alpha below this is see-through in the copy.
const ALPHA_CUT: u8 = 128;

/// A PNG file's bytes as a GIF with the same pixels.
pub fn png_to_gif(png_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("not a PNG: {e}"))?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("the PNG is too large")?];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("not a PNG: {e}"))?;
    let (w, h) = (info.width, info.height);
    let width = u16::try_from(w).map_err(|_| format!("{w} pixels wide; a GIF holds 65535"))?;
    let height = u16::try_from(h).map_err(|_| format!("{h} pixels high; a GIF holds 65535"))?;
    let data = &buf[..info.buffer_size()];
    let rgba: Vec<[u8; 4]> = match info.color_type {
        png::ColorType::Rgba => data
            .chunks_exact(4)
            .map(|p| [p[0], p[1], p[2], p[3]])
            .collect(),
        png::ColorType::Rgb => data
            .chunks_exact(3)
            .map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => data
            .chunks_exact(2)
            .map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => data.iter().map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("indexed PNG was not expanded".into()),
    };

    let clear = rgba.iter().any(|p| p[3] < ALPHA_CUT);
    let mut palette: Vec<[u8; 3]> = Vec::new();
    if clear {
        // Index 0 is the transparent colour; its RGB is never drawn.
        palette.push([0, 0, 0]);
    }
    let mut index: HashMap<[u8; 3], u8> = HashMap::new();
    let mut pixels = Vec::with_capacity(rgba.len());
    for p in &rgba {
        if p[3] < ALPHA_CUT {
            pixels.push(0);
            continue;
        }
        let rgb = [p[0], p[1], p[2]];
        let i = match index.get(&rgb) {
            Some(&i) => i,
            None => {
                if palette.len() == MAX_COLOURS {
                    let limit = if clear { MAX_COLOURS - 1 } else { MAX_COLOURS };
                    return Err(format!(
                        "more than {limit} colours; a GIF holds {MAX_COLOURS} (one is the see-through colour): reduce the palette first"
                    ));
                }
                let i = palette.len() as u8;
                palette.push(rgb);
                index.insert(rgb, i);
                i
            }
        };
        pixels.push(i);
    }
    // A GIF palette has a power-of-two size, at least two entries.
    let size = palette.len().max(2).next_power_of_two();
    palette.resize(size, [0, 0, 0]);
    let flat: Vec<u8> = palette.iter().flatten().copied().collect();

    let mut out = Vec::new();
    {
        let mut enc =
            gif::Encoder::new(&mut out, width, height, &flat).map_err(|e| e.to_string())?;
        let frame = gif::Frame {
            width,
            height,
            buffer: std::borrow::Cow::Owned(pixels),
            transparent: clear.then_some(0),
            ..gif::Frame::default()
        };
        enc.write_frame(&frame).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// One copy `make_gifs` tried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GifCopy {
    pub png: PathBuf,
    pub gif: PathBuf,
    /// `Ok` when written; otherwise why it was skipped.
    pub result: Result<(), String>,
}

/// Write a GIF copy next to every PNG that `check --client-reach` reports
/// as having none ([`crate::check::gif_copies_wanted`]). An existing file
/// is never replaced. Each copy goes through a temporary file and a rename.
pub fn make_gifs(root: &Path) -> Vec<GifCopy> {
    crate::check::gif_copies_wanted(root)
        .into_iter()
        .map(|(png, gif)| {
            let result = write_copy(&png, &gif);
            GifCopy { png, gif, result }
        })
        .collect()
}

fn write_copy(png: &Path, gif: &Path) -> Result<(), String> {
    if gif.exists() {
        return Err("a file of that name is already there".into());
    }
    let bytes = std::fs::read(png).map_err(|e| e.to_string())?;
    let out = png_to_gif(&bytes)?;
    let tmp = gif.with_extension("gif.tmp");
    std::fs::write(&tmp, out).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, gif).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, color: png::ColorType, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, w, h);
            e.set_color(color);
            e.set_depth(png::BitDepth::Eight);
            let mut wr = e.write_header().unwrap();
            wr.write_image_data(data).unwrap();
        }
        out
    }

    fn decode(gif_bytes: &[u8]) -> (u16, u16, Vec<[u8; 4]>) {
        let mut opts = gif::DecodeOptions::new();
        opts.set_color_output(gif::ColorOutput::RGBA);
        let mut d = opts.read_info(gif_bytes).unwrap();
        let f = d.read_next_frame().unwrap().unwrap().clone();
        let px = f
            .buffer
            .chunks_exact(4)
            .map(|p| [p[0], p[1], p[2], p[3]])
            .collect();
        (f.width, f.height, px)
    }

    #[test]
    fn keeps_every_pixel_and_transparency() {
        let data = [
            255, 0, 0, 255, /**/ 0, 255, 0, 255, /**/ 9, 9, 9, 0, //
            0, 0, 255, 200, /**/ 255, 0, 0, 255, /**/ 1, 2, 3, 127,
        ];
        let gif = png_to_gif(&png(3, 2, png::ColorType::Rgba, &data)).unwrap();
        let (w, h, px) = decode(&gif);
        assert_eq!((w, h), (3, 2));
        assert_eq!(px[0], [255, 0, 0, 255]);
        assert_eq!(px[1], [0, 255, 0, 255]);
        assert_eq!(px[2][3], 0, "alpha 0 is see-through");
        assert_eq!(px[3], [0, 0, 255, 255], "alpha 200 is drawn opaque");
        assert_eq!(px[4], [255, 0, 0, 255]);
        assert_eq!(px[5][3], 0, "alpha 127 is see-through");
    }

    #[test]
    fn rgb_and_grey_without_transparency() {
        let gif = png_to_gif(&png(2, 1, png::ColorType::Rgb, &[1, 2, 3, 4, 5, 6])).unwrap();
        assert_eq!(decode(&gif).2, [[1, 2, 3, 255], [4, 5, 6, 255]]);
        let gif = png_to_gif(&png(2, 1, png::ColorType::Grayscale, &[7, 200])).unwrap();
        assert_eq!(decode(&gif).2, [[7, 7, 7, 255], [200, 200, 200, 255]]);
    }

    #[test]
    fn too_many_colours_is_refused_not_dithered() {
        let mut data = Vec::new();
        for i in 0..256u32 {
            data.extend_from_slice(&[i as u8, (i >> 8) as u8, 7]);
        }
        // 256 opaque colours fit.
        assert!(png_to_gif(&png(256, 1, png::ColorType::Rgb, &data)).is_ok());
        // One more does not.
        data.extend_from_slice(&[1, 1, 1]);
        data.extend_from_slice(&[0; 3 * 255]);
        let e = png_to_gif(&png(256, 2, png::ColorType::Rgb, &data)).unwrap_err();
        assert!(e.contains("reduce the palette"), "{e}");
        assert!(png_to_gif(b"not a png").unwrap_err().contains("not a PNG"));
    }
}
