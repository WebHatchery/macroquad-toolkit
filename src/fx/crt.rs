//! CRT / phosphor-monitor post-processing overlay.
//!
//! Draws several cheap, shader-free layers in *screen space* to make a flat 2D
//! render read like an old cathode-ray terminal: horizontal scanlines, a
//! corner vignette (screen curvature falloff), a slow rolling refresh band, a
//! subtle whole-screen flicker, and rounded tube-glass corners.
//!
//! Call [`CrtOverlay::draw`] as the very last step of a frame, after all
//! world/UI rendering and after `end_virtual_ui_frame`, so it sits on top of
//! everything. Keep the [`CrtStyle`] alphas low — the effect should suggest a
//! monitor, not obscure the picture.
//!
//! The vignette is baked once into a small texture and stretched, so the whole
//! overlay is a handful of draw calls per frame regardless of resolution.

use macroquad::prelude::*;

use crate::colors::with_alpha;
use crate::math::smoothstep;

/// Tunable appearance of the CRT overlay. All `*_alpha` fields are in `[0, 1]`;
/// the defaults are deliberately restrained.
#[derive(Debug, Clone, Copy)]
pub struct CrtStyle {
    /// Vertical spacing between scanlines, in screen pixels (2.0–4.0 reads best).
    pub scanline_gap: f32,
    /// Darkness of each scanline (0 disables scanlines).
    pub scanline_alpha: f32,
    /// Strength of the corner vignette (0 disables it).
    pub vignette_alpha: f32,
    /// Peak brightness of the rolling refresh band (0 disables it).
    pub scan_band_alpha: f32,
    /// How many screen-heights the refresh band travels per second.
    pub scan_band_speed: f32,
    /// Amplitude of the whole-screen flicker (0 disables it).
    pub flicker_alpha: f32,
    /// Phosphor tint used for the refresh band and flicker (alpha ignored).
    pub tint: Color,
    /// Radius, in screen pixels, of the rounded tube-glass corners (0 = square).
    pub corner_radius: f32,
    /// Fill for the masked-off corners — the dark bezel around the glass.
    pub bezel: Color,
}

impl Default for CrtStyle {
    fn default() -> Self {
        Self::amber()
    }
}

impl CrtStyle {
    /// Warm amber-phosphor terminal look (the classic P3 monitor).
    pub fn amber() -> Self {
        Self {
            scanline_gap: 3.0,
            scanline_alpha: 0.14,
            vignette_alpha: 0.55,
            scan_band_alpha: 0.045,
            scan_band_speed: 0.18,
            flicker_alpha: 0.025,
            tint: Color::new(1.0, 0.72, 0.20, 1.0),
            corner_radius: 26.0,
            bezel: Color::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    /// Cool green-phosphor terminal look (the classic P1 monitor).
    pub fn green() -> Self {
        Self {
            tint: Color::new(0.35, 1.0, 0.45, 1.0),
            ..Self::amber()
        }
    }
}

/// Draws the CRT overlay, caching the baked vignette and corner textures
/// between frames.
#[derive(Default)]
pub struct CrtOverlay {
    vignette: Option<Texture2D>,
    corner: Option<Texture2D>,
}

impl CrtOverlay {
    /// Creates an overlay; the vignette texture is baked lazily on first draw.
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws all enabled layers across the whole screen. `time` is a
    /// free-running seconds clock (e.g. `macroquad::time::get_time()`); it only
    /// drives cosmetic animation, never gameplay, so it need not be
    /// deterministic.
    pub fn draw(&mut self, time: f32, style: &CrtStyle) {
        let w = screen_width();
        let h = screen_height();

        if style.vignette_alpha > 0.0 {
            let tex = self.vignette.get_or_insert_with(build_vignette);
            draw_texture_ex(
                tex,
                0.0,
                0.0,
                with_alpha(BLACK, style.vignette_alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(w, h)),
                    ..Default::default()
                },
            );
        }

        if style.scanline_alpha > 0.0 {
            let gap = style.scanline_gap.max(1.0);
            let color = with_alpha(BLACK, style.scanline_alpha);
            let mut y = 0.0;
            while y < h {
                draw_line(0.0, y, w, y, 1.0, color);
                y += gap;
            }
        }

        if style.scan_band_alpha > 0.0 {
            let band_h = (h * 0.14).max(24.0);
            let y = scan_band_y(time, h, style.scan_band_speed, band_h);
            // A few stacked translucent strips fake a soft vertical gradient.
            const STRIPS: usize = 4;
            for i in 0..STRIPS {
                let t = i as f32 / (STRIPS - 1) as f32;
                let a = style.scan_band_alpha * (1.0 - (t - 0.5).abs() * 2.0);
                let strip_h = band_h / STRIPS as f32;
                draw_rectangle(
                    0.0,
                    y + t * band_h,
                    w,
                    strip_h + 1.0,
                    with_alpha(style.tint, a),
                );
            }
        }

        if style.flicker_alpha > 0.0 {
            let a = style.flicker_alpha * flicker_factor(time);
            draw_rectangle(0.0, 0.0, w, h, with_alpha(style.tint, a));
        }

        // Rounded tube-glass corners, drawn last so the bezel clips every layer.
        if style.corner_radius > 0.0 {
            let tex = self.corner.get_or_insert_with(build_corner_mask);
            let r = style.corner_radius.min(w.min(h) * 0.5);
            for (fx, fy, px, py) in [
                (false, false, 0.0, 0.0),
                (true, false, w - r, 0.0),
                (false, true, 0.0, h - r),
                (true, true, w - r, h - r),
            ] {
                draw_texture_ex(
                    tex,
                    px,
                    py,
                    style.bezel,
                    DrawTextureParams {
                        dest_size: Some(vec2(r, r)),
                        flip_x: fx,
                        flip_y: fy,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

/// Irregular flicker in `[0, 1]`, mixing two out-of-phase sines so it never
/// reads as a clean pulse.
fn flicker_factor(time: f32) -> f32 {
    let f = (time * 11.0).sin() * 0.6 + (time * 27.0).sin() * 0.4;
    (0.5 + 0.5 * f).clamp(0.0, 1.0)
}

/// Top-edge Y of the refresh band, scrolling down and wrapping fully off-screen
/// so it re-enters from the top rather than popping.
fn scan_band_y(time: f32, height: f32, speed: f32, band_h: f32) -> f32 {
    let span = height + band_h;
    (time * speed * height).rem_euclid(span) - band_h
}

/// Bakes a top-left corner mask: opaque (alpha 1) outside a quarter-circle of
/// glass, transparent inside, with an antialiased edge. Flipped per corner and
/// multiplied by the bezel color, it rounds the screen like a CRT tube face.
fn build_corner_mask() -> Texture2D {
    const N: u16 = 64;
    let r = N as f32;
    let mut image = Image::gen_image_color(N, N, Color::new(0.0, 0.0, 0.0, 0.0));
    for y in 0..N {
        for x in 0..N {
            // Distance to the inner corner (the glass centre side of this tile).
            let dx = r - x as f32;
            let dy = r - y as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = smoothstep(r - 1.5, r + 0.5, dist);
            image.set_pixel(x as u32, y as u32, Color::new(1.0, 1.0, 1.0, alpha));
        }
    }
    let tex = Texture2D::from_image(&image);
    tex.set_filter(FilterMode::Linear);
    tex
}

/// Bakes a small radial vignette: white with alpha rising from 0 at the centre
/// to 1 in the corners. Drawn multiplied by black, this darkens the edges.
fn build_vignette() -> Texture2D {
    const N: u16 = 128;
    let mut image = Image::gen_image_color(N, N, Color::new(0.0, 0.0, 0.0, 0.0));
    let half = (N as f32 - 1.0) / 2.0;
    for y in 0..N {
        for x in 0..N {
            let dx = (x as f32 - half) / half;
            let dy = (y as f32 - half) / half;
            let dist = (dx * dx + dy * dy).sqrt() / std::f32::consts::SQRT_2;
            let shape = smoothstep(0.45, 1.05, dist);
            image.set_pixel(x as u32, y as u32, Color::new(1.0, 1.0, 1.0, shape));
        }
    }
    let tex = Texture2D::from_image(&image);
    tex.set_filter(FilterMode::Linear);
    tex
}

#[cfg(test)]
mod tests;
