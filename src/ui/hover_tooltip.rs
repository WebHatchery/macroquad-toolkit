//! Hover tooltip with show delay and fade-in/out, for UIs where instant
//! tooltips feel twitchy.
//!
//! Extracted from realmseed's tooltip manager. Unlike the stateless
//! [`draw_tooltip`](super::draw_tooltip), this tracks which widget is
//! hovered across frames: the tooltip appears only after the cursor rests
//! on a widget for `delay` seconds, fades in, and fades out after the
//! cursor leaves.
//!
//! Widgets report hover during UI drawing; the owner draws the tooltip once
//! at the end of the frame so it renders above everything:
//!
//! ```
//! use macroquad::prelude::vec2;
//! use macroquad_toolkit::ui::HoverTooltip;
//!
//! let mut tooltip = HoverTooltip::new();
//! // Each frame, for the hovered widget (now = macroquad::time::get_time()):
//! tooltip.hover("forge-button", "Forge a new blade", vec2(120.0, 300.0), 0.0);
//! // Not visible until the cursor has rested for `delay` seconds:
//! assert!(tooltip.visible(0.1).is_none());
//! tooltip.hover("forge-button", "Forge a new blade", vec2(120.0, 300.0), 0.6);
//! assert!(tooltip.visible(0.6).is_some());
//! // At the end of the frame: tooltip.draw(&style, None, now);
//! ```

use macroquad::prelude::{vec2, Font, Rect, Vec2};

use crate::colors::multiply_alpha;

use super::{draw_tooltip_styled, TooltipStyle};

/// What the tooltip would draw this frame: resolved text, anchor, and fade
/// alpha. Returned by [`HoverTooltip::visible`] for custom rendering.
#[derive(Debug, Clone, Copy)]
pub struct HoverTooltipDraw<'a> {
    pub text: &'a str,
    pub anchor: Vec2,
    pub alpha: f32,
}

#[derive(Debug, Clone)]
struct ActiveHover {
    id: String,
    text: String,
    anchor: Vec2,
    entered_at: f64,
    last_hover_at: f64,
}

/// Delayed, fading hover tooltip state. One instance manages the single
/// tooltip a cursor can point at; hovering a different widget id restarts
/// the show delay.
#[derive(Debug, Clone)]
pub struct HoverTooltip {
    /// Seconds the cursor must rest on a widget before the tooltip shows.
    pub delay: f64,
    /// Seconds to fade in once the delay elapses.
    pub fade_in: f64,
    /// Seconds to fade out after the cursor leaves.
    pub fade_out: f64,
    state: Option<ActiveHover>,
}

impl Default for HoverTooltip {
    fn default() -> Self {
        Self {
            delay: 0.45,
            fade_in: 0.12,
            fade_out: 0.22,
            state: None,
        }
    }
}

impl HoverTooltip {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a tooltip with custom delay / fade-in / fade-out seconds.
    pub fn with_timings(delay: f64, fade_in: f64, fade_out: f64) -> Self {
        Self {
            delay,
            fade_in: fade_in.max(0.001),
            fade_out: fade_out.max(0.001),
            state: None,
        }
    }

    /// Reports that widget `id` is hovered this frame. `now` is the current
    /// time in seconds (`macroquad::time::get_time()`). Hovering a new id
    /// restarts the show delay; re-hovering the current id refreshes its
    /// text and anchor.
    pub fn hover(&mut self, id: &str, text: &str, anchor: Vec2, now: f64) {
        match self.state.as_mut() {
            Some(current) if current.id == id => {
                current.text.clear();
                current.text.push_str(text);
                current.anchor = anchor;
                current.last_hover_at = now;
            }
            _ => {
                self.state = Some(ActiveHover {
                    id: id.to_owned(),
                    text: text.to_owned(),
                    anchor,
                    entered_at: now,
                    last_hover_at: now,
                });
            }
        }
    }

    /// Convenience: reports hover when `point` is inside `rect`, anchoring
    /// the tooltip below the rect's bottom-left corner. Returns whether the
    /// point was inside. Use [`hover`](Self::hover) for custom anchors.
    pub fn hover_rect(&mut self, id: &str, text: &str, rect: Rect, point: Vec2, now: f64) -> bool {
        let hovering = rect.contains(point);
        if hovering {
            self.hover(id, text, vec2(rect.x, rect.y + rect.h), now);
        }
        hovering
    }

    /// Immediately hides the tooltip (e.g. when the hovered widget is
    /// clicked or the panel closes).
    pub fn dismiss(&mut self) {
        self.state = None;
    }

    /// The tooltip to draw this frame, or `None` while the show delay is
    /// running or after the fade-out completed.
    pub fn visible(&self, now: f64) -> Option<HoverTooltipDraw<'_>> {
        let current = self.state.as_ref()?;

        let visible_age = now - current.entered_at - self.delay;
        if visible_age < 0.0 {
            return None;
        }

        let leave_age = now - current.last_hover_at;
        if leave_age > self.fade_out {
            return None;
        }

        let fade_in = (visible_age / self.fade_in).clamp(0.0, 1.0);
        let fade_out = (1.0 - leave_age / self.fade_out).clamp(0.0, 1.0);
        Some(HoverTooltipDraw {
            text: &current.text,
            anchor: current.anchor,
            alpha: (fade_in * fade_out) as f32,
        })
    }

    /// Draws the tooltip if visible, applying the fade alpha to the style's
    /// background, border, and text colors. Call once at the end of the
    /// frame, after all widgets reported hover. Also expires finished
    /// fade-outs.
    pub fn draw(&mut self, style: &TooltipStyle, font: Option<&Font>, now: f64) {
        let Some(current) = self.state.as_ref() else {
            return;
        };
        if now - current.last_hover_at > self.fade_out {
            self.state = None;
            return;
        }

        let Some(draw) = self.visible(now) else {
            return;
        };
        if draw.alpha <= 0.02 {
            return;
        }

        let faded = TooltipStyle {
            background: multiply_alpha(style.background, draw.alpha),
            border: multiply_alpha(style.border, draw.alpha),
            text: multiply_alpha(style.text, draw.alpha),
            ..style.clone()
        };
        draw_tooltip_styled(draw.text, draw.anchor, &faded, font);
    }
}

#[cfg(test)]
mod tests;
