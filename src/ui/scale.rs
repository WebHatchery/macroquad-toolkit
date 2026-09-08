//! User-selected scaling for responsive UI layouts, independent of text-only scaling.
use std::cell::Cell;

use super::VirtualUi;
use macroquad::prelude::{screen_height, screen_width};

pub const MIN_UI_SCALE: f32 = 0.75;
pub const MAX_UI_SCALE: f32 = 2.0;

thread_local! {
    static UI_SCALE: Cell<f32> = const { Cell::new(1.0) };
}

/// Reject non-finite preferences and constrain the supported zoom range.
pub fn sanitize_ui_scale(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE)
    } else {
        1.0
    }
}

pub fn set_ui_scale(scale: f32) {
    UI_SCALE.with(|value| value.set(sanitize_ui_scale(scale)));
}

pub fn ui_scale() -> f32 {
    UI_SCALE.with(Cell::get)
}

impl VirtualUi {
    /// Full-window UI coordinates. A scale of 0.75 exposes more layout space,
    /// while 2.0 exposes less. Hosts reflow their panels against these bounds;
    /// world rendering uses its own camera and is unaffected.
    pub fn responsive() -> Self {
        Self::from_responsive_screen_size(screen_width(), screen_height(), ui_scale())
    }

    pub fn from_responsive_screen_size(width: f32, height: f32, scale: f32) -> Self {
        let scale = sanitize_ui_scale(scale);
        Self {
            logical_width: width / scale,
            logical_height: height / scale,
            scale,
            offset: macroquad::prelude::Vec2::ZERO,
        }
    }

    /// Responsive viewport using the current user preference. Lay out content
    /// against its logical dimensions, draw with `begin`, and map input with
    /// `screen_to_ui`. Minimum dimensions prevent clipping on small screens;
    /// when necessary the requested scale is reduced to fit them.
    pub fn scaled(min_width: f32, min_height: f32) -> Self {
        Self::from_scaled_screen_size(
            screen_width(),
            screen_height(),
            ui_scale(),
            min_width,
            min_height,
        )
    }

    /// Injected form for responsive layout and input tests without a window.
    pub fn from_scaled_screen_size(
        width: f32,
        height: f32,
        scale: f32,
        min_width: f32,
        min_height: f32,
    ) -> Self {
        let scale = sanitize_ui_scale(scale);
        Self::from_screen_size(
            (width / scale).max(min_width),
            (height / scale).max(min_height),
            width,
            height,
        )
    }
}

#[cfg(test)]
mod tests;
