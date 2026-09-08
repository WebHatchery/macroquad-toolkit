//! Zoom an authored, fixed-size interface without changing its layout.
use super::{logical_viewport, sanitize_ui_scale, Pointer, VirtualUi, MIN_TARGET};
use macroquad::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct UiCanvas {
    pub view: Rect,
    pub scale: f32,
    pub origin: Vec2,
    pub content: Vec2,
    pub horizontal: Option<Rect>,
    pub vertical: Option<Rect>,
}

impl UiCanvas {
    /// `pan` selects the visible portion of enlarged content: zero is the
    /// top/left, one the bottom/right. Use 0.5 to initially center the view.
    pub fn new(content: Vec2, zoom: f32, pan: Vec2) -> Self {
        Self::from_screen_size(content, vec2(screen_width(), screen_height()), zoom, pan)
    }

    pub fn from_screen_size(content: Vec2, screen: Vec2, zoom: f32, pan: Vec2) -> Self {
        let base = VirtualUi::from_screen_size(content.x, content.y, screen.x, screen.y);
        let scale = base.scale * sanitize_ui_scale(zoom);
        let size = content * scale;
        let mut view = Rect::new(
            base.offset.x,
            base.offset.y,
            content.x * base.scale,
            content.y * base.scale,
        );
        let enlarged = zoom > 1.0;
        let (horizontal, vertical) = if enlarged {
            view.w = (view.w - MIN_TARGET).max(1.0);
            view.h = (view.h - MIN_TARGET).max(1.0);
            (
                Some(Rect::new(view.x, view.bottom(), view.w, MIN_TARGET)),
                Some(Rect::new(view.right(), view.y, MIN_TARGET, view.h)),
            )
        } else {
            view.x += (view.w - size.x) * 0.5;
            view.y += (view.h - size.y) * 0.5;
            view.w = size.x;
            view.h = size.y;
            (None, None)
        };
        let extent = (content - vec2(view.w, view.h) / scale).max(Vec2::ZERO);
        Self {
            view,
            scale,
            origin: extent * pan.clamp(Vec2::ZERO, Vec2::ONE),
            content,
            horizontal,
            vertical,
        }
    }

    pub fn screen_to_ui(&self, point: Vec2) -> Vec2 {
        (point - self.view.point()) / self.scale + self.origin
    }

    pub fn ui_to_screen(&self, point: Vec2) -> Vec2 {
        (point - self.origin) * self.scale + self.view.point()
    }

    /// Suppress input outside the clipped canvas, including its scrollbars.
    pub fn pointer(&self, pointer: Pointer) -> Pointer {
        let mapped = Pointer {
            position: self.screen_to_ui(pointer.position),
            ..pointer
        };
        if self.view.contains(pointer.position) {
            mapped
        } else {
            mapped.suppressed()
        }
    }

    pub fn begin(&self) {
        let visible = vec2(self.view.w, self.view.h) / self.scale;
        set_camera(&Camera2D {
            target: self.origin + visible * 0.5,
            zoom: vec2(2.0 / visible.x, 2.0 / visible.y),
            viewport: Some(logical_viewport(
                self.view.x,
                screen_height() - self.view.bottom(),
                self.view.w,
                self.view.h,
            )),
            ..Default::default()
        });
    }

    /// Tap or drag either track. Call before constructing the drawing canvas
    /// again with the returned pan. Tracks use screen coordinates.
    pub fn navigate(&self, pointer: Pointer, pan: &mut Vec2) {
        if !pointer.down && !pointer.released {
            return;
        }
        for (track, horizontal) in [(self.horizontal, true), (self.vertical, false)] {
            if let Some(track) = track.filter(|r| r.contains(pointer.position)) {
                let (position, start, length, visible, total) = if horizontal {
                    (
                        pointer.position.x,
                        track.x,
                        track.w,
                        self.view.w,
                        self.content.x * self.scale,
                    )
                } else {
                    (
                        pointer.position.y,
                        track.y,
                        track.h,
                        self.view.h,
                        self.content.y * self.scale,
                    )
                };
                let thumb = (length * visible / total).clamp(MIN_TARGET.min(length), length);
                let value =
                    ((position - start - thumb * 0.5) / (length - thumb).max(1.0)).clamp(0.0, 1.0);
                if horizontal {
                    pan.x = value;
                } else {
                    pan.y = value;
                }
            }
        }
    }

    /// Draw after restoring the default camera. Keep navigation outside the
    /// scaled content so it remains reachable at every zoom level.
    pub fn draw_navigation(&self, pan: Vec2, track_color: Color, thumb_color: Color) {
        for (track, horizontal) in [(self.horizontal, true), (self.vertical, false)] {
            if let Some(track) = track {
                draw_rectangle(track.x, track.y, track.w, track.h, track_color);
                let (length, fraction, position) = if horizontal {
                    (track.w, self.view.w / (self.content.x * self.scale), pan.x)
                } else {
                    (track.h, self.view.h / (self.content.y * self.scale), pan.y)
                };
                let thumb = (length * fraction).clamp(MIN_TARGET.min(length), length);
                let offset = (length - thumb) * position.clamp(0.0, 1.0);
                let rect = if horizontal {
                    Rect::new(track.x + offset, track.y + 12.0, thumb, track.h - 24.0)
                } else {
                    Rect::new(track.x + 12.0, track.y + offset, track.w - 24.0, thumb)
                };
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, thumb_color);
            }
        }
    }
}

#[cfg(test)]
mod tests;
