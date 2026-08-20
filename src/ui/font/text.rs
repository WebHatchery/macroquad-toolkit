//! Text layout, fitting, and rendering built on the shared UI font state.

use super::*;

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
    truncate_text_to_width_with_font(text, max_width, font.or(default_ui_font()), font_size)
}

fn truncate_text_to_width_with_font(
    text: &str,
    max_width: f32,
    font: Option<&Font>,
    font_size: f32,
) -> String {
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

pub fn wrap_text_ex(
    text: &str,
    max_width: f32,
    font: Option<&Font>,
    font_size: f32,
) -> Vec<String> {
    wrap_text_with_font(text, max_width, font.or(default_ui_font()), font_size)
}

fn wrap_text_with_font(
    text: &str,
    max_width: f32,
    font: Option<&Font>,
    font_size: f32,
) -> Vec<String> {
    // The single place a block of text is expanded (see `ui::pseudo`). Before
    // wrapping, because the whole question is whether the longer text still
    // fits — and *only* here, since `fit_text_to_box_ex` reaches this function
    // and expanding in both produced `[[doubly marked]]` text.
    let swapped = crate::ui::pseudo::apply(text);
    let text = swapped.as_deref().unwrap_or(text);
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
        let lines = wrap_text_with_font(text, max_width, style.resolved_font(), font_size);
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
    let mut lines = wrap_text_with_font(text, max_width, style.resolved_font(), font_size);
    let truncated = lines.len() > max_lines;
    lines.truncate(max_lines);
    if let Some(last_line) = lines.last_mut() {
        *last_line = truncate_text_to_width_with_font(
            last_line,
            max_width,
            style.resolved_font(),
            font_size,
        );
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
    let swapped = crate::ui::pseudo::apply(text);
    let text = swapped.as_deref().unwrap_or(text);
    let _once = crate::ui::pseudo::Once::new();

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
        if crate::ui::bounds::auditing() {
            crate::ui::bounds::note(line, x, drawn.width);
            crate::ui::bounds::note_contrast(
                line,
                style.color,
                effective_font_size(layout.font_size),
            );
            crate::ui::bounds::note_extent(
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
    let swapped = crate::ui::pseudo::apply(text);
    let text = swapped.as_deref().unwrap_or(text);
    let _once = crate::ui::pseudo::Once::new();
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
        if crate::ui::bounds::auditing() {
            crate::ui::bounds::note(line, line_x, line_width);
            crate::ui::bounds::note_contrast(line, style.color, draw_font_size);
            crate::ui::bounds::note_extent(
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
    let swapped = crate::ui::pseudo::apply(text);
    let text = swapped.as_deref().unwrap_or(text);
    let width = measure_text(
        text,
        style.resolved_font(),
        font_size_u16(style.effective_font_size()),
        1.0,
    )
    .width;
    let left = right_x - width;
    if crate::ui::bounds::auditing() {
        // Right-aligned text runs off the *left*, so it is measured from where
        // it actually starts rather than from where it was anchored.
        crate::ui::bounds::note(text, left, width);
        if let Some(region) = crate::ui::bounds::current() {
            if left < region.x - 2.0 {
                crate::ui::bounds::note(text, region.x, region.right() - left);
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
