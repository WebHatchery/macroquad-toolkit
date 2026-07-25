//! Drawing primitives with somewhere other than the screen to go.
//!
//! Procedural art — shapes assembled from triangles and circles rather than
//! sampled from a PNG — is cheap to ship and impossible to test. It only exists
//! once there is a window, a GL context and a frame, so the only way to check it
//! is for a person to look at a screenshot.
//!
//! Putting a trait in front of the primitives costs nothing at the call sites
//! and fixes that. [`ScreenPainter`] draws to the screen, exactly as before.
//! [`Buffer`] draws into a plain pixel array with no window and no GPU, which a
//! unit test can then measure: coverage, silhouette, how alike two shapes look
//! in monochrome, and a [`fingerprint`](Buffer::fingerprint) that fails when art
//! changes without anyone meaning it to.
//!
//! ```no_run
//! use macroquad_toolkit::paint::{Buffer, Painter};
//! use macroquad::prelude::*;
//!
//! # fn draw_my_symbol<P: Painter>(_: &mut P, _: Rect) {}
//! let mut buffer = Buffer::new(64, 64);
//! let bounds = buffer.bounds();
//! draw_my_symbol(&mut buffer, bounds);
//! assert!(buffer.coverage() > 0.1, "the art drew almost nothing");
//! ```
//!
//! The two implementations share the drawing routines, which is the only reason
//! a measurement taken from the buffer says anything about what a player sees.

use macroquad::prelude::*;

/// The primitives the symbol art is built from.
///
/// Coordinates are already in pixels — the normalising is [`Canvas`]'s job, so
/// an implementation only has to know how to fill a shape.
pub trait Painter {
    fn tri(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color);
    fn circle(&mut self, center: Vec2, radius: f32, color: Color);
    fn ellipse(&mut self, center: Vec2, rx: f32, ry: f32, color: Color);
    fn rect(&mut self, at: Vec2, size: Vec2, color: Color);
}

/// Draws to the screen. What the game uses.
pub struct ScreenPainter;

impl Painter for ScreenPainter {
    fn tri(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color) {
        draw_triangle(a, b, c, color);
    }

    fn circle(&mut self, center: Vec2, radius: f32, color: Color) {
        draw_circle(center.x, center.y, radius, color);
    }

    fn ellipse(&mut self, center: Vec2, rx: f32, ry: f32, color: Color) {
        draw_ellipse(center.x, center.y, rx, ry, 0.0, color);
    }

    fn rect(&mut self, at: Vec2, size: Vec2, color: Color) {
        draw_rectangle(at.x, at.y, size.x, size.y, color);
    }
}

/// Draws into memory instead of onto the screen.
///
/// The point of it is testing. Procedural art is normally verified by a person
/// looking at a screenshot, which needs a person, needs a GPU, and only ever
/// examines the frames somebody thought to capture. Drawing into a buffer means
/// a unit test can ask whether a shape is actually there, how much of its cell
/// it fills, whether two shapes can be told apart, and whether any of it changed
/// since last time.
///
/// Deliberately simple: no antialiasing, no sub-pixel coverage. A test asking
/// "are these two symbols different shapes" does not need either, and adding
/// them would make the buffer disagree with the GPU in ways that are hard to
/// reason about. What it does need is alpha blending, because half the art is
/// translucent highlights over solid facets.
#[derive(Debug, Clone)]
pub struct Buffer {
    width: usize,
    height: usize,
    /// Straight RGB, one triple per pixel. Starts black.
    pixels: Vec<[f32; 3]>,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![[0.0; 3]; width * height],
        }
    }

    pub fn at(&self, x: usize, y: usize) -> [f32; 3] {
        self.pixels[y * self.width + x]
    }

    /// The rectangle the art should be drawn into to fill this buffer.
    ///
    /// Named `bounds` rather than `rect` because `Painter::rect` is a drawing
    /// call and having both on one type reads as a mistake even when it compiles.
    pub fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width as f32, self.height as f32)
    }

    /// Share of pixels with anything drawn on them.
    ///
    /// A symbol that covers almost nothing is invisible at a glance; one that
    /// covers almost everything is a coloured square rather than a shape.
    pub fn coverage(&self) -> f32 {
        let lit = self
            .pixels
            .iter()
            .filter(|pixel| pixel.iter().any(|channel| *channel > 0.02))
            .count();
        lit as f32 / self.pixels.len().max(1) as f32
    }

    /// Which pixels have anything on them at all.
    ///
    /// The silhouette is the part of a symbol that survives colour blindness,
    /// bad contrast and a very small cell. Two symbols with the same silhouette
    /// are the same symbol to anyone not looking closely.
    pub fn silhouette(&self) -> Vec<bool> {
        self.pixels
            .iter()
            .map(|pixel| pixel.iter().any(|channel| *channel > 0.02))
            .collect()
    }

    /// How unalike two silhouettes are: 0 for identical, 1 for no overlap at
    /// all. The Jaccard distance — the share of the *union* that only one of
    /// them covers.
    ///
    /// Not the share of the whole cell that differs, which was the first
    /// attempt and was useless: both symbols leave most of a cell empty, so two
    /// quite different shapes agreed on 93% of the pixels simply by both being
    /// small. Normalising by the area the symbols actually occupy is what makes
    /// the number mean something.
    pub fn silhouette_difference(&self, other: &Buffer) -> f32 {
        let mine = self.silhouette();
        let theirs = other.silhouette();

        let mut intersection = 0usize;
        let mut union = 0usize;
        for (a, b) in mine.iter().zip(theirs.iter()) {
            if *a && *b {
                intersection += 1;
            }
            if *a || *b {
                union += 1;
            }
        }
        if union == 0 {
            return 0.0;
        }
        1.0 - intersection as f32 / union as f32
    }

    /// How different two symbols look in **monochrome**, 0 to 1.
    ///
    /// Mean absolute difference in luminance over every pixel. This is the test
    /// that matters: it catches a shared outline *and* a shared interior at
    /// once, and monochrome is the strictest realistic case — it is what a
    /// symbol has left after colour blindness, a washed-out screen and a cell a
    /// sixth of the reel window tall.
    ///
    /// Silhouette alone proved too blunt a standard for this art. Every symbol
    /// is a centred object filling most of its cell, so a coin and a chest
    /// overlap heavily in outline and always will; what separates them is the
    /// lid, the keyhole and the shading, none of which an outline can see.
    pub fn monochrome_difference(&self, other: &Buffer) -> f32 {
        let luma = |pixel: [f32; 3]| 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
        let total: f32 = self
            .pixels
            .iter()
            .zip(other.pixels.iter())
            .map(|(a, b)| (luma(*a) - luma(*b)).abs())
            .sum();
        total / self.pixels.len().max(1) as f32
    }

    /// A stable hash of the whole image, for golden-image testing.
    ///
    /// Record it once, assert it afterwards, and any change to the art has to
    /// be a decision rather than a side effect — the same bargain a checksummed
    /// sound set makes. Pixels are quantised to 8 bits first so the value does
    /// not move with floating-point noise between platforms.
    pub fn fingerprint(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for pixel in &self.pixels {
            for channel in pixel {
                hash ^= (channel * 255.0).round() as u64;
                hash = hash.wrapping_mul(0x100_0000_01b3);
            }
        }
        hash
    }

    fn blend(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let index = y as usize * self.width + x as usize;
        let alpha = color.a.clamp(0.0, 1.0);
        let pixel = &mut self.pixels[index];
        pixel[0] += (color.r - pixel[0]) * alpha;
        pixel[1] += (color.g - pixel[1]) * alpha;
        pixel[2] += (color.b - pixel[2]) * alpha;
    }

    /// Fill every pixel whose centre satisfies `inside`, over the given bounds.
    fn fill<F: Fn(f32, f32) -> bool>(
        &mut self,
        bounds: (f32, f32, f32, f32),
        color: Color,
        inside: F,
    ) {
        let (min_x, min_y, max_x, max_y) = bounds;
        let x0 = min_x.floor().max(0.0) as i32;
        let y0 = min_y.floor().max(0.0) as i32;
        let x1 = (max_x.ceil() as i32).min(self.width as i32);
        let y1 = (max_y.ceil() as i32).min(self.height as i32);

        for y in y0..y1 {
            for x in x0..x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                if inside(px, py) {
                    self.blend(x, y, color);
                }
            }
        }
    }
}

impl Painter for Buffer {
    fn tri(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color) {
        // Edge functions. Sign-agnostic so winding order does not matter — the
        // art is not consistent about it and has no reason to be.
        let edge =
            |p: Vec2, q: Vec2, x: f32, y: f32| (q.x - p.x) * (y - p.y) - (q.y - p.y) * (x - p.x);
        let bounds = (
            a.x.min(b.x).min(c.x),
            a.y.min(b.y).min(c.y),
            a.x.max(b.x).max(c.x),
            a.y.max(b.y).max(c.y),
        );
        self.fill(bounds, color, |x, y| {
            let (e0, e1, e2) = (edge(a, b, x, y), edge(b, c, x, y), edge(c, a, x, y));
            (e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0) || (e0 <= 0.0 && e1 <= 0.0 && e2 <= 0.0)
        });
    }

    fn circle(&mut self, center: Vec2, radius: f32, color: Color) {
        self.ellipse(center, radius, radius, color);
    }

    fn ellipse(&mut self, center: Vec2, rx: f32, ry: f32, color: Color) {
        let (rx, ry) = (rx.max(0.001), ry.max(0.001));
        let bounds = (center.x - rx, center.y - ry, center.x + rx, center.y + ry);
        self.fill(bounds, color, |x, y| {
            let dx = (x - center.x) / rx;
            let dy = (y - center.y) / ry;
            dx * dx + dy * dy <= 1.0
        });
    }

    fn rect(&mut self, at: Vec2, size: Vec2, color: Color) {
        let bounds = (at.x, at.y, at.x + size.x, at.y + size.y);
        self.fill(bounds, color, |x, y| {
            x >= at.x && x < at.x + size.x && y >= at.y && y < at.y + size.y
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filled_rectangle_covers_exactly_its_area() {
        let mut buffer = Buffer::new(40, 40);
        buffer.rect(vec2(10.0, 10.0), vec2(20.0, 20.0), WHITE);
        // 400 of 1600 pixels.
        assert!(
            (buffer.coverage() - 0.25).abs() < 0.02,
            "{}",
            buffer.coverage()
        );
    }

    #[test]
    fn a_triangle_fills_about_half_its_bounding_box() {
        let mut buffer = Buffer::new(40, 40);
        buffer.tri(vec2(0.0, 0.0), vec2(40.0, 0.0), vec2(0.0, 40.0), WHITE);
        assert!(
            (buffer.coverage() - 0.5).abs() < 0.05,
            "{}",
            buffer.coverage()
        );
    }

    #[test]
    fn winding_order_does_not_matter() {
        // The art is not consistent about it, and a rasteriser that cared would
        // silently drop half the facets.
        let mut clockwise = Buffer::new(32, 32);
        let mut widdershins = Buffer::new(32, 32);
        clockwise.tri(vec2(2.0, 2.0), vec2(30.0, 2.0), vec2(16.0, 30.0), WHITE);
        widdershins.tri(vec2(16.0, 30.0), vec2(30.0, 2.0), vec2(2.0, 2.0), WHITE);
        assert_eq!(clockwise.silhouette_difference(&widdershins), 0.0);
    }

    #[test]
    fn a_circle_fills_pi_over_four_of_its_box() {
        let mut buffer = Buffer::new(64, 64);
        buffer.circle(vec2(32.0, 32.0), 32.0, WHITE);
        let expected = std::f32::consts::FRAC_PI_4;
        assert!(
            (buffer.coverage() - expected).abs() < 0.02,
            "{} against {}",
            buffer.coverage(),
            expected
        );
    }

    #[test]
    fn alpha_blends_rather_than_replacing() {
        let mut buffer = Buffer::new(8, 8);
        buffer.rect(vec2(0.0, 0.0), vec2(8.0, 8.0), BLACK);
        buffer.rect(
            vec2(0.0, 0.0),
            vec2(8.0, 8.0),
            Color::new(1.0, 1.0, 1.0, 0.5),
        );
        let pixel = buffer.at(4, 4);
        assert!((pixel[0] - 0.5).abs() < 0.01, "{:?}", pixel);
    }

    #[test]
    fn drawing_outside_the_buffer_is_ignored_rather_than_panicking() {
        let mut buffer = Buffer::new(16, 16);
        buffer.rect(vec2(-100.0, -100.0), vec2(50.0, 50.0), WHITE);
        buffer.circle(vec2(500.0, 500.0), 20.0, WHITE);
        assert_eq!(buffer.coverage(), 0.0);
    }
}

#[cfg(test)]
mod fingerprint_tests {
    use super::*;

    fn shape(offset: f32) -> Buffer {
        let mut buffer = Buffer::new(32, 32);
        buffer.circle(vec2(16.0 + offset, 16.0), 10.0, WHITE);
        buffer.tri(
            vec2(4.0, 28.0),
            vec2(28.0, 28.0),
            vec2(16.0 + offset, 6.0),
            Color::new(0.5, 0.2, 0.8, 0.6),
        );
        buffer
    }

    #[test]
    fn the_same_drawing_fingerprints_the_same() {
        assert_eq!(shape(0.0).fingerprint(), shape(0.0).fingerprint());
    }

    #[test]
    fn a_changed_drawing_fingerprints_differently() {
        // The whole point: a shape that moved by one pixel must not pass a
        // golden test that was recorded before it moved.
        assert_ne!(shape(0.0).fingerprint(), shape(1.0).fingerprint());
    }

    #[test]
    fn an_empty_buffer_still_has_a_fingerprint() {
        // Rather than a special case a caller has to remember. Two blank
        // buffers of the same size agree; different sizes do not.
        assert_eq!(
            Buffer::new(8, 8).fingerprint(),
            Buffer::new(8, 8).fingerprint()
        );
        assert_ne!(
            Buffer::new(8, 8).fingerprint(),
            Buffer::new(9, 9).fingerprint()
        );
    }

    #[test]
    fn monochrome_difference_is_zero_for_identical_art_and_grows_with_change() {
        let base = shape(0.0);
        assert_eq!(base.monochrome_difference(&shape(0.0)), 0.0);
        assert!(base.monochrome_difference(&shape(6.0)) > 0.0);
    }

    #[test]
    fn silhouette_difference_ignores_colour() {
        // Two shapes of the same outline in different colours are the same
        // silhouette, which is what makes it the right tool for asking whether
        // shape alone carries a difference.
        let mut pale = Buffer::new(24, 24);
        let mut dark = Buffer::new(24, 24);
        pale.circle(vec2(12.0, 12.0), 8.0, WHITE);
        dark.circle(vec2(12.0, 12.0), 8.0, Color::new(0.3, 0.1, 0.1, 1.0));

        assert_eq!(pale.silhouette_difference(&dark), 0.0);
        assert!(pale.monochrome_difference(&dark) > 0.0);
    }
}
