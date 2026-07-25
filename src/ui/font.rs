//! UI font loading, text measurement/layout/drawing, and value formatting
//! (money, idle-magnitude amounts/rates, clocks, and string case helpers).

use crate::colors::dark;
use macroquad::prelude::*;
use std::cell::RefCell;

const RAJDHANI_SEMIBOLD_BYTES: &[u8] = include_bytes!("../../assets/fonts/Rajdhani-SemiBold.ttf");

thread_local! {
    static DEFAULT_UI_FONT: RefCell<Option<&'static Font>> = const { RefCell::new(None) };
    static UI_TEXT_SCALE: RefCell<f32> = const { RefCell::new(1.0) };
    static MIN_UI_FONT_SIZE: RefCell<f32> = const { RefCell::new(1.0) };
}

pub(crate) fn font_size_u16(font_size: f32) -> u16 {
    font_size.round().clamp(1.0, u16::MAX as f32) as u16
}

fn ui_text_scale() -> f32 {
    UI_TEXT_SCALE.with(|stored| stored.borrow().clamp(0.25, 4.0))
}

fn min_ui_font_size() -> f32 {
    MIN_UI_FONT_SIZE.with(|stored| stored.borrow().clamp(1.0, 96.0))
}

pub(crate) fn effective_font_size(font_size: f32) -> f32 {
    font_size.max(min_ui_font_size()) * ui_text_scale()
}

pub(crate) fn effective_line_gap(line_gap: f32) -> f32 {
    line_gap * ui_text_scale()
}

/// Register a default font used by toolkit text helpers when no explicit font is supplied.
///
/// This is intended to be called once during game startup. The font is retained for the
/// process lifetime so `TextStyle::params()` can safely return Macroquad's borrowed font params.
pub fn set_default_ui_font(font: Font) {
    let font = Box::leak(Box::new(font));
    DEFAULT_UI_FONT.with(|stored| {
        *stored.borrow_mut() = Some(font);
    });
}

/// Decode and register a default font from embedded TTF/OTF bytes.
pub fn set_default_ui_font_from_bytes(bytes: &'static [u8]) -> Result<(), String> {
    let font = load_ttf_font_from_bytes(bytes)
        .map_err(|err| format!("failed to load default UI font: {err:?}"))?;
    set_default_ui_font(font);
    Ok(())
}

/// Return the bundled Rajdhani SemiBold font bytes.
pub fn builtin_rajdhani_semibold_font_bytes() -> &'static [u8] {
    RAJDHANI_SEMIBOLD_BYTES
}

/// Decode the bundled Rajdhani SemiBold font.
pub fn load_builtin_rajdhani_semibold_font() -> Result<Font, String> {
    load_ttf_font_from_bytes(RAJDHANI_SEMIBOLD_BYTES)
        .map_err(|err| format!("failed to load bundled Rajdhani SemiBold font: {err:?}"))
}

/// Register the bundled Rajdhani SemiBold font as the default toolkit UI font.
pub fn set_builtin_rajdhani_semibold_ui_font() -> Result<(), String> {
    set_default_ui_font(load_builtin_rajdhani_semibold_font()?);
    Ok(())
}

fn registered_default_ui_font() -> Option<&'static Font> {
    DEFAULT_UI_FONT.with(|stored| *stored.borrow())
}

/// Ensure a default toolkit UI font is available.
///
/// Games can call this during startup. Toolkit text helpers also call it lazily so
/// `TextStyle` users get the shared Rajdhani font without duplicating font assets.
pub fn ensure_default_ui_font() -> Result<(), String> {
    if registered_default_ui_font().is_none() {
        set_builtin_rajdhani_semibold_ui_font()?;
    }
    Ok(())
}

/// Return the registered default UI font, loading the bundled Rajdhani font if needed.
pub fn default_ui_font() -> Option<&'static Font> {
    let _ = ensure_default_ui_font();
    registered_default_ui_font()
}

/// Build Macroquad text params using the shared default toolkit font.
pub fn ui_text_params(font_size: f32, color: Color) -> TextParams<'static> {
    TextStyle::new(font_size, color).params()
}

/// Measure text with an explicit font when supplied, otherwise the shared default UI font.
pub fn measure_ui_text(
    text: &str,
    font: Option<&Font>,
    font_size: u16,
    font_scale: f32,
) -> TextDimensions {
    measure_text(text, font.or(default_ui_font()), font_size, font_scale)
}

/// Draw text using the shared default toolkit font.
pub fn draw_ui_text(text: &str, x: f32, y: f32, font_size: f32, color: Color) -> TextDimensions {
    draw_ui_text_ex(text, x, y, ui_text_params(font_size, color))
}

/// Draw text with Macroquad text params, filling in the shared default font when omitted.
pub fn draw_ui_text_ex<'a>(
    text: &str,
    x: f32,
    y: f32,
    mut params: TextParams<'a>,
) -> TextDimensions {
    if params.font.is_none() {
        params.font = default_ui_font();
    }
    let drawn = macroquad::prelude::draw_text_ex(text, x, y, params);
    // What it measured, against what the enclosing region had. Free unless an
    // audit is running (see `ui::bounds`).
    if super::bounds::auditing() {
        super::bounds::note(text, x, drawn.width);
    }
    drawn
}

/// Set a global multiplier used by toolkit text helpers.
///
/// This is useful for dense fixed-resolution UIs when the canvas is being displayed below
/// its logical resolution. The scale affects drawing and text measurement consistently.
pub fn set_ui_text_scale(scale: f32) {
    UI_TEXT_SCALE.with(|stored| {
        *stored.borrow_mut() = scale.clamp(0.25, 4.0);
    });
}

/// Set the minimum logical font size used by toolkit text helpers.
pub fn set_min_ui_font_size(font_size: f32) {
    MIN_UI_FONT_SIZE.with(|stored| {
        *stored.borrow_mut() = font_size.clamp(1.0, 96.0);
    });
}

/// Scale text up when a fixed logical UI is displayed below its design resolution.
pub fn set_ui_text_scale_for_screen(
    logical_width: f32,
    logical_height: f32,
    max_scale: f32,
) -> f32 {
    let pixel_scale = (screen_width() / logical_width.max(1.0))
        .min(screen_height() / logical_height.max(1.0))
        .max(0.01);
    let scale = (1.0 / pixel_scale).clamp(1.0, max_scale.max(1.0));
    set_ui_text_scale(scale);
    scale
}

#[derive(Debug, Clone)]
pub struct TextLayoutResult {
    pub lines: Vec<String>,
    pub font_size: f32,
    pub truncated: bool,
}

/// Font-aware text drawing configuration.
#[derive(Debug, Clone, Copy)]
pub struct TextStyle<'a> {
    pub font: Option<&'a Font>,
    pub font_size: f32,
    pub color: Color,
    pub line_gap: f32,
}

impl<'a> TextStyle<'a> {
    pub fn new(font_size: f32, color: Color) -> Self {
        Self {
            font: None,
            font_size,
            color,
            line_gap: 4.0,
        }
    }

    pub fn with_font(mut self, font: &'a Font) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_line_gap(mut self, line_gap: f32) -> Self {
        self.line_gap = line_gap;
        self
    }

    pub fn resolved_font(&self) -> Option<&'a Font> {
        self.font.or(default_ui_font())
    }

    pub fn effective_font_size(&self) -> f32 {
        effective_font_size(self.font_size)
    }

    pub fn effective_line_gap(&self) -> f32 {
        effective_line_gap(self.line_gap)
    }

    pub fn params(&self) -> TextParams<'a> {
        TextParams {
            font: self.resolved_font(),
            font_size: font_size_u16(self.effective_font_size()),
            color: self.color,
            ..Default::default()
        }
    }
}

impl Default for TextStyle<'_> {
    fn default() -> Self {
        Self::new(20.0, dark::TEXT)
    }
}

/// Measure text using a [`TextStyle`].
pub fn measure_text_size(text: &str, style: TextStyle<'_>) -> TextDimensions {
    measure_text(
        text,
        style.resolved_font(),
        font_size_u16(style.effective_font_size()),
        1.0,
    )
}

/// Format an integer currency value with comma separators.
pub fn format_money(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let mut digits = value.abs().to_string();
    let mut result = String::new();

    while digits.len() > 3 {
        let tail = digits.split_off(digits.len() - 3);
        if result.is_empty() {
            result = tail;
        } else {
            result = format!("{tail},{result}");
        }
    }

    if result.is_empty() {
        format!("{sign}${digits}")
    } else {
        format!("{sign}${digits},{result}")
    }
}

/// Format an integer currency value compactly for dense UI.
pub fn format_compact_money(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let abs = value.abs();
    if abs >= 1_000_000 {
        format!("{sign}${:.1}m", abs as f32 / 1_000_000.0)
    } else if abs >= 1_000 {
        format!("{sign}${}k", abs / 1_000)
    } else {
        format!("{sign}${abs}")
    }
}

/// Magnitude suffixes for [`format_amount`] / [`format_rate`], each step a
/// thousandfold: K (1e3), M (1e6), B (1e9), T (1e12), then Qa/Qi/Sx/Sp/Oc/No
/// up to 1e30. Values beyond the table fall back to scientific notation.
const AMOUNT_SUFFIXES: [&str; 11] = ["", "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No"];

/// Formats a large `f64` quantity with idle/incremental-genre magnitude
/// suffixes: integers below 1000, two decimals with a suffix up to `No`
/// (1e30), scientific notation beyond. Handles negatives, and degrades
/// `NaN`/infinity to `∞` rather than printing junk.
///
/// Unlike [`format_compact_money`], which takes an `i64` and saturates around
/// 9.2e18, this covers the full `f64` range that idle games routinely reach.
///
/// ```
/// # use macroquad_toolkit::ui::format_amount;
/// assert_eq!(format_amount(999.0), "999");
/// assert_eq!(format_amount(1_500.0), "1.50K");
/// assert_eq!(format_amount(2_340_000.0), "2.34M");
/// assert_eq!(format_amount(1e30), "1.00No");
/// ```
pub fn format_amount(value: f64) -> String {
    if !value.is_finite() {
        return "∞".to_owned();
    }
    if value < 0.0 {
        return format!("-{}", format_amount(-value));
    }
    if value < 1000.0 {
        return format!("{}", value.floor() as i64);
    }

    let tier = (value.log10() / 3.0).floor() as usize;
    if tier < AMOUNT_SUFFIXES.len() {
        let scaled = value / 1000f64.powi(tier as i32);
        format!("{:.2}{}", scaled, AMOUNT_SUFFIXES[tier])
    } else {
        format!("{:.2e}", value)
    }
}

/// Formats a per-second rate: one decimal below 1000, then the same magnitude
/// suffixes as [`format_amount`].
///
/// ```
/// # use macroquad_toolkit::ui::format_rate;
/// assert_eq!(format_rate(2.5), "2.5");
/// assert_eq!(format_rate(12_500.0), "12.50K");
/// ```
pub fn format_rate(value: f64) -> String {
    if value < 1000.0 {
        format!("{:.1}", value)
    } else {
        format_amount(value)
    }
}

/// Format elapsed seconds as `MM:SS` (e.g. `07:42`). Minutes keep growing
/// past an hour (`75:03`).
pub fn format_mmss(total_seconds: f32) -> String {
    let total = total_seconds.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// Format elapsed seconds as `H:MM:SS` once an hour is reached, otherwise
/// `MM:SS`.
pub fn format_hmmss(total_seconds: f32) -> String {
    let total = total_seconds.max(0.0) as u64;
    let hours = total / 3600;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, (total % 3600) / 60, total % 60)
    } else {
        format_mmss(total_seconds)
    }
}

/// Format an in-game clock as `HH:MM` (e.g. `08:30`). Hours wrap at 24.
pub fn format_clock(hour: u32, minute: u32) -> String {
    format!("{:02}:{:02}", hour % 24, minute % 60)
}

/// Capitalize the first character of a string
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars().collect::<Vec<_>>();
    if let Some(c) = chars.get_mut(0) {
        *c = c.to_ascii_uppercase();
    }
    chars.into_iter().collect()
}

/// Format a type_key (snake_case) into a display name (Title Case)
/// e.g., "health_potion" -> "Health Potion"
pub fn display_name(type_key: &str) -> String {
    type_key
        .split('_')
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn truncate_text_to_width(text: &str, max_width: f32, font_size: f32) -> String {
    truncate_text_to_width_ex(text, max_width, None, font_size)
}

pub fn truncate_text_to_width_ex(
    text: &str,
    max_width: f32,
    font: Option<&Font>,
    font_size: f32,
) -> String {
    let font = font.or(default_ui_font());
    let font_size = effective_font_size(font_size);
    if measure_text(text, font, font_size_u16(font_size), 1.0).width <= max_width {
        return text.to_owned();
    }

    let ellipsis = "...";
    let mut result = String::new();
    for ch in text.chars() {
        let candidate = format!("{result}{ch}{ellipsis}");
        if measure_text(&candidate, font, font_size_u16(font_size), 1.0).width > max_width {
            break;
        }
        result.push(ch);
    }

    if result.is_empty() {
        ellipsis.to_owned()
    } else {
        format!("{result}{ellipsis}")
    }
}

pub fn wrap_text(text: &str, max_width: f32, font_size: f32) -> Vec<String> {
    wrap_text_ex(text, max_width, None, font_size)
}

pub fn wrap_text_ex(
    text: &str,
    max_width: f32,
    font: Option<&Font>,
    font_size: f32,
) -> Vec<String> {
    let font = font.or(default_ui_font());
    let font_size = effective_font_size(font_size);
    let mut wrapped = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.trim().is_empty() {
            wrapped.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };

            if measure_text(&candidate, font, font_size_u16(font_size), 1.0).width <= max_width {
                current = candidate;
                continue;
            }

            if !current.is_empty() {
                wrapped.push(std::mem::take(&mut current));
            }

            if measure_text(word, font, font_size_u16(font_size), 1.0).width <= max_width {
                current = word.to_owned();
                continue;
            }

            let mut chunk = String::new();
            for ch in word.chars() {
                let candidate = format!("{chunk}{ch}");
                if measure_text(&candidate, font, font_size_u16(font_size), 1.0).width > max_width
                    && !chunk.is_empty()
                {
                    wrapped.push(chunk);
                    chunk = ch.to_string();
                } else {
                    chunk.push(ch);
                }
            }
            current = chunk;
        }

        if !current.is_empty() {
            wrapped.push(current);
        }
    }

    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

pub fn fit_text_to_box(
    text: &str,
    max_width: f32,
    max_height: f32,
    starting_font_size: f32,
    line_gap: f32,
    min_font_size: f32,
) -> TextLayoutResult {
    fit_text_to_box_ex(
        text,
        max_width,
        max_height,
        TextStyle::new(starting_font_size, dark::TEXT).with_line_gap(line_gap),
        min_font_size,
    )
}

pub fn fit_text_to_box_ex(
    text: &str,
    max_width: f32,
    max_height: f32,
    style: TextStyle<'_>,
    min_font_size: f32,
) -> TextLayoutResult {
    let mut font_size = style.font_size;
    let line_gap = style.effective_line_gap();

    while font_size >= min_font_size {
        let lines = wrap_text_ex(text, max_width, style.font, font_size);
        let draw_font_size = effective_font_size(font_size);
        let total_height =
            lines.len() as f32 * draw_font_size + (lines.len().saturating_sub(1) as f32 * line_gap);
        if total_height <= max_height {
            return TextLayoutResult {
                lines,
                font_size,
                truncated: false,
            };
        }
        font_size -= 1.0;
    }

    let font_size = min_font_size.max(1.0);
    let draw_font_size = effective_font_size(font_size);
    let max_lines = ((max_height + line_gap) / (draw_font_size + line_gap))
        .floor()
        .max(1.0) as usize;
    let mut lines = wrap_text_ex(text, max_width, style.font, font_size);
    let truncated = lines.len() > max_lines;
    lines.truncate(max_lines);
    if let Some(last_line) = lines.last_mut() {
        *last_line = truncate_text_to_width_ex(last_line, max_width, style.font, font_size);
    }

    TextLayoutResult {
        lines,
        font_size,
        truncated,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_block(
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    starting_font_size: f32,
    line_gap: f32,
    color: Color,
) -> TextLayoutResult {
    draw_text_block_ex(
        text,
        x,
        y,
        w,
        h,
        TextStyle::new(starting_font_size, color).with_line_gap(line_gap),
        12.0,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_block_ex(
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    style: TextStyle<'_>,
    min_font_size: f32,
) -> TextLayoutResult {
    let layout = fit_text_to_box_ex(text, w, h, style, min_font_size);
    let draw_font_size = effective_font_size(layout.font_size);
    let line_gap = style.effective_line_gap();
    let mut line_y = y + draw_font_size;
    for line in &layout.lines {
        draw_text_ex(
            line,
            x,
            line_y,
            TextStyle {
                font_size: layout.font_size,
                ..style
            }
            .params(),
        );
        line_y += draw_font_size + line_gap;
    }
    layout
}

pub fn draw_text_centered_in_box(
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    starting_font_size: f32,
    color: Color,
) -> TextLayoutResult {
    draw_text_centered_in_box_ex(text, x, y, w, h, TextStyle::new(starting_font_size, color))
}

pub fn draw_text_centered_in_box_ex(
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    style: TextStyle<'_>,
) -> TextLayoutResult {
    let layout = fit_text_to_box_ex(text, w, h, style, 10.0);
    let draw_font_size = effective_font_size(layout.font_size);
    let line_gap = style.effective_line_gap();
    let total_height = layout.lines.len() as f32 * draw_font_size
        + (layout.lines.len().saturating_sub(1) as f32 * line_gap);
    let mut line_y = y + ((h - total_height) * 0.5) + draw_font_size;

    for line in &layout.lines {
        let line_width = measure_text(
            line,
            style.resolved_font(),
            font_size_u16(draw_font_size),
            1.0,
        )
        .width;
        let line_x = x + (w - line_width) * 0.5;
        draw_text_ex(
            line,
            line_x,
            line_y,
            TextStyle {
                font_size: layout.font_size,
                ..style
            }
            .params(),
        );
        line_y += draw_font_size + line_gap;
    }

    layout
}

/// Draw text centered around `center_x` at the supplied baseline.
pub fn draw_text_centered(text: &str, center_x: f32, baseline_y: f32, style: TextStyle<'_>) {
    let dimensions = measure_text_size(text, style);
    draw_text_ex(
        text,
        center_x - dimensions.width * 0.5,
        baseline_y,
        style.params(),
    );
}

pub fn draw_text_right(text: &str, right_x: f32, baseline_y: f32, style: TextStyle<'_>) {
    let width = measure_text(
        text,
        style.resolved_font(),
        font_size_u16(style.effective_font_size()),
        1.0,
    )
    .width;
    let left = right_x - width;
    if super::bounds::auditing() {
        // Right-aligned text runs off the *left*, so it is measured from where
        // it actually starts rather than from where it was anchored.
        super::bounds::note(text, left, width);
        if let Some(region) = super::bounds::current() {
            if left < region.x - 2.0 {
                super::bounds::note(text, region.x, region.right() - left);
            }
        }
    }
    draw_text_ex(text, left, baseline_y, style.params());
}

pub fn draw_text_shadow(
    text: &str,
    x: f32,
    y: f32,
    style: TextStyle<'_>,
    shadow_offset: Vec2,
    shadow_color: Color,
) {
    draw_text_ex(
        text,
        x + shadow_offset.x,
        y + shadow_offset.y,
        TextStyle {
            color: shadow_color,
            ..style
        }
        .params(),
    );
    draw_text_ex(text, x, y, style.params());
}

/// Eight compass offsets used to build a symmetric text halo.
const GLOW_DIRS: [(f32, f32); 8] = [
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (0.7, 0.7),
    (-0.7, 0.7),
    (0.7, -0.7),
    (-0.7, -0.7),
];

/// Draw `text` with a soft phosphor bloom: several dim, offset copies of the
/// text (in `style.color` at `glow_alpha`) fanned out to `glow_radius`, with a
/// crisp foreground copy on top. A cheap CRT-style halo for bright headings —
/// keep `glow_alpha` low and reserve it for large text so body copy stays
/// legible.
pub fn draw_text_glow(
    text: &str,
    x: f32,
    y: f32,
    style: TextStyle<'_>,
    glow_alpha: f32,
    glow_radius: f32,
) {
    let glow = TextStyle {
        color: Color {
            a: glow_alpha,
            ..style.color
        },
        ..style
    };
    for radius in [glow_radius, glow_radius * 0.5] {
        for (dx, dy) in GLOW_DIRS {
            draw_text_ex(text, x + dx * radius, y + dy * radius, glow.params());
        }
    }
    draw_text_ex(text, x, y, style.params());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_money_with_commas() {
        assert_eq!(format_money(0), "$0");
        assert_eq!(format_money(1_234), "$1,234");
        assert_eq!(format_money(-1_234_567), "-$1,234,567");
    }

    #[test]
    fn formats_compact_money() {
        assert_eq!(format_compact_money(999), "$999");
        assert_eq!(format_compact_money(12_000), "$12k");
        assert_eq!(format_compact_money(1_240_000), "$1.2m");
    }

    #[test]
    fn formats_amount_across_tiers() {
        assert_eq!(format_amount(0.0), "0");
        assert_eq!(format_amount(999.0), "999");
        assert_eq!(format_amount(1_500.0), "1.50K");
        assert_eq!(format_amount(2_340_000.0), "2.34M");
        assert_eq!(format_amount(1e9), "1.00B");
        assert_eq!(format_amount(1e12), "1.00T");
        assert_eq!(format_amount(1e30), "1.00No");
        assert_eq!(format_amount(-1_500.0), "-1.50K");
    }

    #[test]
    fn amount_falls_back_to_scientific_and_survives_extremes() {
        assert_eq!(format_amount(1e36), "1.00e36");
        assert_eq!(format_amount(1e300), "1.00e300");
        // f64::MAX (~1.8e308) is still finite and formats.
        assert!(format_amount(f64::MAX).contains("e308"));
        // Non-finite inputs degrade gracefully rather than printing junk.
        assert_eq!(format_amount(f64::INFINITY), "∞");
        assert_eq!(format_amount(f64::NAN), "∞");
    }

    #[test]
    fn formats_rate_with_one_decimal_then_suffixes() {
        assert_eq!(format_rate(2.5), "2.5");
        assert_eq!(format_rate(12_500.0), "12.50K");
    }

    #[test]
    fn formats_durations() {
        assert_eq!(format_mmss(0.0), "00:00");
        assert_eq!(format_mmss(462.9), "07:42");
        assert_eq!(format_mmss(4503.0), "75:03");
        assert_eq!(format_mmss(-5.0), "00:00");
        assert_eq!(format_hmmss(462.9), "07:42");
        assert_eq!(format_hmmss(4503.0), "1:15:03");
    }

    #[test]
    fn formats_clock() {
        assert_eq!(format_clock(8, 30), "08:30");
        assert_eq!(format_clock(25, 61), "01:01");
    }
}
