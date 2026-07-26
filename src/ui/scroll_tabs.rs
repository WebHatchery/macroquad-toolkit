//! Scrollable regions with rendered scrollbars, and tab/nav bars.
//!
//! Extracted from nanite_swarm's build-palette scrollbar, nightmare_shift's
//! skill-tree/almanac scroll offsets, kaiju_sim's sidebar nav,
//! iron_fauna's codex tab bar, and finallanding's bottom toolbar.

use macroquad::prelude::*;

use crate::colors::{dark, with_alpha};
use crate::input::{is_hovered_rect, was_clicked_rect};
use crate::ui::font::draw_text_centered_in_box;
use crate::ui::{draw_surface, draw_text_centered_in_box_ex, RectExt, SurfaceStyle, TextStyle};

/// Persistent scroll state for a list/panel region: wheel scrolling while
/// hovered, a proportional draggable scrollbar, and offset clamping.
///
/// Keep one `ScrollArea` per scrollable region in your state, call
/// [`update`](Self::update) each frame with the region's rect and total
/// content height, offset your row drawing by [`offset`](Self::offset),
/// then call [`draw_scrollbar`](Self::draw_scrollbar) after the rows.
///
/// The rendered [`offset`](Self::offset) eases toward the wheel/drag target each
/// frame (see [`smoothing`](Self::smoothing)) so the list glides rather than
/// jumping a notch at a time. Under a [`VirtualUi`](crate::ui::VirtualUi) frame,
/// call [`update_at`](Self::update_at) with the logical mouse instead of
/// [`update`](Self::update), which reads the raw window mouse.
#[derive(Debug, Clone, Copy)]
pub struct ScrollArea {
    /// Rendered (eased) offset — what callers subtract from their content's y.
    offset: f32,
    /// Offset the wheel/drag drives; `offset` chases it. Equal once settled.
    target: f32,
    dragging: bool,
    /// Pixels scrolled per wheel notch.
    pub wheel_speed: f32,
    /// Width of the scrollbar drawn at the region's right edge.
    pub bar_width: f32,
    /// Exponential approach rate of `offset` toward its target, in 1/seconds:
    /// higher settles faster, `0.0` disables easing (the offset jumps instantly,
    /// the pre-smoothing behavior). The default (18.0) gives a quick, soft glide.
    pub smoothing: f32,
}

impl Default for ScrollArea {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollArea {
    /// Creates a scroll area with default wheel speed (40px), bar width (8px),
    /// and smoothing (18/s).
    pub fn new() -> Self {
        Self {
            offset: 0.0,
            target: 0.0,
            dragging: false,
            wheel_speed: 40.0,
            bar_width: 8.0,
            smoothing: 18.0,
        }
    }

    /// Current rendered scroll offset in pixels (subtract from your content's y).
    /// Eases toward the wheel/drag target when [`smoothing`](Self::smoothing) > 0.
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// Jumps to a specific offset (clamped on next update), settling the easing
    /// there so it does not glide back.
    pub fn set_offset(&mut self, offset: f32) {
        self.offset = offset.max(0.0);
        self.target = self.offset;
    }

    /// The largest valid offset for the given view/content heights.
    pub fn max_offset(view: Rect, content_height: f32) -> f32 {
        (content_height - view.h).max(0.0)
    }

    /// Handles wheel scrolling while hovered and scrollbar dragging, then eases
    /// and clamps the offset. Call once per frame before drawing content. Uses
    /// the raw window mouse — inside a [`VirtualUi`](crate::ui::VirtualUi) frame
    /// use [`update_at`](Self::update_at) with the logical mouse instead.
    pub fn update(&mut self, view: Rect, content_height: f32) {
        self.update_at(view, content_height, Vec2::from(mouse_position()));
    }

    /// Like [`update`](Self::update) but hit-tests hover and the scrollbar handle
    /// against an explicit `mouse` position, so it works inside a
    /// [`VirtualUi`](crate::ui::VirtualUi) frame (wheel and button state stay
    /// global — they carry no coordinates).
    pub fn update_at(&mut self, view: Rect, content_height: f32, mouse: Vec2) {
        let max_offset = Self::max_offset(view, content_height);

        if view.contains(mouse) {
            let (_, wheel_y) = mouse_wheel();
            if wheel_y != 0.0 {
                self.target -= wheel_y.signum() * self.wheel_speed;
            }
        }

        if max_offset > 0.0 {
            let track = self.track_rect(view);
            if is_mouse_button_pressed(MouseButton::Left) && track.contains(mouse) {
                self.dragging = true;
            }
            if !is_mouse_button_down(MouseButton::Left) {
                self.dragging = false;
            }
            if self.dragging {
                let handle_h = self.handle_height(view, content_height);
                let t = ((mouse.y - view.y - handle_h * 0.5) / (view.h - handle_h)).clamp(0.0, 1.0);
                // Dragging tracks the handle exactly, so settle the easing on it.
                self.target = t * max_offset;
                self.offset = self.target;
            }
        } else {
            self.dragging = false;
        }

        self.target = self.target.clamp(0.0, max_offset);

        // Ease the rendered offset toward the target frame-rate-independently.
        let gap = self.target - self.offset;
        if self.smoothing > 0.0 && gap.abs() > 0.05 {
            let dt = get_frame_time().clamp(0.0, 0.05);
            self.offset += gap * (1.0 - (-self.smoothing * dt).exp());
        } else {
            self.offset = self.target;
        }
        self.offset = self.offset.clamp(0.0, max_offset);
    }

    /// Draws the scrollbar track and proportional handle in the default neutral
    /// palette. No-op when the content fits inside the view. Games with their own
    /// theme should call [`draw_scrollbar_with`](Self::draw_scrollbar_with).
    pub fn draw_scrollbar(&self, view: Rect, content_height: f32) {
        self.draw_scrollbar_with(
            view,
            content_height,
            Color::new(0.1, 0.1, 0.12, 0.8),
            with_alpha(dark::TEXT_DIM, 0.8),
            dark::ACCENT,
        );
    }

    /// Like [`draw_scrollbar`](Self::draw_scrollbar) but with caller-supplied
    /// colors, so a themed UI can match its own palette: `track` behind the bar,
    /// `handle` for the grip at rest, and `handle_active` while it is being
    /// dragged. Pass colors evaluated at draw time so a runtime theme switch is
    /// reflected. No-op when the content fits inside the view.
    pub fn draw_scrollbar_with(
        &self,
        view: Rect,
        content_height: f32,
        track_color: Color,
        handle: Color,
        handle_active: Color,
    ) {
        let max_offset = Self::max_offset(view, content_height);
        if max_offset <= 0.0 {
            return;
        }
        let track = self.track_rect(view);
        draw_rectangle(track.x, track.y, track.w, track.h, track_color);

        let handle_h = self.handle_height(view, content_height);
        let t = self.offset / max_offset;
        let handle_y = view.y + t * (view.h - handle_h);
        let color = if self.dragging { handle_active } else { handle };
        draw_rectangle(track.x + 1.0, handle_y, track.w - 2.0, handle_h, color);
    }

    fn track_rect(&self, view: Rect) -> Rect {
        Rect::new(
            view.right() - self.bar_width,
            view.y,
            self.bar_width,
            view.h,
        )
    }

    fn handle_height(&self, view: Rect, content_height: f32) -> f32 {
        (view.h * (view.h / content_height.max(1.0))).clamp(24.0_f32.min(view.h), view.h)
    }
}

/// True when `item` sits fully within `view` (a small epsilon absorbs
/// rounding). macroquad has no scissor rect, so a [`ScrollArea`] can't clip
/// its rows — cull the partially-scrolled cards at the top and bottom with
/// this so panel edges stay clean.
///
/// ```
/// # use macroquad::prelude::Rect;
/// # use macroquad_toolkit::ui::is_fully_visible;
/// let view = Rect::new(0.0, 0.0, 100.0, 100.0);
/// assert!(is_fully_visible(Rect::new(0.0, 10.0, 100.0, 40.0), view));
/// assert!(!is_fully_visible(Rect::new(0.0, 80.0, 100.0, 40.0), view));
/// ```
pub fn is_fully_visible(item: Rect, view: Rect) -> bool {
    item.y >= view.y - 0.5 && item.bottom() <= view.bottom() + 0.5
}

/// Orientation for [`tab_bar_ex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabOrientation {
    /// Tabs side by side; the active tab is underlined.
    Horizontal,
    /// Tabs stacked; the active tab gets a left accent bar.
    Vertical,
}

/// Draws a horizontal tab bar with equal-width tabs and an accent underline
/// on the active tab. Returns the index clicked this frame, if any.
pub fn tab_bar(rect: Rect, labels: &[&str], active: usize) -> Option<usize> {
    tab_bar_ex(rect, labels, active, TabOrientation::Horizontal)
}

/// Draws a vertical nav column with an accent side bar on the active item.
/// Returns the index clicked this frame, if any.
pub fn nav_column(rect: Rect, labels: &[&str], active: usize) -> Option<usize> {
    tab_bar_ex(rect, labels, active, TabOrientation::Vertical)
}

/// Draws a one-of-N tab/nav bar. Returns the index clicked this frame, if any.
pub fn tab_bar_ex(
    rect: Rect,
    labels: &[&str],
    active: usize,
    orientation: TabOrientation,
) -> Option<usize> {
    if labels.is_empty() {
        return None;
    }
    let count = labels.len() as f32;
    let mut clicked = None;

    for (index, label) in labels.iter().enumerate() {
        let tab = match orientation {
            TabOrientation::Horizontal => {
                let w = rect.w / count;
                Rect::new(rect.x + index as f32 * w, rect.y, w, rect.h)
            }
            TabOrientation::Vertical => {
                let h = rect.h / count;
                Rect::new(rect.x, rect.y + index as f32 * h, rect.w, h)
            }
        };

        let is_active = index == active;
        let hovered = is_hovered_rect(tab);
        let fill = if is_active {
            Color::new(0.22, 0.22, 0.28, 1.0)
        } else if hovered {
            Color::new(0.18, 0.18, 0.22, 1.0)
        } else {
            Color::new(0.14, 0.14, 0.17, 1.0)
        };
        draw_rectangle(tab.x, tab.y, tab.w, tab.h, fill);

        if is_active {
            match orientation {
                TabOrientation::Horizontal => {
                    draw_rectangle(tab.x, tab.bottom() - 3.0, tab.w, 3.0, dark::ACCENT)
                }
                TabOrientation::Vertical => draw_rectangle(tab.x, tab.y, 3.0, tab.h, dark::ACCENT),
            }
        }

        let text_color = if is_active {
            dark::TEXT_BRIGHT
        } else {
            dark::TEXT_DIM
        };
        draw_text_centered_in_box(
            label,
            tab.x + 4.0,
            tab.y,
            tab.w - 8.0,
            tab.h,
            17.0,
            text_color,
        );

        if was_clicked_rect(tab) {
            clicked = Some(index);
        }
    }

    clicked
}

/// Visual styling for [`tab_bar_styled_at`]. The [`Default`] matches the common
/// dark chrome (blue-tinted active fill, bordered tabs, an accent bar on the
/// active tab) so most callers only need `TabStyle::default()`.
#[derive(Debug, Clone, Copy)]
pub struct TabStyle {
    pub active_fill: Color,
    pub hover_fill: Color,
    pub inactive_fill: Color,
    /// Tab border `(width, color)`, or `None` for borderless tabs.
    pub border: Option<(f32, Color)>,
    /// Accent bar on the active tab `(thickness, color)`: a top highlight for
    /// horizontal bars, a left accent for vertical nav columns.
    pub active_accent: Option<(f32, Color)>,
    pub text_size: f32,
    pub active_text: Color,
    pub inactive_text: Color,
    /// Horizontal padding reserved on each side of the label.
    pub text_pad: f32,
}

impl Default for TabStyle {
    fn default() -> Self {
        Self {
            active_fill: Color::new(0.16, 0.22, 0.32, 1.0),
            hover_fill: Color::new(0.12, 0.14, 0.18, 1.0),
            inactive_fill: Color::new(0.08, 0.09, 0.12, 1.0),
            border: Some((1.0, Color::new(0.3, 0.36, 0.46, 0.5))),
            active_accent: Some((3.0, dark::ACCENT)),
            text_size: 17.0,
            active_text: dark::TEXT_BRIGHT,
            inactive_text: dark::TEXT_DIM,
            text_pad: 4.0,
        }
    }
}

/// Mouse-aware, fully styled one-of-N tab/nav bar. Unlike [`tab_bar`], it
/// hit-tests against an explicit logical `mouse` position — so it works inside a
/// [`VirtualUi`](crate::ui::VirtualUi) frame — and takes a [`TabStyle`] so games
/// can match their own chrome. Returns the index clicked this frame, if any.
pub fn tab_bar_styled_at(
    rect: Rect,
    labels: &[&str],
    active: usize,
    orientation: TabOrientation,
    style: &TabStyle,
    mouse: Vec2,
) -> Option<usize> {
    if labels.is_empty() {
        return None;
    }
    let count = labels.len() as f32;
    let mut clicked = None;

    for (index, label) in labels.iter().enumerate() {
        let tab = match orientation {
            TabOrientation::Horizontal => {
                let w = rect.w / count;
                Rect::new(rect.x + index as f32 * w, rect.y, w, rect.h)
            }
            TabOrientation::Vertical => {
                let h = rect.h / count;
                Rect::new(rect.x, rect.y + index as f32 * h, rect.w, h)
            }
        };

        let is_active = index == active;
        let hovered = tab.contains_point(mouse);
        let fill = if is_active {
            style.active_fill
        } else if hovered {
            style.hover_fill
        } else {
            style.inactive_fill
        };

        let mut surface = SurfaceStyle::new(fill);
        if let Some((width, color)) = style.border {
            surface = surface.with_border(width, color);
        }
        if is_active {
            if let Some((thickness, color)) = style.active_accent {
                surface = match orientation {
                    TabOrientation::Horizontal => surface.with_top_highlight(thickness, color),
                    TabOrientation::Vertical => surface.with_left_accent(thickness, color),
                };
            }
        }
        draw_surface(tab, &surface);

        draw_text_centered_in_box_ex(
            label,
            tab.x + style.text_pad,
            tab.y,
            tab.w - style.text_pad * 2.0,
            tab.h,
            TextStyle::new(
                style.text_size,
                if is_active {
                    style.active_text
                } else {
                    style.inactive_text
                },
            ),
        );

        if hovered && is_mouse_button_released(MouseButton::Left) {
            clicked = Some(index);
        }
    }

    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_offset_clamps_to_zero_when_content_fits() {
        let view = Rect::new(0.0, 0.0, 100.0, 200.0);
        assert_eq!(ScrollArea::max_offset(view, 150.0), 0.0);
        assert_eq!(ScrollArea::max_offset(view, 500.0), 300.0);
    }

    #[test]
    fn set_offset_never_negative() {
        let mut area = ScrollArea::new();
        area.set_offset(-50.0);
        assert_eq!(area.offset(), 0.0);
        area.set_offset(120.0);
        assert_eq!(area.offset(), 120.0);
    }
}
