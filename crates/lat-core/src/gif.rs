//! `check --make-gif` (#38): a palette GIF next to each PNG that a pre-2.1
//! client would ask for. Those clients ask for `.gif` names, so a world
//! drawn with PNG art shows them nothing.
//!
//! The PNGs are the ones [`crate::check::png_without_gif`] lists: named by a
//! level NPC or a weapon, with no GIF of the same name anywhere in the
//! world. Each GIF is written beside its PNG with the same stem. A file
//! that already exists is never replaced, and nothing else is written.
//!
//! The picture is kept exactly when it has at most 255 opaque colours
//! (plus one transparent index when any pixel is transparent); otherwise
//! the `gif` crate's NeuQuant quantiser picks 256. Alpha is cut at 128:
//! GIF has one transparent colour and no partial transparency.

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// What [`make_gifs`] did with one PNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Made {
    /// Wrote this GIF. `exact` when the colours were kept as they are.
    Wrote { gif: PathBuf, exact: bool },
    /// Could not read or convert the PNG; nothing written.
    Failed { png: PathBuf, reason: String },
}

/// Write the GIFs for every PNG in `root` that pre-2.1 clients would ask
/// for. A PNG named from several places is converted once.
pub fn make_gifs(root: &Path) -> Vec<Made> {
    let mut done = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for want in crate::check::png_without_gif(root) {
        if !done.insert(want.png.clone()) {
            continue;
        }
        let stem = want
            .png
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| want.stem.clone());
        let gif = want.png.with_file_name(format!("{stem}.gif"));
        if gif.exists() {
            continue;
        }
        let result = fs::read(&want.png)
            .map_err(|e| e.to_string())
            .and_then(|bytes| png_to_gif(&bytes));
        match result {
            Ok((data, exact)) => match write_new(&gif, &data) {
                Ok(()) => out.push(Made::Wrote { gif, exact }),
                Err(reason) => out.push(Made::Failed {
                    png: want.png,
                    reason,
                }),
            },
            Err(reason) => out.push(Made::Failed {
                png: want.png,
                reason,
            }),
        }
    }
    out
}

/// Create `path` with `data`, refusing to replace a file that appeared in
/// the meantime.
fn write_new(path: &Path, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(data).map_err(|e| e.to_string())
}

/// Convert PNG bytes to GIF bytes. The flag says whether the colours were
/// kept exactly (no quantising).
pub fn png_to_gif(png_bytes: &[u8]) -> Result<(Vec<u8>, bool), String> {
    let (width, height, mut rgba) = decode_png(png_bytes)?;
    let (w, h) = (
        u16::try_from(width).map_err(|_| "wider than a GIF can be".to_string())?,
        u16::try_from(height).map_err(|_| "taller than a GIF can be".to_string())?,
    );
    for px in rgba.chunks_exact_mut(4) {
        if px[3] < 128 {
            px.copy_from_slice(&[0, 0, 0, 0]);
        } else {
            px[3] = 255;
        }
    }
    let (frame, exact) = match exact_palette(&rgba) {
        Some((palette, pixels, transparent)) => (
            gif::Frame::from_palette_pixels(w, h, pixels, palette, transparent),
            true,
        ),
        None => (gif::Frame::from_rgba_speed(w, h, &mut rgba, 10), false),
    };
    let mut out = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut out, w, h, &[]).map_err(|e| e.to_string())?;
        enc.write_frame(&frame).map_err(|e| e.to_string())?;
    }
    Ok((out, exact))
}

/// The palette, indexed pixels and transparent index, when the picture
/// fits in 256 entries. Transparent pixels share one index.
#[allow(clippy::type_complexity)]
fn exact_palette(rgba: &[u8]) -> Option<(Vec<u8>, Vec<u8>, Option<u8>)> {
    let mut index: HashMap<[u8; 4], u8> = HashMap::new();
    let mut palette = Vec::new();
    let mut pixels = Vec::with_capacity(rgba.len() / 4);
    let mut transparent = None;
    for px in rgba.chunks_exact(4) {
        let key = [px[0], px[1], px[2], px[3]];
        let i = match index.get(&key) {
            Some(&i) => i,
            None => {
                let i = u8::try_from(index.len()).ok()?;
                index.insert(key, i);
                palette.extend_from_slice(&key[..3]);
                if key[3] == 0 {
                    transparent = Some(i);
                }
                i
            }
        };
        pixels.push(i);
    }
    if palette.is_empty() {
        palette.extend_from_slice(&[0, 0, 0]);
    }
    Some((palette, pixels, transparent))
}

/// Width, height and 8-bit RGBA.
fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "picture too large".to_string())?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("indexed PNG was not expanded".into()),
    };
    Ok((info.width, info.height, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, width, height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(rgba).unwrap();
        }
        out
    }

    fn read_gif(bytes: &[u8]) -> (u16, u16, Vec<u8>) {
        let mut opts = gif::DecodeOptions::new();
        opts.set_color_output(gif::ColorOutput::RGBA);
        let mut d = opts.read_info(Cursor::new(bytes)).unwrap();
        let f = d.read_next_frame().unwrap().unwrap();
        (f.width, f.height, f.buffer.to_vec())
    }

    #[test]
    fn few_colours_are_kept_exactly_with_transparency() {
        let rgba = [
            255, 0, 0, 255, /**/ 0, 255, 0, 255, //
            0, 0, 0, 0, /*   */ 10, 20, 30, 40, // under 128: transparent
        ];
        let (gif, exact) = png_to_gif(&png(2, 2, &rgba)).unwrap();
        assert!(exact);
        assert!(gif.starts_with(b"GIF89a"));
        let (w, h, px) = read_gif(&gif);
        assert_eq!((w, h), (2, 2));
        assert_eq!(&px[..8], &[255, 0, 0, 255, 0, 255, 0, 255]);
        assert_eq!(px[11], 0);
        assert_eq!(px[15], 0);
    }

    #[test]
    fn many_colours_are_quantised() {
        let mut rgba = Vec::new();
        for i in 0..1024u32 {
            rgba.extend_from_slice(&[(i % 256) as u8, (i / 4) as u8, (i * 7 % 256) as u8, 255]);
        }
        let (gif, exact) = png_to_gif(&png(32, 32, &rgba)).unwrap();
        assert!(!exact);
        let (w, h, _) = read_gif(&gif);
        assert_eq!((w, h), (32, 32));
    }

    #[test]
    fn not_a_png_is_an_error() {
        assert!(png_to_gif(b"art").is_err());
    }
}
