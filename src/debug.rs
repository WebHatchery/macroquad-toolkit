//! Toggleable FPS / frame-time debug overlay.
//!
//! Extracted from toybox's `DebugOverlay` (smoothed frame time, F3 toggle,
//! caller-supplied stat lines) and finallanding's threshold-colored FPS
//! panel; nanite_swarm and scrapyard carry `show_fps` settings this serves.

use macroquad::prelude::*;

use crate::colors::dark;
use crate::ui::draw_ui_text;

/// A small overlay panel showing smoothed FPS/frame-time plus optional
/// caller-supplied stat lines.
///
/// Call [`record_frame`](Self::record_frame) every frame (even while
/// hidden, so the average is warm when opened) and
/// [`draw`](Self::draw) after the rest of the frame.
#[derive(Debug, Clone)]
pub struct DebugOverlay {
    pub visible: bool,
    smoothed_frame_seconds: f32,
    /// Exponential moving average factor per frame (higher reacts faster).
    pub smoothing: f32,
}

impl Default for DebugOverlay {
    fn default() -> Self {
        Self {
            visible: false,
            smoothed_frame_seconds: 1.0 / 60.0,
            smoothing: 0.08,
        }
    }
}

impl DebugOverlay {
    /// Creates a hidden overlay.
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows/hides the overlay (wire to a debug key such as F3).
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Feeds one frame's delta time into the smoothed average.
    pub fn record_frame(&mut self, dt: f32) {
        if dt > 0.0 {
            self.smoothed_frame_seconds += (dt - self.smoothed_frame_seconds) * self.smoothing;
        }
    }

    /// Smoothed frames per second.
    pub fn fps(&self) -> f32 {
        if self.smoothed_frame_seconds <= 0.0 {
            0.0
        } else {
            1.0 / self.smoothed_frame_seconds
        }
    }

    /// Smoothed frame time in milliseconds.
    pub fn frame_ms(&self) -> f32 {
        self.smoothed_frame_seconds * 1000.0
    }

    /// Green at 55+ FPS, yellow at 30+, red below.
    pub fn fps_color(fps: f32) -> Color {
        if fps >= 55.0 {
            dark::POSITIVE
        } else if fps >= 30.0 {
            dark::WARNING
        } else {
            dark::NEGATIVE
        }
    }

    /// Draws the overlay in the top-left corner with optional extra stat
    /// lines below the FPS row. No-op while hidden.
    pub fn draw(&self, extra_lines: &[String]) {
        if !self.visible {
            return;
        }
        let line_height = 18.0;
        let padding = 8.0;
        let height = padding * 2.0 + line_height * (1 + extra_lines.len()) as f32;
        let width = 190.0;

        draw_rectangle(8.0, 8.0, width, height, Color::new(0.05, 0.05, 0.07, 0.85));
        draw_rectangle_lines(8.0, 8.0, width, height, 1.0, dark::TEXT_DIM);

        let fps = self.fps();
        draw_ui_text(
            &format!("{:.0} FPS  {:.1} ms", fps, self.frame_ms()),
            8.0 + padding,
            8.0 + padding + 12.0,
            16.0,
            Self::fps_color(fps),
        );
        for (index, line) in extra_lines.iter().enumerate() {
            draw_ui_text(
                line,
                8.0 + padding,
                8.0 + padding + 12.0 + line_height * (index + 1) as f32,
                16.0,
                dark::TEXT,
            );
        }
    }
}

#[cfg(test)]
mod tests;
