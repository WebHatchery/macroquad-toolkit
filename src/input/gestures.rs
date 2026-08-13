//! Touch gestures that keep taps distinct from map movement.
//!
//! Macroquad synthesises left-mouse events for the first finger. That is handy
//! for ordinary buttons, but ambiguous on a game board: the same contact might
//! be a tap, the beginning of a pan, or one half of a pinch. [`TouchGesture`]
//! waits until the intent is known and reports one semantic result.

use macroquad::prelude::*;

/// Movement before a single-finger contact becomes a pan rather than a tap.
pub const DRAG_THRESHOLD: f32 = 6.0;

/// One touch in logical screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestureTouch {
    pub id: u64,
    pub position: Vec2,
    pub phase: TouchPhase,
}

impl GestureTouch {
    pub const fn new(id: u64, position: Vec2, phase: TouchPhase) -> Self {
        Self {
            id,
            position,
            phase,
        }
    }

    fn active(self) -> bool {
        !matches!(self.phase, TouchPhase::Ended | TouchPhase::Cancelled)
    }
}

/// The semantic result of one gesture frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchGestureFrame {
    /// Translation since the previous frame. Zero until a pan is established.
    pub pan: Vec2,
    /// Pinch scale since the previous frame. `1.0` means no zoom.
    pub scale: f32,
    /// Current contact centroid, suitable as the zoom focus.
    pub center: Vec2,
    /// A short, single-finger gesture completed at this position.
    pub tap: Option<Vec2>,
    /// At least one finger is currently on the surface.
    pub active: bool,
    /// The contact has become a pan or pinch and must not activate controls.
    pub claimed: bool,
}

impl Default for TouchGestureFrame {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            scale: 1.0,
            center: Vec2::ZERO,
            tap: None,
            active: false,
            claimed: false,
        }
    }
}

/// Persistent recognizer for one-finger taps/pans and two-finger pan/pinch.
#[derive(Debug, Clone)]
pub struct TouchGesture {
    primary: Option<u64>,
    origin: Vec2,
    last_center: Vec2,
    last_span: f32,
    last_count: usize,
    claimed: bool,
}

impl Default for TouchGesture {
    fn default() -> Self {
        Self::new()
    }
}

impl TouchGesture {
    pub const fn new() -> Self {
        Self {
            primary: None,
            origin: Vec2::ZERO,
            last_center: Vec2::ZERO,
            last_span: 0.0,
            last_count: 0,
            claimed: false,
        }
    }

    /// Read Macroquad's touches and convert physical coordinates to logical
    /// screen coordinates before recognizing the gesture.
    pub fn update(&mut self) -> TouchGestureFrame {
        let dpi = screen_dpi_scale();
        let logical = |position: Vec2| {
            if dpi > 0.0 {
                position / dpi
            } else {
                position
            }
        };
        let touches: Vec<_> = touches()
            .into_iter()
            .map(|touch| GestureTouch::new(touch.id, logical(touch.position), touch.phase))
            .collect();
        self.update_with(&touches)
    }

    /// Recognize a frame from explicit input, allowing deterministic tests and
    /// virtual-coordinate callers.
    pub fn update_with(&mut self, touches: &[GestureTouch]) -> TouchGestureFrame {
        let mut active: Vec<_> = touches
            .iter()
            .copied()
            .filter(|touch| touch.active())
            .collect();
        active.sort_by_key(|touch| touch.id);

        if active.is_empty() {
            let was_claimed = self.claimed;
            let tap = if self.primary.is_some() && !was_claimed && self.last_count == 1 {
                touches
                    .iter()
                    .find(|touch| {
                        Some(touch.id) == self.primary && touch.phase == TouchPhase::Ended
                    })
                    .map(|touch| touch.position)
            } else {
                None
            };
            self.reset();
            return TouchGestureFrame {
                tap,
                claimed: was_claimed,
                ..TouchGestureFrame::default()
            };
        }

        let center = active.iter().map(|touch| touch.position).sum::<Vec2>() / active.len() as f32;
        let count = active.len();
        let starting = self.primary.is_none();
        let primary_changed = !starting && self.primary != Some(active[0].id);

        if starting || primary_changed {
            self.primary = Some(active[0].id);
            self.origin = active[0].position;
            self.last_center = center;
            self.last_span = span(&active);
            self.last_count = count;
            if count > 1 || primary_changed {
                self.claimed = true;
            }
            return TouchGestureFrame {
                center,
                active: true,
                claimed: self.claimed,
                ..TouchGestureFrame::default()
            };
        }

        let count_changed = count != self.last_count;
        if count > 1 || active[0].position.distance(self.origin) > DRAG_THRESHOLD {
            self.claimed = true;
        }

        let current_span = span(&active);
        let pan = if self.claimed && !count_changed {
            center - self.last_center
        } else {
            Vec2::ZERO
        };
        let scale = if count > 1
            && !count_changed
            && self.last_span > f32::EPSILON
            && current_span > f32::EPSILON
        {
            current_span / self.last_span
        } else {
            1.0
        };

        self.last_center = center;
        self.last_span = current_span;
        self.last_count = count;

        TouchGestureFrame {
            pan,
            scale,
            center,
            tap: None,
            active: true,
            claimed: self.claimed,
        }
    }

    fn reset(&mut self) {
        self.primary = None;
        self.origin = Vec2::ZERO;
        self.last_center = Vec2::ZERO;
        self.last_span = 0.0;
        self.last_count = 0;
        self.claimed = false;
    }
}

fn span(touches: &[GestureTouch]) -> f32 {
    if touches.len() < 2 {
        0.0
    } else {
        touches[0].position.distance(touches[1].position)
    }
}

#[cfg(test)]
mod tests;
