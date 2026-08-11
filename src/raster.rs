//! CPU-side pixel drawing onto a macroquad [`Image`] for procedural art:
//! filled primitives, Bresenham lines, and deterministic per-pixel noise.
//!
//! Extracted from finallanding's `ui/art/pixels.rs` (itself modeled on
//! toybox's procedural texture painting). All functions clip against the
//! image bounds, so callers can draw shapes that overlap the edges.

use macroquad::prelude::{Color, Image};

/// Sets a pixel, silently ignoring coordinates outside the image.
pub fn set_pixel_safe(image: &mut Image, x: i32, y: i32, color: Color) {
    if x < 0 || y < 0 || x >= image.width as i32 || y >= image.height as i32 {
        return;
    }
    image.set_pixel(x as u32, y as u32, color);
}

/// Fills an axis-aligned rectangle, clipped to the image.
pub fn fill_rect(image: &mut Image, x: i32, y: i32, width: i32, height: i32, color: Color) {
    for yy in y..(y + height) {
        for xx in x..(x + width) {
            set_pixel_safe(image, xx, yy, color);
        }
    }
}

/// Fills a circle centered at (`cx`, `cy`), clipped to the image.
pub fn fill_circle(image: &mut Image, cx: i32, cy: i32, radius: i32, color: Color) {
    let radius_sq = radius * radius;
    for yy in (cy - radius)..=(cy + radius) {
        for xx in (cx - radius)..=(cx + radius) {
            let dx = xx - cx;
            let dy = yy - cy;
            if dx * dx + dy * dy <= radius_sq {
                set_pixel_safe(image, xx, yy, color);
            }
        }
    }
}

/// Fills an axis-aligned ellipse with radii `rx`/`ry`, clipped to the image.
pub fn fill_ellipse(image: &mut Image, cx: i32, cy: i32, rx: i32, ry: i32, color: Color) {
    let rx_sq = (rx * rx).max(1);
    let ry_sq = (ry * ry).max(1);
    for yy in (cy - ry)..=(cy + ry) {
        for xx in (cx - rx)..=(cx + rx) {
            let dx = xx - cx;
            let dy = yy - cy;
            if dx * dx * ry_sq + dy * dy * rx_sq <= rx_sq * ry_sq {
                set_pixel_safe(image, xx, yy, color);
            }
        }
    }
}

/// Draws a 1px Bresenham line from (`x0`, `y0`) to (`x1`, `y1`), clipped to
/// the image.
pub fn draw_line_pixels(image: &mut Image, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;

    loop {
        set_pixel_safe(image, x, y, color);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * error;
        if e2 >= dy {
            error += dy;
            x += sx;
        }
        if e2 <= dx {
            error += dx;
            y += sy;
        }
    }
}

/// Adds deterministic value noise of strength `amount` (`0..=1` is typical)
/// to every non-transparent pixel. The same `seed` always produces the same
/// grain, so procedural textures stay stable across frames and runs.
pub fn add_noise(image: &mut Image, seed: u32, amount: f32) {
    for y in 0..image.height {
        for x in 0..image.width {
            let current = image.get_pixel(x as u32, y as u32);
            if current.a <= 0.0 {
                continue;
            }

            let value = noise_value(x as u32, y as u32, seed);
            let delta = (value as f32 / 255.0 - 0.5) * amount;
            image.set_pixel(
                x as u32,
                y as u32,
                Color::new(
                    (current.r + delta).clamp(0.0, 1.0),
                    (current.g + delta).clamp(0.0, 1.0),
                    (current.b + delta).clamp(0.0, 1.0),
                    current.a,
                ),
            );
        }
    }
}

fn noise_value(x: u32, y: u32, seed: u32) -> u8 {
    let mixed =
        x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663) ^ seed.wrapping_mul(83_492_791);
    (mixed & 0xFF) as u8
}

#[cfg(test)]
mod tests;
