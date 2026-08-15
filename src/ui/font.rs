//! UI font loading, and the text measurement, layout and drawing built on it.
//!
//! Value formatting used to live here too and now lives in `ui::format`: a
//! money string has nothing to do with a glyph, and carrying both took this
//! file past the 800-line limit.

use crate::colors::dark;
use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

const RAJDHANI_SEMIBOLD_BYTES: &[u8] = include_bytes!("../../assets/fonts/Rajdhani-SemiBold.ttf");

thread_local! {
    static DEFAULT_UI_FONT: RefCell<Option<&'static Font>> = const { RefCell::new(None) };
    static USE_MACROQUAD_DEFAULT_FONT: RefCell<bool> = const { RefCell::new(false) };
    static UI_TEXT_SCALE: RefCell<f32> = const { RefCell::new(1.0) };
    static MIN_UI_FONT_SIZE: RefCell<f32> = const { RefCell::new(1.0) };
    static TEXT_LAYOUT_CACHE: RefCell<HashMap<TextLayoutCacheKey, TextLayoutResult>> = RefCell::new(HashMap::new());
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextLayoutCacheKey {
    text: String,
    max_width: u32,
    max_height: u32,
    starting_font_size: u32,
    min_font_size: u32,
    line_gap: u32,
    text_scale: u32,
    font: usize,
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
    USE_MACROQUAD_DEFAULT_FONT.with(|enabled| *enabled.borrow_mut() = false);
}

/// Use Macroquad's built-in font for every toolkit text helper without lazy-loading Rajdhani.
///
/// This is useful for text-heavy applications that prefer Macroquad's stable shared atlas over
/// the bundled display face. Calling [`set_default_ui_font`] later selects that explicit font.
pub fn use_macroquad_default_ui_font() {
    USE_MACROQUAD_DEFAULT_FONT.with(|enabled| *enabled.borrow_mut() = true);
}

fn uses_macroquad_default_font() -> bool {
    USE_MACROQUAD_DEFAULT_FONT.with(|enabled| *enabled.borrow())
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
    if !uses_macroquad_default_font() && registered_default_ui_font().is_none() {
        set_builtin_rajdhani_semibold_ui_font()?;
    }
    Ok(())
}

/// Populate the custom UI font atlas before a text-heavy frame is batched.
///
/// Macroquad adds custom-font glyphs lazily. If an atlas grows midway through a
/// dense frame, draw calls recorded against the earlier texture layout can show
/// stale glyph cells. Games can call this once at startup with the font sizes
/// used by their UI to make those atlas changes happen before rendering.
pub fn prewarm_default_ui_font(sizes: &[u16]) -> Result<(), String> {
    ensure_default_ui_font()?;
    let Some(font) = registered_default_ui_font() else {
        return Ok(());
    };
    let mut characters = Font::latin_character_list();
    characters.extend(" -+/$|'?_<>;=~%#&@!…—–×⚙".chars());
    characters.sort_unstable();
    characters.dedup();
    for size in sizes.iter().copied().filter(|size| *size > 0) {
        font.populate_font_cache(&characters, size);
    }
    Ok(())
}

/// Populate only the characters a specific large text style will draw.
///
/// This avoids spending atlas space on a full alphabet when a 40–80px style is
/// reserved for formatted prices or clocks.
pub fn prewarm_default_ui_font_text(samples: &[(u16, &str)]) -> Result<(), String> {
    ensure_default_ui_font()?;
    let Some(font) = registered_default_ui_font() else {
        return Ok(());
    };
    for (size, text) in samples.iter().copied().filter(|(size, _)| *size > 0) {
        let mut characters: Vec<char> = text.chars().collect();
        characters.sort_unstable();
        characters.dedup();
        font.populate_font_cache(&characters, size);
    }
    Ok(())
}

/// Queue one off-screen draw for every populated UI-font size.
///
/// Call this after [`prewarm_default_ui_font`], then present the frame before
/// batching the first real screen. Macroquad can otherwise retain texture
/// coordinates from before a custom-font atlas finished growing, corrupting a
/// dense first frame even though every glyph is already cached.
pub fn draw_default_ui_font_atlas_warmup(sizes: &[u16]) {
    const WARMUP_TEXT: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz 0123456789 !@#$%^&*()[]{}.,:;'\"?/\\|_+-=<>—";
    for size in sizes.iter().copied().filter(|size| *size > 0) {
        draw_ui_text(WARMUP_TEXT, -2_000.0, -2_000.0, f32::from(size), WHITE);
    }
}

/// Queue targeted off-screen draws for styles prewarmed with
/// [`prewarm_default_ui_font_text`]. Present the frame once before real UI.
pub fn draw_default_ui_font_text_atlas_warmup(samples: &[(u16, &str)]) {
    for (size, text) in samples.iter().copied().filter(|(size, _)| *size > 0) {
        draw_ui_text(text, -2_000.0, -2_000.0, f32::from(size), WHITE);
    }
}

/// Return the registered default UI font, loading the bundled Rajdhani font if needed.
pub fn default_ui_font() -> Option<&'static Font> {
    if uses_macroquad_default_font() {
        return None;
    }
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
    // Pseudolocalisation (see `ui::pseudo`) happens here rather than at the call
    // sites, so a string that never reaches a text helper is visible by *not*
    // being marked — which is itself a finding.
    let swapped = super::pseudo::apply(text);
    let text = swapped.as_deref().unwrap_or(text);
    // Read before the params are handed over, since drawing consumes them.
    let (color, size) = (params.color, params.font_size as f32);
    let drawn = macroquad::prelude::draw_text_ex(text, x, y, params);
    // What it measured, against what the enclosing region had. Free unless an
    // audit is running (see `ui::bounds`).
    if super::bounds::auditing() {
        super::bounds::note(text, x, drawn.width);
        super::bounds::note_contrast(text, color, size);
        // The baseline sits at `y`, so the box rises above it. Cap height is
        // near 0.72em and descenders reach about 0.22 below, which is close
        // enough to catch a collision and loose enough not to invent one.
        super::bounds::note_extent(
            text,
            Rect::new(x, y - size * 0.72, drawn.width, size * 0.94),
        );
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
    pub(crate) macroquad_default: bool,
    pub font_size: f32,
    pub color: Color,
    pub line_gap: f32,
}

impl<'a> TextStyle<'a> {
    pub fn new(font_size: f32, color: Color) -> Self {
        Self {
            font: None,
            macroquad_default: false,
            font_size,
            color,
            line_gap: 4.0,
        }
    }

    pub fn with_font(mut self, font: &'a Font) -> Self {
        self.font = Some(font);
        self.macroquad_default = false;
        self
    }

    /// Draw with Macroquad's built-in font instead of the registered toolkit font.
    pub fn with_macroquad_font(mut self) -> Self {
        self.font = None;
        self.macroquad_default = true;
        self
    }

    pub fn with_line_gap(mut self, line_gap: f32) -> Self {
        self.line_gap = line_gap;
        self
    }

    pub fn resolved_font(&self) -> Option<&'a Font> {
        if self.macroquad_default {
            None
        } else {
            self.font.or(default_ui_font())
        }
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
    // The single place a block of text is expanded (see `ui::pseudo`). Before
    // wrapping, because the whole question is whether the longer text still
    // fits — and *only* here, since `fit_text_to_box_ex` reaches this function
    // and expanding in both produced `[[doubly marked]]` text.
    let swapped = super::pseudo::apply(text);
    let text = swapped.as_deref().unwrap_or(text);
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
    let key = TextLayoutCacheKey {
        text: text.to_owned(),
        max_width: max_width.to_bits(),
        max_height: max_height.to_bits(),
        starting_font_size: style.font_size.to_bits(),
        min_font_size: min_font_size.to_bits(),
        line_gap: style.line_gap.to_bits(),
        text_scale: ui_text_scale().to_bits(),
        font: style
            .resolved_font()
            .map_or(0, |font| font as *const Font as usize),
    };
    if let Some(layout) = TEXT_LAYOUT_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return layout;
    }
    let mut font_size = style.font_size;
    let line_gap = style.effective_line_gap();

    while font_size >= min_font_size {
        let lines = wrap_text_ex(text, max_width, style.font, font_size);
        let draw_font_size = effective_font_size(font_size);
        let total_height =
            lines.len() as f32 * draw_font_size + (lines.len().saturating_sub(1) as f32 * line_gap);
        if total_height <= max_height {
            return cache_text_layout(
                key,
                TextLayoutResult {
                    lines,
                    font_size,
                    truncated: false,
                },
            );
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

    cache_text_layout(
        key,
        TextLayoutResult {
            lines,
            font_size,
            truncated,
        },
    )
}

fn cache_text_layout(key: TextLayoutCacheKey, layout: TextLayoutResult) -> TextLayoutResult {
    TEXT_LAYOUT_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= 1_024 {
            cache.clear();
        }
        cache.insert(key, layout.clone());
    });
    layout
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
    // Expand the block once, then hold it: the per-line draws below go through
    // the same helpers (see `ui::pseudo::Once`).
    let swapped = super::pseudo::apply(text);
    let text = swapped.as_deref().unwrap_or(text);
    let _once = super::pseudo::Once::new();

    let layout = fit_text_to_box_ex(text, w, h, style, min_font_size);
    let draw_font_size = effective_font_size(layout.font_size);
    let line_gap = style.effective_line_gap();
    let mut line_y = y + draw_font_size;
    for line in &layout.lines {
        let params = TextStyle {
            font_size: layout.font_size,
            ..style
        }
        .params();
        let drawn = draw_text_ex(line, x, line_y, params);
        // These lines go straight to macroquad rather than through
        // `draw_ui_text_ex`, so they have to report themselves or a whole class
        // of the game's prose would be outside the audit (§5.37, §5.40).
        if super::bounds::auditing() {
            super::bounds::note(line, x, drawn.width);
            super::bounds::note_contrast(line, style.color, effective_font_size(layout.font_size));
            super::bounds::note_extent(
                line,
                Rect::new(
                    x,
                    line_y - draw_font_size * 0.72,
                    drawn.width,
                    draw_font_size * 0.94,
                ),
            );
        }
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
    let swapped = super::pseudo::apply(text);
    let text = swapped.as_deref().unwrap_or(text);
    let _once = super::pseudo::Once::new();
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
        if super::bounds::auditing() {
            super::bounds::note(line, line_x, line_width);
            super::bounds::note_contrast(line, style.color, draw_font_size);
            super::bounds::note_extent(
                line,
                Rect::new(
                    line_x,
                    line_y - draw_font_size * 0.72,
                    line_width,
                    draw_font_size * 0.94,
                ),
            );
        }
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
    let swapped = super::pseudo::apply(text);
    let text = swapped.as_deref().unwrap_or(text);
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
