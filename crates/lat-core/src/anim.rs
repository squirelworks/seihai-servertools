//! gif2mng: a GIF as the client's PNG (one frame) or MNG (two or more),
//! the job of Graal's own `gif2mng` (behaviour notes: Graal_analysis
//! `tools/gif2mng.md`). Reading is `seihai_core::assets::read_gif_frames`,
//! writing `write_indexed_png` and `write_mng`; this module only joins them.
//!
//! `body` is gif2mng's `-b`: the original also snaps the palette to the
//! seven body colour slots, whose RGB values are not described yet, so here
//! it only makes palette index 2 the transparent colour and leaves the
//! colours as they are.

use seihai_core::assets::{read_gif_frames, write_indexed_png, write_mng, MngRepeat};

/// What [`gif_to_anim`] wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anim {
    pub bytes: Vec<u8>,
    /// `"png"` for one frame, `"mng"` for more.
    pub extension: &'static str,
    pub frames: usize,
    pub width: u32,
    pub height: u32,
}

/// Convert a GIF file's bytes.
pub fn gif_to_anim(gif: &[u8], body: bool) -> Result<Anim, String> {
    let mut read = read_gif_frames(gif).map_err(|e| format!("not a GIF: {e}"))?;
    if body {
        for f in &mut read.frames {
            f.transparent = Some(2);
        }
    }
    let first = read.frames.first().ok_or("no image in the GIF")?;
    let (width, height, frames) = (first.width, first.height, read.frames.len());
    let (bytes, extension) = if frames == 1 {
        (write_indexed_png(first).map_err(|e| e.to_string())?, "png")
    } else {
        // NETSCAPE2.0 counts the plays after the first; 0 is for ever.
        let repeat = match read.loops {
            Some(0) => MngRepeat::Forever,
            Some(n) => MngRepeat::Times(u32::from(n) + 1),
            None => MngRepeat::Times(1),
        };
        (
            write_mng(&read.frames, repeat).map_err(|e| e.to_string())?,
            "mng",
        )
    };
    Ok(Anim {
        bytes,
        extension,
        frames,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2 x 1 GIF89a with `frames` images of one colour index each, a
    /// four-colour table, and a loop-for-ever block when `frames > 1`.
    fn gif(frames: &[u8]) -> Vec<u8> {
        let mut g = b"GIF89a".to_vec();
        g.extend([2, 0, 1, 0, 0x81, 0, 0]);
        g.extend([0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
        if frames.len() > 1 {
            g.extend([0x21, 0xFF, 11]);
            g.extend(b"NETSCAPE2.0");
            g.extend([3, 1, 0, 0, 0]);
        }
        for &c in frames {
            g.extend([0x21, 0xF9, 4, 0, 10, 0, 0, 0]);
            g.extend([0x2C, 0, 0, 0, 0, 2, 0, 1, 0, 0]);
            // LZW, min code 2, 3-bit codes: clear c clear c end.
            let codes = [4u32, u32::from(c), 4, u32::from(c), 5];
            let (mut acc, mut n, mut bytes) = (0u32, 0u32, Vec::new());
            for code in codes {
                acc |= code << n;
                n += 3;
                while n >= 8 {
                    bytes.push(acc as u8);
                    acc >>= 8;
                    n -= 8;
                }
            }
            if n > 0 {
                bytes.push(acc as u8);
            }
            g.extend([2, bytes.len() as u8]);
            g.extend(bytes);
            g.push(0);
        }
        g.push(0x3B);
        g
    }

    #[test]
    fn one_frame_is_a_png_and_more_are_an_mng() {
        let one = gif_to_anim(&gif(&[1]), false).unwrap();
        assert_eq!((one.extension, one.frames), ("png", 1));
        assert!(one.bytes.starts_with(b"\x89PNG"));

        let three = gif_to_anim(&gif(&[1, 2, 3]), false).unwrap();
        assert_eq!((three.extension, three.frames), ("mng", 3));
        assert_eq!((three.width, three.height), (2, 1));
        let mng = seihai_core::assets::read_mng(&three.bytes, &[]).unwrap();
        assert_eq!(mng.frames.len(), 3);
    }

    #[test]
    fn body_mode_makes_index_2_transparent() {
        let png = gif_to_anim(&gif(&[2]), true).unwrap();
        let (_, _, rgba) = seihai_core::assets::read_png_rgba(&png.bytes).unwrap();
        assert_eq!(rgba[3], 0, "index 2 is see-through with -b");
        let png = gif_to_anim(&gif(&[2]), false).unwrap();
        let (_, _, rgba) = seihai_core::assets::read_png_rgba(&png.bytes).unwrap();
        assert_eq!(rgba[3], 255);
        assert!(gif_to_anim(b"\x89PNG", false).is_err());
    }
}
