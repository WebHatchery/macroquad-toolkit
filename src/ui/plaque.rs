//! Ornamented "plaque" buttons and corner marks for title screens and
//! pause/settings menus: a framed two-layer button with a drop shadow,
//! state-tinted face, and decorative corner ticks.
//!
//! Extracted from the near-identical title/menu widgets in toybox and
//! the_enchanters_ledger (with scrapyard's keyboard-selected variant).
//! The style struct exposes the hooks those games differ on — bevel band,
//! inner borders, top highlight, and corner-mark placement — so each keeps
//! its look by configuring a [`PlaqueStyle`] + [`PlaquePalette`] per tone.
//!
//! ```no_run
//! use macroquad::prelude::*;
//! use macroquad_toolkit::ui::{plaque_button, PlaquePalette, PlaqueStyle};
//!
//! let style = PlaqueStyle::default();
//! let palette = PlaquePalette::default();
//! let mouse = Vec2::from(mouse_position());
//! if plaque_button(
//!     Rect::new(100.0, 200.0, 148.0, 38.0),
//!     "New Game",
//!     &style,
//!     &palette,
//!     true,
//!     mouse,
//! ) {
//!     // start the game
//! }
//! ```

use macroquad::prelude::*;

use super::{draw_surface, draw_text_centered_in_box_ex, RectExt, SurfaceStyle, TextStyle};

/// Geometry of L-shaped corner tick marks.
#[derive(Debug, Clone, Copy)]
pub struct CornerMarkSpec {
    /// Length of each tick arm in pixels.
    pub len: f32,
    /// Distance from the rect edge.
    pub gap: f32,
    /// Line thickness.
    pub thickness: f32,
}

impl Default for CornerMarkSpec {
    fn default() -> Self {
        Self {
            len: 11.0,
            gap: 6.0,
            thickness: 1.0,
        }
    }
}

/// Draws L-shaped tick marks in all four corners of `rect` with the default
/// geometry.
pub fn draw_corner_marks(rect: Rect, color: Color) {
    draw_corner_marks_spec(rect, color, &CornerMarkSpec::default());
}

/// Draws L-shaped tick marks in all four corners of `rect`.
pub fn draw_corner_marks_spec(rect: Rect, color: Color, spec: &CornerMarkSpec) {
    let CornerMarkSpec {
        len,
        gap,
        thickness,
    } = *spec;
    // (corner x, corner y, x arm direction, y arm direction)
    let corners = [
        (rect.x + gap, rect.y + gap, 1.0, 1.0),
        (rect.right() - gap, rect.y + gap, -1.0, 1.0),
        (rect.x + gap, rect.bottom() - gap, 1.0, -1.0),
        (rect.right() - gap, rect.bottom() - gap, -1.0, -1.0),
    ];
    for (x, y, dx, dy) in corners {
        draw_line(x, y, x + len * dx, y, thickness, color);
        draw_line(x, y, x, y + len * dy, thickness, color);
    }
}

/// Face and text colors for one plaque tone. Games typically build one per
/// [`ButtonTone`](super::ButtonTone)-like semantic.
#[derive(Debug, Clone, Copy)]
pub struct PlaquePalette {
    pub normal: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub disabled: Color,
    pub border: Color,
    pub text: Color,
}

impl Default for PlaquePalette {
    /// Neutral dark palette, close to toybox's muted tone.
    fn default() -> Self {
        Self {
            normal: Color::new(0.080, 0.083, 0.090, 0.90),
            hovered: Color::new(0.13, 0.135, 0.145, 0.96),
            pressed: Color::new(0.055, 0.058, 0.064, 0.98),
            disabled: Color::new(0.055, 0.055, 0.060, 0.68),
            border: Color::new(0.54, 0.48, 0.38, 0.72),
            text: Color::new(0.92, 0.82, 0.62, 1.0),
        }
    }
}

impl PlaquePalette {
    /// The face color for an interaction state.
    pub fn face(&self, state: PlaqueState) -> Color {
        if !state.enabled {
            self.disabled
        } else if state.pressed {
            self.pressed
        } else if state.hovered || state.selected {
            self.hovered
        } else {
            self.normal
        }
    }
}

/// Interaction state a plaque is drawn in. `selected` is for keyboard-driven
/// menus where a cursor highlights an option without hovering it.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlaqueState {
    pub enabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub selected: bool,
}

impl PlaqueState {
    /// An idle (not hovered/pressed/selected) state.
    pub fn idle(enabled: bool) -> Self {
        Self {
            enabled,
            ..Self::default()
        }
    }
}

/// Which rect the corner marks decorate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaqueMarkPlacement {
    /// On the outer frame rect (toybox's title buttons).
    Frame,
    /// On the inset face rect (the_enchanters_ledger's buttons).
    Face,
}

/// Corner-mark hook for a plaque.
#[derive(Debug, Clone, Copy)]
pub struct PlaqueMarks {
    pub spec: CornerMarkSpec,
    pub placement: PlaqueMarkPlacement,
    /// Mark color when enabled; `None` uses the face border color.
    pub color: Option<Color>,
    pub disabled_color: Color,
}

impl Default for PlaqueMarks {
    fn default() -> Self {
        Self {
            spec: CornerMarkSpec::default(),
            placement: PlaqueMarkPlacement::Frame,
            color: None,
            disabled_color: Color::new(0.30, 0.28, 0.24, 0.60),
        }
    }
}

/// Raised-band bevel across the top of the face (the_enchanters_ledger's
/// polished-metal look).
#[derive(Debug, Clone, Copy)]
pub struct PlaqueBevel {
    /// Height of the lightened band as a fraction of the face height.
    pub band_frac: f32,
    /// Amount added to each face RGB channel inside the band.
    pub lift: f32,
    /// Bright line under the band's top edge.
    pub highlight: Color,
    /// Dark line above the face's bottom edge.
    pub shade: Color,
    /// Alpha multiplier applied to highlight/shade when disabled.
    pub disabled_fade: f32,
}

impl Default for PlaqueBevel {
    fn default() -> Self {
        Self {
            band_frac: 0.38,
            lift: 0.05,
            highlight: Color::new(1.0, 0.82, 0.44, 0.28),
            shade: Color::new(0.0, 0.0, 0.0, 0.30),
            disabled_fade: 0.4,
        }
    }
}

/// Style hooks for [`draw_plaque`]. The defaults reproduce a dark
/// gold-trimmed title button; games override the pieces their look differs
/// on.
#[derive(Debug, Clone)]
pub struct PlaqueStyle {
    /// Drop shadow offset and color behind the frame.
    pub shadow: Option<(Vec2, Color)>,
    /// The outer frame surface (fill, border, inner border).
    pub frame: SurfaceStyle,
    /// Gap between the frame rect and the face rect.
    pub inset: f32,
    pub face_border_width: f32,
    /// Optional (inset, width, color) hairline inside the face.
    pub face_inner_border: Option<(f32, f32, Color)>,
    /// Optional (height, color) highlight along the face's top edge.
    pub top_highlight: Option<(f32, Color)>,
    pub bevel: Option<PlaqueBevel>,
    pub corner_marks: Option<PlaqueMarks>,
    /// Face border color when disabled.
    pub disabled_border: Color,
    /// Label color when disabled.
    pub disabled_text: Color,
    /// Face border color override while selected.
    pub selected_border: Option<Color>,
    /// Face border width while selected (falls back to `face_border_width`).
    pub selected_border_width: Option<f32>,
    /// Fixed label size; `None` fits the label to the plaque height.
    pub font_size: Option<f32>,
    /// How far the label shifts down while pressed.
    pub press_nudge: f32,
}

impl Default for PlaqueStyle {
    fn default() -> Self {
        Self {
            shadow: Some((vec2(2.0, 3.0), Color::new(0.015, 0.012, 0.010, 0.58))),
            frame: SurfaceStyle::new(Color::new(0.040, 0.036, 0.030, 0.94))
                .with_border(1.0, Color::new(0.18, 0.12, 0.06, 0.92))
                .with_inner_border(2.0, 1.0, Color::new(0.80, 0.55, 0.24, 0.20)),
            inset: 3.0,
            face_border_width: 1.0,
            face_inner_border: None,
            top_highlight: Some((2.0, Color::new(1.0, 0.82, 0.42, 0.20))),
            bevel: None,
            corner_marks: Some(PlaqueMarks::default()),
            disabled_border: Color::new(0.30, 0.28, 0.24, 0.80),
            disabled_text: Color::new(0.42, 0.40, 0.36, 1.0),
            selected_border: None,
            selected_border_width: None,
            font_size: None,
            press_nudge: 1.0,
        }
    }
}

impl PlaqueStyle {
    /// The face border color for a state.
    pub fn border_color(&self, palette: &PlaquePalette, state: PlaqueState) -> Color {
        if state.selected {
            if let Some(color) = self.selected_border {
                return color;
            }
        }
        if state.enabled {
            palette.border
        } else {
            self.disabled_border
        }
    }

    /// The label font size for a plaque of height `h`.
    pub fn label_size(&self, h: f32) -> f32 {
        self.font_size.unwrap_or((h * 0.42).clamp(10.0, 17.0))
    }
}

/// Draws a plaque (frame + state-tinted face + decorations) without a label.
pub fn draw_plaque(rect: Rect, style: &PlaqueStyle, palette: &PlaquePalette, state: PlaqueState) {
    if let Some((offset, color)) = style.shadow {
        draw_rectangle(rect.x + offset.x, rect.y + offset.y, rect.w, rect.h, color);
    }
    draw_surface(rect, &style.frame);

    let inset = rect.inset(style.inset);
    let face = palette.face(state);
    let border = style.border_color(palette, state);
    let border_width = if state.selected {
        style
            .selected_border_width
            .unwrap_or(style.face_border_width)
    } else {
        style.face_border_width
    };

    let mut face_style = SurfaceStyle::new(face).with_border(border_width, border);
    if let Some((border_inset, width, color)) = style.face_inner_border {
        face_style = face_style.with_inner_border(border_inset, width, color);
    }
    if let Some((height, color)) = style.top_highlight {
        face_style = face_style.with_top_highlight(height, color);
    }
    draw_surface(inset, &face_style);

    if let Some(bevel) = &style.bevel {
        let fade = if state.enabled {
            1.0
        } else {
            bevel.disabled_fade
        };
        let band = Color::new(
            (face.r + bevel.lift).min(1.0),
            (face.g + bevel.lift).min(1.0),
            (face.b + bevel.lift).min(1.0),
            face.a,
        );
        draw_rectangle(
            inset.x + 2.0,
            inset.y + 2.0,
            (inset.w - 4.0).max(0.0),
            inset.h * bevel.band_frac,
            band,
        );
        let highlight = crate::colors::multiply_alpha(bevel.highlight, fade);
        let shade = crate::colors::multiply_alpha(bevel.shade, fade);
        draw_line(
            inset.x + 4.0,
            inset.y + 4.0,
            inset.right() - 4.0,
            inset.y + 4.0,
            1.0,
            highlight,
        );
        draw_line(
            inset.x + 4.0,
            inset.bottom() - 4.0,
            inset.right() - 4.0,
            inset.bottom() - 4.0,
            1.0,
            shade,
        );
    }

    if let Some(marks) = &style.corner_marks {
        let target = match marks.placement {
            PlaqueMarkPlacement::Frame => rect,
            PlaqueMarkPlacement::Face => inset,
        };
        let color = if state.enabled {
            marks.color.unwrap_or(border)
        } else {
            marks.disabled_color
        };
        draw_corner_marks_spec(target, color, &marks.spec);
    }
}

/// A plaque button driven by an explicit mouse position (pass your logical /
/// virtual-resolution cursor). Returns true when activated (released over
/// the button).
pub fn plaque_button(
    rect: Rect,
    text: &str,
    style: &PlaqueStyle,
    palette: &PlaquePalette,
    enabled: bool,
    mouse: Vec2,
) -> bool {
    plaque_button_ex(rect, text, style, palette, enabled, false, mouse)
}

/// [`plaque_button`] with a keyboard-selection highlight for menus driven by
/// a cursor (e.g. [`MenuCursor`](crate::input::MenuCursor)).
pub fn plaque_button_ex(
    rect: Rect,
    text: &str,
    style: &PlaqueStyle,
    palette: &PlaquePalette,
    enabled: bool,
    selected: bool,
    mouse: Vec2,
) -> bool {
    let hovered = enabled && rect.contains_point(mouse);
    let pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let activated = hovered && is_mouse_button_released(MouseButton::Left);
    let state = PlaqueState {
        enabled,
        hovered,
        pressed,
        selected,
    };
    draw_plaque(rect, style, palette, state);

    let text_color = if enabled {
        palette.text
    } else {
        style.disabled_text
    };
    let nudge = if pressed { style.press_nudge } else { 0.0 };
    draw_text_centered_in_box_ex(
        text,
        rect.x + 8.0,
        rect.y + nudge - 1.0,
        rect.w - 16.0,
        rect.h,
        TextStyle::new(style.label_size(rect.h), text_color),
    );
    activated
}

#[cfg(test)]
mod tests;
