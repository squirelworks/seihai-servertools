//! Show an RGBA buffer from `lat-core` in the window.
//!
//! The pixels are already painted. This only wraps them in an egui image.

use eframe::egui;
use lat_core::preview::Rgba;

/// The buffer as an egui image. The bytes are copied in order, not
/// premultiplied: `seihai-draw` writes straight alpha.
pub fn color_image(buf: &Rgba) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([buf.width as usize, buf.height as usize], &buf.pixels)
}

/// Draw `buf` at its own size, capped so a full board still fits the page.
pub fn show_buffer(ui: &mut egui::Ui, name: &str, buf: &Rgba) {
    if buf.width == 0 || buf.height == 0 {
        ui.label("Nothing to draw.");
        return;
    }
    let image = color_image(buf);
    let tex = ui
        .ctx()
        .load_texture(name, image, egui::TextureOptions::NEAREST);
    let w = buf.width as f32;
    let h = buf.height as f32;
    let cap = 512.0;
    let scale = (cap / w).min(cap / h).min(1.0);
    ui.add(egui::Image::new(&tex).fit_to_exact_size(egui::vec2(w * scale, h * scale)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use lat_core::gani::GaniDoc;
    use lat_core::level::LevelDoc;
    use lat_core::preview::{coloured_sheet, BoardView, FrameView, GaniPics, Sheet};

    #[test]
    fn the_window_image_is_the_level_and_gani_buffer() {
        let (rgb, width, height) = coloured_sheet(2);
        let sheet = Sheet {
            rgb: &rgb,
            width,
            height,
        };
        let view = BoardView {
            origin_x: 0,
            origin_y: 0,
            width: 16,
            height: 16,
        };
        let mut level = LevelDoc::open(b"GLEVNW01\nBOARD 0 0 1 0 AA\n").unwrap();
        let red = color_image(&level.paint(sheet, view));
        assert_eq!(
            red.pixels[0],
            egui::Color32::from_rgba_unmultiplied(255, 0, 0, 255)
        );
        level.set_tile(0, 0, 1).unwrap();
        let green = color_image(&level.paint(sheet, view));
        assert_eq!(
            green.pixels[0],
            egui::Color32::from_rgba_unmultiplied(0, 255, 0, 255)
        );

        let body = [255, 0, 0, 0, 255, 0];
        let pics = GaniPics {
            body: Some(Sheet {
                rgb: &body,
                width: 2,
                height: 1,
            }),
            ..GaniPics::default()
        };
        let frame = FrameView {
            frame: 0,
            dir: 2,
            x: 0,
            y: 0,
            width: 8,
            height: 4,
        };
        let gani =
            GaniDoc::open(b"SPRITE 0 BODY 0 0 1 1\nSINGLEDIRECTION\nANI\n0 4 2\nANIEND\n").unwrap();
        let image = color_image(&gani.paint(&pics, frame));
        let i = 2 * 8 + 4;
        assert_eq!(
            image.pixels[i],
            egui::Color32::from_rgba_unmultiplied(255, 0, 0, 255)
        );
    }
}
