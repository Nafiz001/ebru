//! Laying a sheet: the tray rendered onto paper.
//!
//! Inks are filled in the order they were dropped, which is also the order
//! they sit on the bath: a newer drop pushes older ink aside rather than
//! covering it, so nothing ever overlaps and painter's order is exact. The
//! paper then gets a texture of fibre and uneven absorption, so a print reads
//! as ink taken up by paper rather than a flat vector fill.

use marbling::{HEIGHT, PIGMENTS, Tray, WIDTH};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};

pub fn render(tray: &Tray, width: u32) -> Pixmap {
    let scale = width as f32 / WIDTH as f32;
    let height = (HEIGHT as f32 * scale).round() as u32;
    let mut pixmap = Pixmap::new(width, height).expect("print size is non-zero");

    let [r, g, b] = PIGMENTS[tray.ground as usize].rgb;
    pixmap.fill(Color::from_rgba8(r, g, b, 255));

    let transform = Transform::from_scale(scale, scale);
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    for ink in &tray.inks {
        let mut path = PathBuilder::new();
        let mut points = ink.outline.iter();
        let Some(first) = points.next() else { continue };
        path.move_to(first.x as f32, first.y as f32);
        for p in points {
            path.line_to(p.x as f32, p.y as f32);
        }
        path.close();
        let Some(path) = path.finish() else { continue };
        let [r, g, b] = PIGMENTS[ink.pigment as usize].rgb;
        paint.set_color_rgba8(r, g, b, 255);
        pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
    }

    paper(&mut pixmap, scale);
    pixmap
}

/// Value noise on an integer lattice, smoothly interpolated; 0 to 1.
fn noise(x: f32, y: f32, seed: u32) -> f32 {
    fn hash(x: i32, y: i32, seed: u32) -> f32 {
        let mut h = (x as u32).wrapping_mul(0x27d4eb2d) ^ (y as u32).wrapping_mul(0x165667b1) ^ seed.wrapping_mul(0x9e3779b9);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2c1b3c6d);
        h ^= h >> 12;
        (h & 0xffff) as f32 / 65535.0
    }
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - xi as f32, y - yi as f32);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let top = hash(xi, yi, seed) + (hash(xi + 1, yi, seed) - hash(xi, yi, seed)) * sx;
    let bottom = hash(xi, yi + 1, seed) + (hash(xi + 1, yi + 1, seed) - hash(xi, yi + 1, seed)) * sx;
    top + (bottom - top) * sy
}

/// Darkens each pixel a little by paper fibre and blotchy absorption.
fn paper(pixmap: &mut Pixmap, scale: f32) {
    let width = pixmap.width() as usize;
    let unit = 1.0 / scale;
    let (pixels, _) = pixmap.data_mut().as_chunks_mut::<4>();
    for (i, px) in pixels.iter_mut().enumerate() {
        let (x, y) = ((i % width) as f32 * unit, (i / width) as f32 * unit);
        // Broad, soft variation where the sheet took up more or less ink.
        let blotch = noise(x / 90.0, y / 90.0, 1) * 0.6 + noise(x / 35.0, y / 35.0, 2) * 0.4;
        // Fibres run mostly one way, as they do in laid paper.
        let fibre = noise(x / 2.2, y / 14.0, 3);
        let tooth = noise(x / 0.9, y / 0.9, 4);
        let shade = 1.0 - 0.07 * blotch - 0.05 * fibre - 0.035 * tooth;
        for c in &mut px[..3] {
            *c = (*c as f32 * shade).round().clamp(0.0, 255.0) as u8;
        }
    }
}

pub fn png(tray: &Tray, width: u32) -> Vec<u8> {
    render(tray, width).encode_png().expect("a pixmap always encodes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use marbling::Op;

    #[test]
    fn a_print_is_the_ground_with_the_ink_on_it() {
        let mut tray = Tray::new(9);
        tray.apply(&Op::Drop { x: 800.0, y: 500.0, radius: 150.0, pigment: 0 });
        let pixmap = render(&tray, 400);
        assert_eq!((pixmap.width(), pixmap.height()), (400, 250));

        let at = |x: u32, y: u32| {
            let p = pixmap.pixel(x, y).expect("in bounds");
            [p.red(), p.green(), p.blue()]
        };
        let near = |actual: [u8; 3], expected: [u8; 3]| {
            actual.iter().zip(expected).all(|(a, e)| (*a as i32 - e as i32).abs() <= 40 && *a <= e)
        };
        // The corner is chalk ground, the middle is lapis, both slightly darkened by the paper.
        assert!(near(at(5, 5), PIGMENTS[9].rgb), "corner {:?}", at(5, 5));
        assert!(near(at(200, 125), PIGMENTS[0].rgb), "middle {:?}", at(200, 125));
    }

    #[test]
    fn a_full_size_print_renders_and_encodes() {
        let mut tray = Tray::new(1);
        for i in 0..20 {
            tray.apply(&Op::Drop { x: 100.0 + i as f64 * 70.0, y: 500.0, radius: 50.0, pigment: (i % 9) as u8 });
        }
        let start = std::time::Instant::now();
        let bytes = png(&tray, 2400);
        eprintln!("2400px print in {:?}, {} bytes", start.elapsed(), bytes.len());
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }
}
