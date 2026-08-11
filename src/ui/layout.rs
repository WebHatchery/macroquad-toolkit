//! UI layout primitives: `UiRect`, virtual (logical-resolution) UI scaling,
//! grid placement, and scroll handling.

use crate::input::*;
use macroquad::prelude::*;

/// Convenience rectangle wrapper used by several games for UI layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiRect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_rect(rect: Rect) -> Self {
        rect.into()
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> Vec2 {
        vec2(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    pub fn centered_x(width: f32, y: f32, w: f32, h: f32) -> Self {
        Self::new((width - w) * 0.5, y, w, h)
    }

    pub fn centered_on_screen(w: f32, h: f32) -> Self {
        Self::new(
            (screen_width() - w) * 0.5,
            (screen_height() - h) * 0.5,
            w,
            h,
        )
    }

    pub fn inset(&self, amount: f32) -> Self {
        Self::new(
            self.x + amount,
            self.y + amount,
            (self.w - amount * 2.0).max(0.0),
            (self.h - amount * 2.0).max(0.0),
        )
    }

    pub fn contains_point(&self, point: Vec2) -> bool {
        rect_contains(self.x, self.y, self.w, self.h, point.x, point.y)
    }

    pub fn contains_mouse(&self) -> bool {
        let (mx, my) = mouse_position();
        self.contains_point(vec2(mx, my))
    }
}

impl From<Rect> for UiRect {
    fn from(rect: Rect) -> Self {
        Self::new(rect.x, rect.y, rect.w, rect.h)
    }
}

impl From<UiRect> for Rect {
    fn from(rect: UiRect) -> Self {
        rect.rect()
    }
}

/// Convenience methods for macroquad `Rect` values.
pub trait RectExt {
    fn right(&self) -> f32;
    fn bottom(&self) -> f32;
    fn center(&self) -> Vec2;
    fn inset(&self, amount: f32) -> Rect;
    fn contains_point(&self, point: Vec2) -> bool;
    fn contains_mouse(&self) -> bool;
}

impl RectExt for Rect {
    fn right(&self) -> f32 {
        self.x + self.w
    }

    fn bottom(&self) -> f32 {
        self.y + self.h
    }

    fn center(&self) -> Vec2 {
        vec2(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    fn inset(&self, amount: f32) -> Rect {
        Rect::new(
            self.x + amount,
            self.y + amount,
            (self.w - amount * 2.0).max(0.0),
            (self.h - amount * 2.0).max(0.0),
        )
    }

    fn contains_point(&self, point: Vec2) -> bool {
        rect_contains(self.x, self.y, self.w, self.h, point.x, point.y)
    }

    fn contains_mouse(&self) -> bool {
        self.contains_point(mouse_position_vec2())
    }
}

/// Fixed-resolution UI mapper with optional letterboxing.
#[derive(Debug, Clone, Copy)]
pub struct VirtualUi {
    pub logical_width: f32,
    pub logical_height: f32,
    pub scale: f32,
    pub offset: Vec2,
}

impl VirtualUi {
    pub fn new(logical_width: f32, logical_height: f32) -> Self {
        let scale_x = screen_width() / logical_width;
        let scale_y = screen_height() / logical_height;
        let scale = scale_x.min(scale_y);
        let viewport_width = logical_width * scale;
        let viewport_height = logical_height * scale;
        let offset = vec2(
            (screen_width() - viewport_width) * 0.5,
            (screen_height() - viewport_height) * 0.5,
        );

        Self {
            logical_width,
            logical_height,
            scale,
            offset,
        }
    }

    /// The letterbox rect in **physical framebuffer pixels**, ready for
    /// `Camera2D::viewport` (which macroquad forwards straight to `glViewport`).
    ///
    /// `screen_width()`/`screen_height()` and `mouse_position()` are *logical*
    /// (macroquad divides them by the DPI scale), so `offset`/`scale` above are
    /// logical too. `glViewport` however takes physical pixels. On a display
    /// with `devicePixelRatio == 1` the two are identical, but on a scaled
    /// display (125%/150%/160% — most laptops and tablets) a logical viewport
    /// covers only `1/dpi` of the framebuffer, so the UI renders shrunk into the
    /// bottom-left corner while hit-testing still spans the whole window. Scale
    /// back up to physical here, and only here, so all the mouse math stays
    /// logical.
    pub fn viewport(&self) -> (i32, i32, i32, i32) {
        self.viewport_for_dpi(screen_dpi_scale())
    }

    /// [`Self::viewport`] with the DPI scale injected, so the conversion can be
    /// tested without a live macroquad context.
    pub fn viewport_for_dpi(&self, dpi: f32) -> (i32, i32, i32, i32) {
        (
            (self.offset.x * dpi).round() as i32,
            (self.offset.y * dpi).round() as i32,
            (self.logical_width * self.scale * dpi).round() as i32,
            (self.logical_height * self.scale * dpi).round() as i32,
        )
    }

    pub fn camera(&self) -> macroquad::camera::Camera2D {
        macroquad::camera::Camera2D {
            target: vec2(self.logical_width * 0.5, self.logical_height * 0.5),
            // Camera2D already applies the screen-space Y inversion for non-render-target draws.
            // A positive Y zoom keeps virtual UI coordinates top-left anchored like Macroquad's default.
            zoom: vec2(2.0 / self.logical_width, 2.0 / self.logical_height),
            viewport: Some(self.viewport()),
            ..Default::default()
        }
    }

    pub fn begin(&self) {
        set_camera(&self.camera());
    }

    pub fn screen_to_ui(&self, point: Vec2) -> Vec2 {
        (point - self.offset) / self.scale
    }

    pub fn ui_to_screen(&self, point: Vec2) -> Vec2 {
        self.offset + point * self.scale
    }

    pub fn mouse_position(&self) -> Vec2 {
        let (mx, my) = mouse_position();
        self.screen_to_ui(vec2(mx, my))
    }

    pub fn is_mouse_inside(&self) -> bool {
        let pos = self.mouse_position();
        pos.x >= 0.0 && pos.y >= 0.0 && pos.x <= self.logical_width && pos.y <= self.logical_height
    }
}

/// Begin drawing in a fixed logical UI resolution.
///
/// Call `end_virtual_ui_frame()` after drawing.
pub fn begin_virtual_ui_frame(logical_width: f32, logical_height: f32) -> VirtualUi {
    let ui = VirtualUi::new(logical_width, logical_height);
    ui.begin();
    ui
}

/// Restore the default macroquad camera after `begin_virtual_ui_frame`.
pub fn end_virtual_ui_frame() {
    set_default_camera();
}

/// Convert current mouse position into a fixed logical UI resolution.
pub fn virtual_mouse_position(logical_width: f32, logical_height: f32) -> Vec2 {
    VirtualUi::new(logical_width, logical_height).mouse_position()
}

/// Convert a rect in logical screen coordinates (the space `screen_width()` and
/// `mouse_position()` report) into the physical framebuffer pixels that the
/// `viewport` field of `Camera2D`/`Camera3D` expects.
///
/// Use this for any hand-built `viewport: Some(..)`; passing logical pixels
/// straight through renders shrunk into the bottom-left corner on displays with
/// a `devicePixelRatio` above 1. See [`VirtualUi::viewport`].
pub fn logical_viewport(x: f32, y: f32, w: f32, h: f32) -> (i32, i32, i32, i32) {
    let dpi = screen_dpi_scale();
    (
        (x * dpi).round() as i32,
        (y * dpi).round() as i32,
        (w * dpi).round() as i32,
        (h * dpi).round() as i32,
    )
}

/// Helper for grid layouts
pub struct GridLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub padding: f32,
    pub cols: usize,
    pub card_height: f32,
}

impl GridLayout {
    pub fn new(x: f32, y: f32, width: f32, padding: f32, cols: usize, card_height: f32) -> Self {
        Self {
            x,
            y,
            width,
            padding,
            cols,
            card_height,
        }
    }

    /// Multiply card height by number of rows to get total content height
    pub fn content_height(&self, item_count: usize) -> f32 {
        let rows = item_count.div_ceil(self.cols);
        (rows as f32) * (self.card_height + self.padding)
    }

    /// Get position and size for an item at index
    pub fn get_item_rect(&self, index: usize, scroll_y: f32) -> (f32, f32, f32, f32) {
        let col = (index % self.cols) as f32;
        let row = (index / self.cols) as f32;

        // Distribute width
        let total_padding = (self.cols - 1) as f32 * self.padding;
        let card_width = (self.width - total_padding) / self.cols as f32;

        let item_x = self.x + col * (card_width + self.padding);
        let item_y = self.y + row * (self.card_height + self.padding) - scroll_y;

        (item_x, item_y, card_width, self.card_height)
    }
}

/// Helper to handle scrolling logic
/// Returns the new scroll value clamped to 0..max_scroll
pub fn handle_scroll(
    current_scroll: f32,
    total_height: f32,
    view_height: f32,
    scroll_speed: f32,
) -> f32 {
    let (_, wheel_y) = mouse_wheel();
    let mut scroll = current_scroll;

    if wheel_y != 0.0 {
        scroll -= wheel_y * scroll_speed;
    }

    let max_scroll = (total_height - view_height).max(0.0);
    scroll.clamp(0.0, max_scroll)
}

#[cfg(test)]
mod tests;
