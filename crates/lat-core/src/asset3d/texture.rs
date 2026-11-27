//! Textures the 5.12 draws: decoded from any common picture, made at most
//! [`MAX_SIDE`] pixels a side, alpha cut to 0 or 255, written as 8-bit PNG.

use crate::preview::Rgba;

/// The largest side a real 5.12 drew a texture at: 1024 drew, 2048 and
/// 4096 drew white (Graal_analysis `torque/live-512-shapes-2026-11-25.md`).
pub const MAX_SIDE: u32 = 1024;

/// Alpha at or above this is opaque, below it cut out (the 5.12 cuts
/// alpha 0 and draws any other alpha opaque, T3D-032).
pub const ALPHA_CUT: u8 = 128;

/// A picture's bytes as RGBA, by their content: PNG (8 or 16 bits), JPEG,
/// GIF, BMP, TGA, or the first frame of an MNG.
pub fn decode(bytes: &[u8]) -> Result<Rgba, String> {
    if seihai_core::assets::is_mng(bytes) {
        return crate::render::decode_image(bytes).ok_or_else(|| "a broken MNG".to_string());
    }
    let img = image::load_from_memory(bytes).map_err(|e| format!("picture: {e}"))?;
    let rgba = img.to_rgba8();
    Ok(Rgba {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}

/// The power of two nearest `n` (on a log scale), at most `max`'s.
fn pow2(n: u32, max: u32) -> u32 {
    let top = 1u32 << (31 - max.max(1).leading_zeros());
    let lo = 1u32 << (31 - n.max(1).leading_zeros());
    let near = if lo == n || (n as f32) < lo as f32 * std::f32::consts::SQRT_2 {
        lo
    } else {
        lo * 2
    };
    near.min(top)
}

/// `img` made safe: resized to power-of-two sides (the 5.12 pads a
/// non-power-of-two texture and keeps the texture places, so a 44 x 64
/// picture filled only 44/64 of its card in the lab), neither over `max`,
/// and every alpha 0 or 255. Stretching keeps the texture places right,
/// tiled ones too. Growing uses the nearest pixel, so pixel art stays sharp.
pub fn safe(img: &Rgba, max: u32) -> Rgba {
    let max = max.max(1);
    let (w, h) = (pow2(img.width, max), pow2(img.height, max));
    let mut out = if (w, h) != (img.width, img.height) {
        let filter = if w >= img.width && h >= img.height {
            image::imageops::FilterType::Nearest
        } else {
            image::imageops::FilterType::Triangle
        };
        let src = image::RgbaImage::from_raw(img.width, img.height, img.pixels.clone())
            .expect("an RGBA buffer of its size");
        let r = image::imageops::resize(&src, w, h, filter);
        Rgba {
            width: w,
            height: h,
            pixels: r.into_raw(),
        }
    } else {
        img.clone()
    };
    for px in out.pixels.as_chunks_mut::<4>().0.iter_mut() {
        px[3] = if px[3] >= ALPHA_CUT { 255 } else { 0 };
    }
    out
}

/// Whether every pixel is opaque.
pub fn opaque(img: &Rgba) -> bool {
    img.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255)
}

/// 8-bit PNG bytes: RGB when every pixel is opaque, else RGBA.
pub fn png(img: &Rgba) -> Result<Vec<u8>, String> {
    let rgb = opaque(img);
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width, img.height);
        enc.set_color(if rgb {
            png::ColorType::Rgb
        } else {
            png::ColorType::Rgba
        });
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        if rgb {
            let data: Vec<u8> = img
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect();
            w.write_image_data(&data).map_err(|e| e.to_string())?;
        } else {
            w.write_image_data(&img.pixels).map_err(|e| e.to_string())?;
        }
    }
    Ok(out)
}

/// The average colour of the opaque pixels (mid grey when none).
pub fn average(img: &Rgba) -> [u8; 3] {
    let mut sum = [0u64; 3];
    let mut n = 0u64;
    for p in img.pixels.as_chunks::<4>().0.iter() {
        if p[3] >= ALPHA_CUT {
            for k in 0..3 {
                sum[k] += p[k] as u64;
            }
            n += 1;
        }
    }
    if n == 0 {
        return [128, 128, 128];
    }
    sum.map(|s| (s / n) as u8)
}

/// The pixel at texture place `uv` (`v` up from the bottom; wrapping, as
/// textures repeat), or `None` where it is cut out.
pub fn sample(img: &Rgba, uv: [f32; 2]) -> Option<[u8; 3]> {
    if img.width == 0 || img.height == 0 {
        return None;
    }
    let wrap = |t: f32| {
        let f = t - t.floor();
        if f.is_finite() {
            f
        } else {
            0.0
        }
    };
    let x = ((wrap(uv[0]) * img.width as f32) as u32).min(img.width - 1);
    let y = (((1.0 - wrap(uv[1])) * img.height as f32) as u32).min(img.height - 1);
    let p = img.pixel(x, y)?;
    (p[3] >= ALPHA_CUT).then_some([p[0], p[1], p[2]])
}

/// `rgb` kept to `bits` bits a channel (the middle of each step).
pub fn quantise(rgb: [u8; 3], bits: u8) -> [u8; 3] {
    let bits = bits.clamp(1, 8);
    if bits == 8 {
        return rgb;
    }
    let step = 256u32 >> bits;
    rgb.map(|c| ((c as u32 / step) * step + step / 2).min(255) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u32, h: u32, px: [u8; 4]) -> Rgba {
        Rgba {
            width: w,
            height: h,
            pixels: px.repeat((w * h) as usize),
        }
    }

    #[test]
    fn textures_become_powers_of_two_within_the_limit() {
        let s = safe(&img(2048, 512, [10, 20, 30, 200]), MAX_SIDE);
        assert_eq!((s.width, s.height), (1024, 512));
        let s = safe(&img(3000, 1500, [0, 0, 0, 255]), MAX_SIDE);
        assert_eq!((s.width, s.height), (1024, 1024));
        // Not a power of two: stretched to the nearest one.
        let s = safe(&img(44, 64, [0, 0, 0, 255]), MAX_SIDE);
        assert_eq!((s.width, s.height), (32, 64));
        let s = safe(&img(48, 100, [0, 0, 0, 255]), MAX_SIDE);
        assert_eq!((s.width, s.height), (64, 128));
        assert_eq!(pow2(1, 1024), 1);
        assert_eq!(pow2(5000, 1000), 512);
        assert!(s.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
        let s = safe(&img(8, 8, [0, 0, 0, 50]), MAX_SIDE);
        assert_eq!((s.width, s.height), (8, 8));
        assert!(s.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 0));
    }

    #[test]
    fn png_is_8_bit_and_reads_back() {
        let mut i = img(3, 2, [200, 100, 50, 255]);
        let b = png(&i).unwrap();
        let back = decode(&b).unwrap();
        assert_eq!(back, i);
        i.pixels[3] = 0;
        let back = decode(&png(&i).unwrap()).unwrap();
        assert_eq!(back.pixels[3], 0);
        // 16-bit PNGs (which the 5.12 draws white) are read and written as 8.
        let mut b16 = Vec::new();
        {
            let mut e = png::Encoder::new(&mut b16, 1, 1);
            e.set_color(png::ColorType::Rgb);
            e.set_depth(png::BitDepth::Sixteen);
            let mut w = e.write_header().unwrap();
            w.write_image_data(&[0xff, 0xff, 0x80, 0x00, 0, 0]).unwrap();
        }
        let d = decode(&b16).unwrap();
        assert_eq!(d.pixels, vec![255, 128, 0, 255]);
    }

    #[test]
    fn sampling_reads_v_from_the_bottom_and_wraps() {
        let mut i = img(1, 2, [0, 0, 0, 255]);
        i.pixels[..4].copy_from_slice(&[255, 0, 0, 255]); // top row red
        assert_eq!(sample(&i, [0.5, 0.9]), Some([255, 0, 0]));
        assert_eq!(sample(&i, [0.5, 0.1]), Some([0, 0, 0]));
        assert_eq!(sample(&i, [1.5, 1.9]), Some([255, 0, 0]));
        assert_eq!(quantise([255, 0, 130], 1), [192, 64, 192]);
    }
}
