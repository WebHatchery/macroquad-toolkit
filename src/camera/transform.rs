//! Pure logical-pixel transforms. No window, input polling or drawing is required.

use super::CameraBounds;
use crate::input::gestures::TouchGestureFrame;
use macroquad::prelude::{vec2, Rect, Vec2};

/// Constraint policy is explicit: centering a camera and keeping part of a map
/// visible are different requirements. Neither policy changes zoom.
#[derive(Debug, Clone, Copy)]
pub enum CameraBoundsPolicy {
    TargetInside,
    /// Keep at least this many logical pixels visible along each axis, capped
    /// to the smaller of the viewport and projected map size.
    KeepVisible {
        pixels: f32,
    },
}

/// The world point at a viewport's center and its logical-pixels-per-world-unit scale.
/// Viewports are supplied per operation, so resizing does not invalidate stored state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraTransform {
    target: Vec2,
    zoom: f32,
}

fn valid_viewport(rect: Rect) -> bool {
    [rect.x, rect.y, rect.w, rect.h]
        .iter()
        .all(|v| v.is_finite())
        && rect.w > 0.0
        && rect.h > 0.0
        && (rect.x + rect.w).is_finite()
        && (rect.y + rect.h).is_finite()
}

fn center(rect: Rect) -> Vec2 {
    vec2(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
}

impl CameraTransform {
    pub fn new(target: Vec2, zoom: f32) -> Result<Self, String> {
        if !target.is_finite() || !zoom.is_finite() || zoom <= 0.0 {
            return Err("Camera target must be finite and zoom finite and positive".into());
        }
        Ok(Self { target, zoom })
    }

    pub fn target(self) -> Vec2 {
        self.target
    }
    pub fn zoom(self) -> f32 {
        self.zoom
    }

    /// Points outside the viewport are valid. Hit-testing policy belongs to the caller.
    pub fn screen_to_world(self, viewport: Rect, screen: Vec2) -> Option<Vec2> {
        if !valid_viewport(viewport) || !screen.is_finite() {
            return None;
        }
        let world = self.target + (screen - center(viewport)) / self.zoom;
        world.is_finite().then_some(world)
    }

    pub fn world_to_screen(self, viewport: Rect, world: Vec2) -> Option<Vec2> {
        if !valid_viewport(viewport) || !world.is_finite() {
            return None;
        }
        let screen = center(viewport) + (world - self.target) * self.zoom;
        screen.is_finite().then_some(screen)
    }

    /// Drag content by a logical screen delta. Returns false on invalid input,
    /// leaving the transform unchanged. Positive X moves content to the right.
    pub fn pan_screen(&mut self, delta: Vec2) -> bool {
        let target = self.target - delta / self.zoom;
        if !delta.is_finite() || !target.is_finite() {
            return false;
        }
        self.target = target;
        true
    }

    /// Keep the anchor's world point fixed while changing zoom. Limits must be
    /// finite, positive and ordered. Invalid operations leave the transform unchanged.
    /// Apply bounds separately; constraining a view can necessarily move the anchor.
    pub fn zoom_at(
        &mut self,
        viewport: Rect,
        anchor: Vec2,
        factor: f32,
        limits: (f32, f32),
    ) -> bool {
        if !factor.is_finite()
            || factor <= 0.0
            || !limits.0.is_finite()
            || !limits.1.is_finite()
            || limits.0 <= 0.0
            || limits.1 < limits.0
        {
            return false;
        }
        let Some(world) = self.screen_to_world(viewport, anchor) else {
            return false;
        };
        let zoom = (self.zoom * factor).clamp(limits.0, limits.1);
        let target = world - (anchor - center(viewport)) / zoom;
        if !target.is_finite() {
            return false;
        }
        self.target = target;
        self.zoom = zoom;
        true
    }

    pub fn constrain(
        &mut self,
        viewport: Rect,
        bounds: CameraBounds,
        policy: CameraBoundsPolicy,
    ) -> bool {
        if !valid_viewport(viewport)
            || !bounds.min.is_finite()
            || !bounds.max.is_finite()
            || bounds.min.x > bounds.max.x
            || bounds.min.y > bounds.max.y
        {
            return false;
        }
        let (min, max) = match policy {
            CameraBoundsPolicy::TargetInside => (bounds.min, bounds.max),
            CameraBoundsPolicy::KeepVisible { pixels } => {
                if !pixels.is_finite() || pixels < 0.0 {
                    return false;
                }
                let extent = vec2(viewport.w, viewport.h) / self.zoom;
                let visible = vec2(pixels, pixels) / self.zoom;
                let visible = visible.min(extent).min(bounds.max - bounds.min);
                (
                    bounds.min - extent * 0.5 + visible,
                    bounds.max + extent * 0.5 - visible,
                )
            }
        };
        if !min.is_finite() || !max.is_finite() {
            return false;
        }
        self.target = self.target.clamp(min, max);
        true
    }

    /// Compose the existing recognizer's claimed pan/pinch frame. Unclaimed taps
    /// never move the camera. Pan then zoom at the current centroid, atomically.
    /// The caller owns viewport routing and must suppress controls when claimed.
    pub fn apply_gesture(
        &mut self,
        viewport: Rect,
        frame: &TouchGestureFrame,
        limits: (f32, f32),
    ) -> bool {
        if !frame.claimed {
            return false;
        }
        let mut next = *self;
        if !next.pan_screen(frame.pan) || !next.zoom_at(viewport, frame.center, frame.scale, limits)
        {
            return false;
        }
        *self = next;
        true
    }
}

#[cfg(test)]
mod tests;
