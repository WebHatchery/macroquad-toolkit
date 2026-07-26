//! A contact sheet of frames across an action, instead of one settled frame.
//!
//! # What a screenshot cannot show
//!
//! [`run_capture`](super::run_capture) steps the simulation a fixed number of
//! frames and photographs the last one. That is the right tool for a panel, a
//! menu, a laid-out screen — anything that has come to rest.
//!
//! It is the wrong tool for motion, and quietly so: every capture is of a
//! settled frame, so a fault that only exists *during* an animation cannot
//! appear in one. Two bugs in the game this was written for were found by a
//! player rather than by the harness, and both were mid-flight — a reel that had
//! landed still drawing the previous spin's symbols until the last reel settled,
//! and a spin fast enough that the art could not be read while it turned.
//! Neither is visible in a photograph of the end.
//!
//! So this takes a shot every `every` frames, scales each down, and tiles them
//! into one image in reading order. The result is a strip that shows what
//! happened between the start and the end, which is the part nobody could see.
//!
//! # Scale is not a detail
//!
//! Tiles are box-filtered rather than point-sampled. Nearest-neighbour halving
//! drops every other pixel, which turns one-pixel borders and thin text into
//! aliasing hash — and the whole point is to be able to look at the result and
//! judge it.

use macroquad::prelude::*;

/// How the frames are laid out.
#[derive(Debug, Clone, Copy)]
pub struct StripConfig {
    /// Take a shot every this many frames.
    pub every: u32,
    /// Tiles per row.
    pub columns: u32,
    /// Each tile is this fraction of the window.
    pub scale: f32,
}

impl Default for StripConfig {
    fn default() -> Self {
        Self {
            every: 4,
            columns: 4,
            scale: 0.25,
        }
    }
}

impl StripConfig {
    /// Read the strip settings from `PREFIX_CAPTURE_STRIP*`.
    ///
    /// Returns `None` unless `PREFIX_CAPTURE_STRIP` is set, so a game keeps the
    /// single-frame behaviour until it explicitly asks for a strip.
    pub fn from_env(prefix: &str) -> Option<Self> {
        super::env_string(&format!("{prefix}_CAPTURE_STRIP"))?;
        Some(Self {
            every: super::env_u32(&format!("{prefix}_CAPTURE_STRIP_EVERY"), 4).max(1),
            columns: super::env_u32(&format!("{prefix}_CAPTURE_STRIP_COLUMNS"), 4).max(1),
            scale: super::env_f32(&format!("{prefix}_CAPTURE_STRIP_SCALE"), 0.25).clamp(0.05, 1.0),
        })
    }
}

/// Step the simulation, photographing every `strip.every` frames, and write the
/// tiles as one image.
///
/// `frame` is the game's update-and-draw, exactly as for
/// [`run_capture`](super::run_capture) — and it runs on *every* frame, not just
/// the photographed ones, which is what a per-frame motion check needs. Put the
/// check in there alongside the draw.
pub async fn run_filmstrip<F>(config: &super::CaptureConfig, strip: &StripConfig, mut frame: F)
where
    F: FnMut(f32),
{
    let mut tiles: Vec<Image> = Vec::new();
    for rendered in 0..config.frames {
        frame(config.timestep);
        // After the draw and before `next_frame`, exactly as `run_capture`
        // does: reading afterwards returns the swapped buffer and every tile
        // comes back solid black.
        if rendered % strip.every == 0 {
            tiles.push(shrink(&get_screen_data(), strip.scale));
        }
        next_frame().await;
    }

    match tile(&tiles, strip.columns) {
        Some(sheet) => {
            sheet.export_png(&config.path);
            println!(
                "captured {} ({} frames of scene {}, every {})",
                config.path,
                tiles.len(),
                config.scene,
                strip.every
            );
        }
        None => println!("no frames captured for {}", config.scene),
    }
}

/// Box-filtered downscale. See the module note on why not nearest-neighbour.
fn shrink(source: &Image, scale: f32) -> Image {
    let width = ((source.width() as f32 * scale) as u32).max(1);
    let height = ((source.height() as f32 * scale) as u32).max(1);
    let mut out = Image::gen_image_color(width as u16, height as u16, BLACK);
    let box_w = (source.width() as f32 / width as f32).max(1.0) as u32;
    let box_h = (source.height() as f32 / height as f32).max(1.0) as u32;

    for y in 0..height {
        for x in 0..width {
            let (mut r, mut g, mut b) = (0.0f32, 0.0f32, 0.0f32);
            let mut count = 0.0f32;
            for dy in 0..box_h {
                for dx in 0..box_w {
                    let sx = x * box_w + dx;
                    let sy = y * box_h + dy;
                    if sx < source.width() as u32 && sy < source.height() as u32 {
                        let pixel = source.get_pixel(sx, sy);
                        r += pixel.r;
                        g += pixel.g;
                        b += pixel.b;
                        count += 1.0;
                    }
                }
            }
            let n = count.max(1.0);
            out.set_pixel(x, y, Color::new(r / n, g / n, b / n, 1.0));
        }
    }
    out
}

/// Lay the tiles out in reading order, with a hairline between them so the
/// frame boundaries are visible rather than merging into one wide picture.
fn tile(tiles: &[Image], columns: u32) -> Option<Image> {
    let first = tiles.first()?;
    let (tw, th) = (first.width() as u32, first.height() as u32);
    let columns = columns.min(tiles.len() as u32).max(1);
    let rows = (tiles.len() as u32).div_ceil(columns);
    const GAP: u32 = 2;

    let width = columns * tw + (columns - 1) * GAP;
    let height = rows * th + (rows - 1) * GAP;
    let mut sheet = Image::gen_image_color(
        width as u16,
        height as u16,
        Color::new(0.35, 0.30, 0.12, 1.0),
    );

    // Rows are laid out bottom-up on purpose.
    //
    // `get_screen_data` hands back the framebuffer in OpenGL's order, origin at
    // the bottom left, and `export_png` flips it on the way out. Each tile is
    // therefore stored upside down and comes out the right way up — but the
    // flip applies to the whole sheet, so tiles written in reading order are
    // written out in reverse reading order.
    //
    // The symptom was a filmstrip of a spin that appeared to run backwards:
    // settled reels in the first row, blurred ones in the last. It took single
    // frames at known counts to see that the game was right and the picture was
    // wrong — which is a fair warning about what a lying visualisation costs.
    for (index, tile) in tiles.iter().enumerate() {
        let column = index as u32 % columns;
        let row = index as u32 / columns;
        let origin_x = column * (tw + GAP);
        let origin_y = (rows - 1 - row) * (th + GAP);
        for y in 0..th.min(tile.height() as u32) {
            for x in 0..tw.min(tile.width() as u32) {
                sheet.set_pixel(origin_x + x, origin_y + y, tile.get_pixel(x, y));
            }
        }
    }
    Some(sheet)
}
