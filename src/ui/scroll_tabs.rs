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

/// Movement, in pixels, before a press inside a scroll area stops being a tap
/// and becomes a drag of the content.
///
/// A finger never holds perfectly still, so zero would turn every tap into a
/// one-pixel scroll and swallow the press. Too large and a short flick does
/// nothing. Six is above the jitter of a deliberate tap and well below the
/// shortest movement anyone means as a swipe.
pub const PAN_THRESHOLD: f32 = 6.0;

/// How fast a released fling loses speed, in 1/seconds.
const FLING_DECAY: f32 = 5.0;

/// Speed below which a fling is over, in pixels/second. Left running, it would
/// creep for several seconds after it had visibly stopped.
const FLING_STOP: f32 = 30.0;

/// One frame of pointer state for a [`ScrollArea`], with no globals read.
///
/// [`update_at`](ScrollArea::update_at) fills this from macroquad and calls
/// [`update_with`](ScrollArea::update_with). Keeping the decision separate from
/// the reading is what lets the drag/fling behaviour be tested without a window
/// — the same reason [`Pointer`](crate::ui::Pointer) takes its position rather
/// than fetching it.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollInput {
    /// Pointer position, in the same space as the view rect.
    pub pointer: Vec2,
    /// Button held, or a finger on the glass.
    pub down: bool,
    /// Button or finger arrived this frame.
    pub pressed: bool,
    /// Wheel delta this frame. Always zero on touch.
    pub wheel: f32,
    /// Seconds since the last frame.
    pub dt: f32,
}

/// Persistent scroll state for a list/panel region: wheel scrolling while
/// hovered, drag-to-scroll with a fling, a proportional draggable scrollbar,
/// and offset clamping.
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
///
/// # Touch
///
/// A wheel is a mouse, and a scrollbar handle is eight pixels wide — on a
/// tablet neither exists, which leaves a list that overflows simply unreadable
/// past its first panelful. So a press inside the region that travels more than
/// [`PAN_THRESHOLD`] drags the content directly and carries its momentum on
/// release.
///
/// That gesture starts life indistinguishable from a tap, and both want the
/// same pixels. **Callers must ask [`absorbs_press`](Self::absorbs_press) before
/// hit-testing anything inside the region** — otherwise a swipe that lifts over
/// a button presses it, which is how a scroll gesture buys something.
#[derive(Debug, Clone, Copy)]
pub struct ScrollArea {
    /// Rendered (eased) offset — what callers subtract from their content's y.
    offset: f32,
    /// Offset the wheel/drag drives; `offset` chases it. Equal once settled.
    target: f32,
    dragging: bool,
    /// Where a press inside the region landed, until it is lifted. Holds the
    /// origin, not the latest position, so the threshold measures the whole
    /// gesture rather than one frame of it.
    grab: Option<Vec2>,
    /// Pointer position last frame, for the per-frame drag delta.
    last: Vec2,
    /// The press in progress has travelled far enough to be a scroll.
    panning: bool,
    /// A pan ended this frame: the release belongs to the gesture, not to
    /// whatever control sits under the finger.
    absorbed: bool,
    /// Drag speed in offset-pixels/second, kept after release as the fling.
    velocity: f32,
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
            grab: None,
            last: Vec2::ZERO,
            panning: false,
            absorbed: false,
            velocity: 0.0,
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

    /// Should controls inside this region ignore the pointer this frame?
    ///
    /// True while a drag-scroll is under way, on the frame it ends, and while
    /// the scrollbar handle is held. A press that became a scroll belongs to the
    /// scroll: without this a swipe that happens to lift over a button presses
    /// it, and every list becomes a minefield on a touchscreen.
    ///
    /// Ask before hit-testing, not after drawing — the rows should still be
    /// drawn, just not listening.
    pub fn absorbs_press(&self) -> bool {
        self.panning || self.absorbed || self.dragging
    }

    /// Handles wheel scrolling while hovered, drag-to-scroll, and scrollbar
    /// dragging, then eases and clamps the offset. Call once per frame before
    /// drawing content. Uses the raw window mouse — inside a
    /// [`VirtualUi`](crate::ui::VirtualUi) frame use
    /// [`update_at`](Self::update_at) with the logical mouse instead.
    pub fn update(&mut self, view: Rect, content_height: f32) {
        self.update_at(view, content_height, Vec2::from(mouse_position()));
    }

    /// Like [`update`](Self::update) but hit-tests against an explicit `mouse`
    /// position, so it works inside a [`VirtualUi`](crate::ui::VirtualUi) frame
    /// (wheel and button state stay global — they carry no coordinates).
    ///
    /// A touch arrives here as a mouse: macroquad raises button and motion
    /// events for it by default, so one path serves both.
    pub fn update_at(&mut self, view: Rect, content_height: f32, mouse: Vec2) {
        let (_, wheel_y) = mouse_wheel();
        self.update_with(
            view,
            content_height,
            ScrollInput {
                pointer: mouse,
                down: is_mouse_button_down(MouseButton::Left),
                pressed: is_mouse_button_pressed(MouseButton::Left),
                wheel: wheel_y,
                dt: get_frame_time(),
            },
        );
    }

    /// The whole scroll decision, from explicit input. See
    /// [`update_at`](Self::update_at) for the version that reads macroquad.
    pub fn update_with(&mut self, view: Rect, content_height: f32, input: ScrollInput) {
        let max_offset = Self::max_offset(view, content_height);
        let dt = input.dt.clamp(0.0, 0.05);

        if max_offset <= 0.0 {
            // Nothing to scroll: no handle to grab, no content to drag, and no
            // press to take from the controls underneath.
            self.dragging = false;
            self.grab = None;
            self.panning = false;
            self.absorbed = false;
            self.velocity = 0.0;
            self.target = 0.0;
            self.offset = 0.0;
            return;
        }

        if view.contains(input.pointer) && input.wheel != 0.0 {
            self.target -= input.wheel.signum() * self.wheel_speed;
            // A wheel notch mid-fling is a correction, not an addition.
            self.velocity = 0.0;
        }

        let track = self.track_rect(view);
        if input.pressed && track.contains(input.pointer) {
            self.dragging = true;
        }
        if !input.down {
            self.dragging = false;
        }
        if self.dragging {
            let handle_h = self.handle_height(view, content_height);
            let t =
                ((input.pointer.y - view.y - handle_h * 0.5) / (view.h - handle_h)).clamp(0.0, 1.0);
            // Dragging tracks the handle exactly, so settle the easing on it.
            self.target = t * max_offset;
            self.offset = self.target;
            self.velocity = 0.0;
        }

        // Drag the content itself. The scrollbar takes precedence: a press in
        // the gutter is aiming at the handle, not at the list.
        if input.pressed && !self.dragging && view.contains(input.pointer) {
            self.grab = Some(input.pointer);
            self.last = input.pointer;
            self.velocity = 0.0;
        }

        if input.down {
            if let Some(origin) = self.grab {
                let travel = input.pointer.y - origin.y;
                if !self.panning && travel.abs() > PAN_THRESHOLD {
                    self.panning = true;
                    // Measure from the threshold, not from the origin and not
                    // from here: the first counts the deadzone as movement and
                    // makes the content jump, the second throws away everything
                    // travelled so far and makes a fast flick start from a
                    // standstill.
                    self.last.y = origin.y + PAN_THRESHOLD * travel.signum();
                }
                if self.panning {
                    let dy = input.pointer.y - self.last.y;
                    self.last = input.pointer;
                    // The content follows the finger: dragging down reveals
                    // what is above, which is a smaller offset.
                    self.target -= dy;
                    self.offset = self.target.clamp(0.0, max_offset);
                    if dt > 0.0 {
                        // Smoothed, because the last frame before a lift is
                        // often a still one and would kill the fling outright.
                        self.velocity = self.velocity * 0.6 + (-dy / dt) * 0.4;
                    }
                }
            }
        } else {
            // Whatever the press was, it is over. A pan keeps its momentum and
            // claims this frame's release from the controls underneath.
            self.absorbed = self.panning;
            self.grab = None;
            self.panning = false;
        }

        // Coast. Runs only once the finger is gone, so it never fights a drag.
        if !input.down && self.velocity != 0.0 {
            self.target += self.velocity * dt;
            self.velocity *= (-FLING_DECAY * dt).exp();
            if self.velocity.abs() < FLING_STOP || self.target <= 0.0 || self.target >= max_offset {
                // Stop at the end stops rather than pushing at them.
                self.velocity = 0.0;
            }
            self.offset = self.target.clamp(0.0, max_offset);
        }

        self.target = self.target.clamp(0.0, max_offset);

        // Ease the rendered offset toward the target frame-rate-independently.
        let gap = self.target - self.offset;
        if self.smoothing > 0.0 && gap.abs() > 0.05 {
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

    const VIEW: Rect = Rect {
        x: 0.0,
        y: 0.0,
        w: 200.0,
        h: 100.0,
    };
    const CONTENT: f32 = 500.0;

    fn at(y: f32, down: bool, pressed: bool) -> ScrollInput {
        ScrollInput {
            // Left of the scrollbar gutter, so these are presses on the list.
            pointer: vec2(50.0, y),
            down,
            pressed,
            wheel: 0.0,
            dt: 1.0 / 60.0,
        }
    }

    /// Press, drag, lift — the gesture the whole feature exists for.
    #[test]
    fn dragging_the_content_scrolls_it() {
        let mut area = ScrollArea::new();
        area.update_with(VIEW, CONTENT, at(80.0, true, true));
        assert_eq!(area.offset(), 0.0, "the press alone must not move anything");

        // Up the screen, past the threshold: the content follows the finger.
        area.update_with(VIEW, CONTENT, at(60.0, true, false));
        assert!(area.offset() > 0.0, "{}", area.offset());
        let after_first = area.offset();
        area.update_with(VIEW, CONTENT, at(40.0, true, false));
        assert!(area.offset() > after_first, "{}", area.offset());
    }

    /// The threshold is the whole point: a tap must not scroll.
    #[test]
    fn a_press_that_barely_moves_is_not_a_scroll() {
        let mut area = ScrollArea::new();
        area.update_with(VIEW, CONTENT, at(80.0, true, true));
        area.update_with(VIEW, CONTENT, at(80.0 - PAN_THRESHOLD + 1.0, true, false));
        assert_eq!(area.offset(), 0.0);
        assert!(!area.absorbs_press());
    }

    /// The gesture and the controls under it want the same pixels, and the
    /// scroll wins — a swipe that lifts over a button must not press it.
    #[test]
    fn a_swipe_takes_the_release_from_the_controls_underneath() {
        let mut area = ScrollArea::new();
        area.update_with(VIEW, CONTENT, at(80.0, true, true));
        area.update_with(VIEW, CONTENT, at(40.0, true, false));
        assert!(area.absorbs_press(), "mid-drag");

        // The lift itself: still absorbed, because this is the frame the button
        // underneath would otherwise fire on.
        area.update_with(VIEW, CONTENT, at(40.0, false, false));
        assert!(area.absorbs_press(), "on release");

        // And released again the next frame, or nothing would ever be clickable.
        area.update_with(VIEW, CONTENT, at(40.0, false, false));
        assert!(!area.absorbs_press(), "the frame after");
    }

    #[test]
    fn a_tap_leaves_the_press_for_the_controls_underneath() {
        let mut area = ScrollArea::new();
        area.update_with(VIEW, CONTENT, at(80.0, true, true));
        area.update_with(VIEW, CONTENT, at(80.0, false, false));
        assert!(!area.absorbs_press());
    }

    #[test]
    fn a_released_drag_coasts_and_then_stops() {
        let mut area = ScrollArea::new();
        area.update_with(VIEW, CONTENT, at(90.0, true, true));
        for step in 1..=4 {
            area.update_with(VIEW, CONTENT, at(90.0 - step as f32 * 20.0, true, false));
        }
        let at_release = area.offset();
        area.update_with(VIEW, CONTENT, at(10.0, false, false));
        assert!(area.offset() > at_release, "the fling should carry on");

        // And settle, rather than creeping forever.
        for _ in 0..600 {
            area.update_with(VIEW, CONTENT, at(10.0, false, false));
        }
        let settled = area.offset();
        area.update_with(VIEW, CONTENT, at(10.0, false, false));
        assert_eq!(area.offset(), settled);
    }

    #[test]
    fn a_fling_stops_at_the_end_of_the_content() {
        let mut area = ScrollArea::new();
        area.update_with(VIEW, CONTENT, at(95.0, true, true));
        for step in 1..=8 {
            area.update_with(VIEW, CONTENT, at(95.0 - step as f32 * 11.0, true, false));
        }
        for _ in 0..600 {
            area.update_with(VIEW, CONTENT, at(5.0, false, false));
        }
        assert!(area.offset() <= ScrollArea::max_offset(VIEW, CONTENT) + 0.01);
        assert!(area.offset() >= 0.0);
    }

    /// Content that fits has nothing to scroll, so it must not eat presses —
    /// otherwise a short list becomes unclickable for the price of a feature it
    /// never uses.
    #[test]
    fn a_region_with_nothing_to_scroll_never_absorbs_a_press() {
        let mut area = ScrollArea::new();
        area.update_with(VIEW, 60.0, at(80.0, true, true));
        area.update_with(VIEW, 60.0, at(20.0, true, false));
        assert_eq!(area.offset(), 0.0);
        assert!(!area.absorbs_press());
    }

    /// A press in the right-edge gutter is aiming at the handle, and dragging
    /// it must still track the handle rather than the content — the two read
    /// opposite ways round.
    #[test]
    fn the_scrollbar_handle_still_wins_the_gutter() {
        let mut area = ScrollArea::new();
        let gutter = ScrollInput {
            pointer: vec2(VIEW.right() - 2.0, 90.0),
            down: true,
            pressed: true,
            wheel: 0.0,
            dt: 1.0 / 60.0,
        };
        area.update_with(VIEW, CONTENT, gutter);
        // Handle dragged to the bottom of the track: the end of the content,
        // where content-dragging the same way would have gone to the start.
        assert!(area.offset() > ScrollArea::max_offset(VIEW, CONTENT) * 0.5);
        assert!(area.absorbs_press());
    }

    #[test]
    fn the_wheel_still_scrolls_for_a_mouse() {
        let mut area = ScrollArea::new();
        area.update_with(
            VIEW,
            CONTENT,
            ScrollInput {
                pointer: vec2(50.0, 50.0),
                down: false,
                pressed: false,
                wheel: -1.0,
                dt: 1.0 / 60.0,
            },
        );
        assert!(area.offset() > 0.0);
    }
}
