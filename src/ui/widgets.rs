//! Button widgets: styles, tones, and press/release trigger variants over
//! plain rects or `UiRect`, with enabled/disabled and explicit-mouse forms.

use crate::colors::dark;
use crate::input::*;
use macroquad::prelude::*;

use super::{
    default_ui_font, draw_surface, draw_surface_with_title, draw_text_centered_in_box,
    draw_text_centered_in_box_ex, effective_font_size, effective_line_gap, font_size_u16,
    wrap_text_ex, RectExt, SurfaceStyle, TextStyle,
};

/// Mouse event used by a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonTrigger {
    Press,
    Release,
}

/// Semantic button tone for common UI actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonTone {
    Primary,
    Secondary,
    Positive,
    Warning,
    Danger,
    Muted,
}

/// Style configuration for buttons
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub border: Color,
    pub text_color: Color,
    pub disabled: Color,
}

impl ButtonStyle {
    /// Default dark theme button style
    pub fn default_dark() -> Self {
        Self {
            normal: dark::PANEL,
            hovered: dark::HOVERED,
            pressed: Color::new(0.25, 0.35, 0.5, 1.0),
            border: dark::ACCENT,
            text_color: dark::TEXT,
            disabled: Color::new(0.1, 0.1, 0.1, 1.0),
        }
    }

    /// Style from a semantic button tone.
    pub fn from_tone(tone: ButtonTone) -> Self {
        match tone {
            ButtonTone::Primary => Self {
                normal: dark::ACCENT,
                hovered: Color::new(0.35, 0.55, 0.9, 1.0),
                pressed: Color::new(0.18, 0.32, 0.58, 1.0),
                border: dark::TEXT_BRIGHT,
                text_color: dark::TEXT_BRIGHT,
                disabled: Color::new(0.14, 0.16, 0.2, 1.0),
            },
            ButtonTone::Secondary => Self::default_dark(),
            ButtonTone::Positive => Self {
                normal: dark::POSITIVE,
                hovered: Color::new(0.35, 0.75, 0.45, 1.0),
                pressed: Color::new(0.12, 0.42, 0.22, 1.0),
                border: dark::TEXT_BRIGHT,
                text_color: dark::TEXT_BRIGHT,
                disabled: Color::new(0.12, 0.18, 0.14, 1.0),
            },
            ButtonTone::Warning => Self {
                normal: dark::WARNING,
                hovered: Color::new(0.95, 0.72, 0.25, 1.0),
                pressed: Color::new(0.55, 0.34, 0.08, 1.0),
                border: dark::TEXT_BRIGHT,
                text_color: dark::TEXT_BRIGHT,
                disabled: Color::new(0.2, 0.16, 0.08, 1.0),
            },
            ButtonTone::Danger => Self {
                normal: dark::NEGATIVE,
                hovered: Color::new(0.9, 0.32, 0.32, 1.0),
                pressed: Color::new(0.55, 0.12, 0.12, 1.0),
                border: dark::TEXT_BRIGHT,
                text_color: dark::TEXT_BRIGHT,
                disabled: Color::new(0.18, 0.1, 0.1, 1.0),
            },
            ButtonTone::Muted => Self {
                normal: Color::new(0.12, 0.12, 0.14, 1.0),
                hovered: Color::new(0.18, 0.18, 0.22, 1.0),
                pressed: Color::new(0.08, 0.08, 0.1, 1.0),
                border: dark::TEXT_DIM,
                text_color: dark::TEXT_DIM,
                disabled: Color::new(0.08, 0.08, 0.09, 1.0),
            },
        }
    }
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self::default_dark()
    }
}

/// Draw a button with default styling. Returns true if clicked (on mouse release).
pub fn button(x: f32, y: f32, w: f32, h: f32, text: &str) -> bool {
    button_styled(x, y, w, h, text, &ButtonStyle::default())
}

/// Draw a button with custom styling. Returns true if clicked (on mouse release).
pub fn button_styled(x: f32, y: f32, w: f32, h: f32, text: &str, style: &ButtonStyle) -> bool {
    button_on_release(x, y, w, h, text, style)
}

/// Draw a button from a `Rect` using default styling.
pub fn button_rect(rect: Rect, text: &str) -> bool {
    button_rect_styled(rect, text, &ButtonStyle::default())
}

/// Draw a button from a `Rect` using custom styling.
pub fn button_rect_styled(rect: Rect, text: &str, style: &ButtonStyle) -> bool {
    button_rect_enabled_styled(rect, text, true, style)
}

/// Draw an enabled/disabled button with default styling.
pub fn button_enabled(x: f32, y: f32, w: f32, h: f32, text: &str, enabled: bool) -> bool {
    button_rect_enabled(Rect::new(x, y, w, h), text, enabled)
}

/// Draw an enabled/disabled button from a `Rect`.
pub fn button_rect_enabled(rect: Rect, text: &str, enabled: bool) -> bool {
    button_rect_enabled_styled(rect, text, enabled, &ButtonStyle::default())
}

/// Draw an enabled/disabled button with a semantic tone.
pub fn button_rect_tone(rect: Rect, text: &str, enabled: bool, tone: ButtonTone) -> bool {
    let style = ButtonStyle::from_tone(tone);
    button_rect_enabled_styled(rect, text, enabled, &style)
}

/// Draw an enabled/disabled button from a `Rect` using custom styling.
pub fn button_rect_enabled_styled(
    rect: Rect,
    text: &str,
    enabled: bool,
    style: &ButtonStyle,
) -> bool {
    button_rect_enabled_styled_ex(
        rect,
        text,
        enabled,
        style,
        TextStyle::new(20.0, style.text_color),
        ButtonTrigger::Release,
    )
}

/// Font-aware, `Rect`-based button renderer.
pub fn button_rect_enabled_styled_ex(
    rect: Rect,
    text: &str,
    enabled: bool,
    style: &ButtonStyle,
    text_style: TextStyle<'_>,
    trigger: ButtonTrigger,
) -> bool {
    button_rect_enabled_styled_ex_at(
        rect,
        text,
        enabled,
        style,
        text_style,
        trigger,
        Vec2::from(mouse_position()),
    )
}

/// Mouse-aware variant of [`button_rect_tone`]: hit-tests against an explicit
/// logical `mouse` position. Use this inside a [`VirtualUi`](crate::ui::VirtualUi)
/// frame, where the raw screen mouse is in physical pixels and would miss the
/// letterboxed layout.
pub fn button_rect_tone_at(
    rect: Rect,
    text: &str,
    enabled: bool,
    tone: ButtonTone,
    mouse: Vec2,
) -> bool {
    let style = ButtonStyle::from_tone(tone);
    button_rect_enabled_styled_ex_at(
        rect,
        text,
        enabled,
        &style,
        TextStyle::new(20.0, style.text_color),
        ButtonTrigger::Release,
        mouse,
    )
}

/// Like [`button_rect_enabled_styled_ex`], but hit-tests against an explicit
/// logical `mouse` position instead of the raw screen mouse. This is the
/// [`VirtualUi`](crate::ui::VirtualUi)-safe form; `plaque_button` follows the
/// same convention. The plain `_ex` renderer delegates here with the screen
/// mouse.
#[allow(clippy::too_many_arguments)]
pub fn button_rect_enabled_styled_ex_at(
    rect: Rect,
    text: &str,
    enabled: bool,
    style: &ButtonStyle,
    text_style: TextStyle<'_>,
    trigger: ButtonTrigger,
    mouse: Vec2,
) -> bool {
    let hovered = enabled && rect.contains_point(mouse);
    let is_pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let activated = match trigger {
        ButtonTrigger::Press => hovered && is_mouse_button_pressed(MouseButton::Left),
        ButtonTrigger::Release => hovered && is_mouse_button_released(MouseButton::Left),
    };

    let bg_color = if !enabled {
        style.disabled
    } else if is_pressed {
        style.pressed
    } else if hovered {
        style.hovered
    } else {
        style.normal
    };

    let text_color = if enabled {
        text_style.color
    } else {
        Color::new(
            text_style.color.r,
            text_style.color.g,
            text_style.color.b,
            0.45,
        )
    };

    let surface = SurfaceStyle::new(bg_color).with_border(2.0, style.border);
    draw_surface(rect, &surface);

    let y_offset = if is_pressed { 2.0 } else { 0.0 };
    draw_text_centered_in_box_ex(
        text,
        rect.x + 8.0,
        rect.y + y_offset,
        rect.w - 16.0,
        rect.h,
        TextStyle {
            color: text_color,
            ..text_style
        },
    );

    activated
}

/// Draw a button that triggers on mouse press (button down).
/// Returns true when mouse button is pressed down over the button.
pub fn button_on_press(x: f32, y: f32, w: f32, h: f32, text: &str, style: &ButtonStyle) -> bool {
    let hovered = is_hovered(x, y, w, h);
    let is_pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let clicked = hovered && is_mouse_button_pressed(MouseButton::Left);

    // Determine button color
    let bg_color = if is_pressed {
        style.pressed
    } else if hovered {
        style.hovered
    } else {
        style.normal
    };

    let surface = SurfaceStyle::new(bg_color).with_border(2.0, style.border);
    draw_surface(Rect::new(x, y, w, h), &surface);

    // Text offset for press effect
    let y_offset = if is_pressed { 2.0 } else { 0.0 };

    draw_text_centered_in_box(
        text,
        x + 8.0,
        y + y_offset,
        w - 16.0,
        h,
        20.0,
        style.text_color,
    );

    clicked
}

/// Draw a button that triggers on mouse release (button up).
/// Returns true when mouse button is released over the button.
/// This is the safer default as it prevents accidental double-clicks.
pub fn button_on_release(x: f32, y: f32, w: f32, h: f32, text: &str, style: &ButtonStyle) -> bool {
    let hovered = is_hovered(x, y, w, h);
    let is_pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let clicked = hovered && is_mouse_button_released(MouseButton::Left);

    // Determine button color
    let bg_color = if is_pressed {
        style.pressed
    } else if hovered {
        style.hovered
    } else {
        style.normal
    };

    let surface = SurfaceStyle::new(bg_color).with_border(2.0, style.border);
    draw_surface(Rect::new(x, y, w, h), &surface);

    // Text offset for press effect
    let y_offset = if is_pressed { 2.0 } else { 0.0 };

    draw_text_centered_in_box(
        text,
        x + 8.0,
        y + y_offset,
        w - 16.0,
        h,
        20.0,
        style.text_color,
    );

    clicked
}

/// Draw a button with explicit colors (simplified wrapper)
pub fn colored_button(x: f32, y: f32, w: f32, h: f32, text: &str, color: Color) -> bool {
    let style = ButtonStyle {
        normal: color,
        hovered: Color::new(color.r * 1.2, color.g * 1.2, color.b * 1.2, color.a),
        pressed: Color::new(color.r * 0.8, color.g * 0.8, color.b * 0.8, color.a),
        border: dark::TEXT_DIM,
        text_color: dark::TEXT_BRIGHT,
        ..ButtonStyle::default()
    };
    button_on_release(x, y, w, h, text, &style)
}

/// Draw a simple window/modal frame
pub fn window(x: f32, y: f32, w: f32, h: f32, title: Option<&str>, close_button: bool) -> bool {
    let mut surface = SurfaceStyle::new(dark::PANEL)
        .with_shadow(vec2(4.0, 4.0), Color::new(0.0, 0.0, 0.0, 0.5))
        .with_border(2.0, dark::ACCENT);
    if title.is_some() {
        surface = surface.with_header(30.0, dark::PANEL_HEADER);
    }
    draw_surface_with_title(
        Rect::new(x, y, w, h),
        title,
        &surface,
        TextStyle::new(20.0, dark::TEXT),
    );

    // Close button
    if close_button {
        let btn_size = 24.0;
        let btn_x = x + w - btn_size - 3.0;
        let btn_y = y + 3.0;

        let style = ButtonStyle {
            normal: dark::NEGATIVE,
            hovered: Color::new(0.9, 0.4, 0.4, 1.0),
            pressed: Color::new(0.7, 0.2, 0.2, 1.0),
            border: dark::TEXT,
            text_color: dark::TEXT_BRIGHT,
            ..ButtonStyle::default()
        };

        if button_on_release(btn_x, btn_y, btn_size, btn_size, "X", &style) {
            return true;
        }
    }

    false
}

/// Draw a panel with optional title
pub fn panel(x: f32, y: f32, w: f32, h: f32, title: Option<&str>) {
    let mut style = SurfaceStyle::dark_panel();
    if title.is_some() {
        style = style.with_header(30.0, dark::PANEL_HEADER);
    }
    draw_surface_with_title(
        Rect::new(x, y, w, h),
        title,
        &style,
        TextStyle::new(20.0, dark::TEXT),
    );
}

/// Draw a panel with shadow effect
pub fn panel_with_shadow(x: f32, y: f32, w: f32, h: f32) {
    let style = SurfaceStyle::new(dark::PANEL)
        .with_shadow(vec2(4.0, 4.0), Color::new(0.0, 0.0, 0.0, 0.5))
        .with_border(2.0, dark::TEXT_DIM)
        .with_inner_border(2.0, 1.0, Color::new(0.2, 0.2, 0.22, 0.5));
    draw_surface(Rect::new(x, y, w, h), &style);
}

/// Draw a progress bar
pub fn progress_bar(x: f32, y: f32, w: f32, h: f32, value: f32, max: f32, color: Color) {
    let ratio = if max <= 0.0 {
        0.0
    } else {
        (value / max).clamp(0.0, 1.0)
    };
    let fill_width = ratio * w;

    // Background
    draw_rectangle(x, y, w, h, Color::new(0.15, 0.15, 0.15, 1.0));

    // Fill
    draw_rectangle(x, y, fill_width, h, color);

    // Border
    draw_rectangle_lines(x, y, w, h, 1.0, dark::TEXT_DIM);
}

#[allow(clippy::too_many_arguments)]
/// Draw a progress bar with centered label
pub fn progress_bar_labeled(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    value: f32,
    max: f32,
    label: &str,
    color: Color,
) {
    progress_bar(x, y, w, h, value, max, color);

    draw_text_centered_in_box(label, x + 6.0, y, w - 12.0, h, 16.0, dark::TEXT);
}

/// Draw a section panel with title header - common for UI sections
pub fn section_panel(x: f32, y: f32, w: f32, h: f32, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.1, 0.1, 0.15, 0.85))
        .with_border(1.0, Color::new(0.4, 0.4, 0.6, 0.5));
    draw_surface_with_title(
        Rect::new(x, y, w, h),
        Some(title),
        &style,
        TextStyle::new(18.0, dark::ACCENT),
    );
}

/// Draw a clickable card component. Returns true if clicked.
pub fn card(x: f32, y: f32, w: f32, h: f32, selected: bool) -> bool {
    let hovered = is_hovered(x, y, w, h);
    let clicked = hovered && is_mouse_button_released(MouseButton::Left);

    let bg_color = if selected {
        Color::new(0.2, 0.25, 0.35, 0.9)
    } else if hovered {
        Color::new(0.18, 0.18, 0.25, 0.9)
    } else {
        Color::new(0.12, 0.12, 0.18, 0.9)
    };

    let border_color = if selected {
        dark::ACCENT
    } else {
        Color::new(0.5, 0.5, 0.5, 0.4)
    };

    let style = SurfaceStyle::new(bg_color).with_border(1.0, border_color);
    draw_surface(Rect::new(x, y, w, h), &style);

    clicked
}

/// Draw a full-screen semi-transparent overlay (for modals/screens)
pub fn full_screen_overlay(alpha: f32) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.05, 0.05, 0.1, alpha),
    );
}

/// Style configuration for tooltip rendering.
#[derive(Debug, Clone)]
pub struct TooltipStyle {
    pub background: Color,
    pub border: Color,
    pub text: Color,
    pub padding: f32,
    pub max_width: f32,
    pub font_size: f32,
    pub line_gap: f32,
}

impl Default for TooltipStyle {
    fn default() -> Self {
        Self {
            background: Color::new(0.04, 0.04, 0.06, 0.94),
            border: dark::ACCENT,
            text: dark::TEXT,
            padding: 8.0,
            max_width: 320.0,
            font_size: 16.0,
            line_gap: 3.0,
        }
    }
}

/// Draw a tooltip near an anchor point, clamped inside the current screen.
pub fn draw_tooltip(text: &str, anchor: Vec2) -> Rect {
    draw_tooltip_styled(text, anchor, &TooltipStyle::default(), None)
}

/// Draw a font-aware tooltip near an anchor point, clamped inside the current screen.
pub fn draw_tooltip_styled(
    text: &str,
    anchor: Vec2,
    style: &TooltipStyle,
    font: Option<&Font>,
) -> Rect {
    let font = font.or(default_ui_font());
    let font_size = effective_font_size(style.font_size);
    let line_gap = effective_line_gap(style.line_gap);
    let lines = wrap_text_ex(
        text,
        (style.max_width - style.padding * 2.0).max(1.0),
        font,
        style.font_size,
    );
    let content_width = lines
        .iter()
        .map(|line| measure_text(line, font, font_size_u16(font_size), 1.0).width)
        .fold(0.0, f32::max);
    let content_height =
        lines.len() as f32 * font_size + lines.len().saturating_sub(1) as f32 * line_gap;
    let width = (content_width + style.padding * 2.0).min(style.max_width);
    let height = content_height + style.padding * 2.0;

    let mut rect = Rect::new(anchor.x + 14.0, anchor.y + 14.0, width, height);
    if rect.x + rect.w > screen_width() {
        rect.x = (screen_width() - rect.w - 6.0).max(6.0);
    }
    if rect.y + rect.h > screen_height() {
        rect.y = (screen_height() - rect.h - 6.0).max(6.0);
    }

    draw_rectangle(rect.x, rect.y, rect.w, rect.h, style.background);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, style.border);

    let mut y = rect.y + style.padding + font_size;
    let text_style = TextStyle {
        font,
        font_size: style.font_size,
        color: style.text,
        line_gap: style.line_gap,
    };
    for line in &lines {
        draw_text_ex(line, rect.x + style.padding, y, text_style.params());
        y += font_size + line_gap;
    }

    rect
}

/// Draw a compact badge/chip.
pub fn draw_badge(rect: Rect, label: &str, fill: Color, text_color: Color) {
    let style = SurfaceStyle::new(fill).with_border(1.0, Color::new(1.0, 1.0, 1.0, 0.2));
    draw_surface(rect, &style);
    draw_text_centered_in_box(
        label,
        rect.x + 4.0,
        rect.y,
        rect.w - 8.0,
        rect.h,
        14.0,
        text_color,
    );
}

/// Draw a meter with optional centered label.
pub fn meter(rect: Rect, value: f32, max: f32, fill: Color, label: Option<&str>) {
    progress_bar(rect.x, rect.y, rect.w, rect.h, value, max, fill);
    if let Some(label) = label {
        // A full/bright fill (e.g. a green bar at 100%) leaves light text almost
        // unreadable, so stroke the label with a dark outline first — it keeps
        // contrast over both the bright fill and the dark empty track.
        let outline = Color::new(0.0, 0.0, 0.0, 0.75);
        let _stroke = super::bounds::Decorative::new();
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            draw_text_centered_in_box(
                label,
                rect.x + 4.0 + dx,
                rect.y + dy,
                rect.w - 8.0,
                rect.h,
                14.0,
                outline,
            );
        }
        drop(_stroke);
        draw_text_centered_in_box(
            label,
            rect.x + 4.0,
            rect.y,
            rect.w - 8.0,
            rect.h,
            14.0,
            dark::TEXT,
        );
    }
}
