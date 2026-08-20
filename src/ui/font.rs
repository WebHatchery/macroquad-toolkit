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

mod text;

pub use text::*;

/// Wrap text using the shared default UI font.
pub fn wrap_text(text: &str, max_width: f32, font_size: f32) -> Vec<String> {
    text::wrap_text_ex(text, max_width, None, font_size)
}

#[cfg(test)]
mod tests;
