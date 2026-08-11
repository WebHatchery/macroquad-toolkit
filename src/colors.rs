//! Color palettes and color manipulation helpers for consistent game UI theming

use macroquad::prelude::Color;

/// Returns the color with its alpha replaced by `alpha`.
pub fn with_alpha(color: Color, alpha: f32) -> Color {
    Color::new(color.r, color.g, color.b, alpha)
}

/// Returns the color with its alpha multiplied by `factor` (for fading an already-translucent color).
pub fn multiply_alpha(mut color: Color, factor: f32) -> Color {
    color.a *= factor;
    color
}

/// Additively brightens each RGB channel by `amount` (clamped to 1.0). Alpha is preserved.
pub fn lighten(color: Color, amount: f32) -> Color {
    Color::new(
        (color.r + amount).clamp(0.0, 1.0),
        (color.g + amount).clamp(0.0, 1.0),
        (color.b + amount).clamp(0.0, 1.0),
        color.a,
    )
}

/// Additively darkens each RGB channel by `amount` (clamped to 0.0). Alpha is preserved.
pub fn darken(color: Color, amount: f32) -> Color {
    lighten(color, -amount)
}

/// Blends the color toward black by `amount` (`0.0` = unchanged, `1.0` = black).
/// Multiplicative shading — unlike [`darken`], which subtracts a fixed amount
/// per channel. Alpha is preserved.
pub fn shade(color: Color, amount: f32) -> Color {
    mix(color, Color::new(0.0, 0.0, 0.0, color.a), amount)
}

/// Blends the color toward white by `amount` (`0.0` = unchanged, `1.0` = white).
/// Multiplicative tinting — unlike [`lighten`], which adds a fixed amount per
/// channel. Alpha is preserved.
pub fn tint(color: Color, amount: f32) -> Color {
    mix(color, Color::new(1.0, 1.0, 1.0, color.a), amount)
}

/// Multiplies each RGB channel by `factor` (clamped to `[0, 1]`). Alpha is preserved.
pub fn scale_rgb(color: Color, factor: f32) -> Color {
    Color::new(
        (color.r * factor).clamp(0.0, 1.0),
        (color.g * factor).clamp(0.0, 1.0),
        (color.b * factor).clamp(0.0, 1.0),
        color.a,
    )
}

/// Component-wise linear interpolation between two colors (including alpha).
/// `t` is clamped to `[0, 1]`.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Alias for [`mix`], matching the common `lerp_color(a, b, t)` naming.
pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    mix(a, b, t)
}

/// Converts RGB (each `[0, 1]`) to HSV: hue in degrees `[0, 360)`, saturation and value `[0, 1]`.
pub fn rgb_to_hsv(color: Color) -> (f32, f32, f32) {
    let max = color.r.max(color.g).max(color.b);
    let min = color.r.min(color.g).min(color.b);
    let delta = max - min;

    let hue = if delta <= f32::EPSILON {
        0.0
    } else if max == color.r {
        60.0 * (((color.g - color.b) / delta).rem_euclid(6.0))
    } else if max == color.g {
        60.0 * ((color.b - color.r) / delta + 2.0)
    } else {
        60.0 * ((color.r - color.g) / delta + 4.0)
    };

    let saturation = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };
    (hue, saturation, max)
}

/// Converts HSV (hue in degrees, saturation/value `[0, 1]`) to an opaque RGB color.
pub fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> Color {
    let hue = hue.rem_euclid(360.0);
    let saturation = saturation.clamp(0.0, 1.0);
    let value = value.clamp(0.0, 1.0);

    let c = value * saturation;
    let x = c * (1.0 - ((hue / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = value - c;

    let (r, g, b) = match (hue / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::new(r + m, g + m, b + m, 1.0)
}

/// Rotates the hue of a color by `degrees`, preserving saturation, value, and alpha.
pub fn shift_hue(color: Color, degrees: f32) -> Color {
    let (h, s, v) = rgb_to_hsv(color);
    let mut shifted = hsv_to_rgb(h + degrees, s, v);
    shifted.a = color.a;
    shifted
}

/// Dark theme color palette - suitable for most game UIs
pub mod dark {
    use macroquad::prelude::Color;

    pub const BACKGROUND: Color = Color::new(0.12, 0.12, 0.14, 1.0);
    pub const PANEL: Color = Color::new(0.18, 0.18, 0.22, 1.0);
    pub const PANEL_HEADER: Color = Color::new(0.22, 0.22, 0.28, 1.0);

    pub const TEXT: Color = Color::new(0.9, 0.9, 0.9, 1.0);
    pub const TEXT_BRIGHT: Color = Color::new(1.0, 1.0, 1.0, 1.0);
    pub const TEXT_DIM: Color = Color::new(0.6, 0.6, 0.6, 1.0);

    pub const ACCENT: Color = Color::new(0.3, 0.6, 0.9, 1.0);
    pub const POSITIVE: Color = Color::new(0.3, 0.8, 0.4, 1.0);
    pub const WARNING: Color = Color::new(0.9, 0.7, 0.2, 1.0);
    pub const NEGATIVE: Color = Color::new(0.9, 0.3, 0.3, 1.0);

    pub const HOVERED: Color = Color::new(0.3, 0.4, 0.55, 1.0);
}

/// Rarity color palette - for items, equipment, loot in RPG-style games
pub mod rarity {
    use macroquad::prelude::Color;

    pub const COMMON: Color = Color::new(0.6, 0.6, 0.6, 1.0);
    pub const UNCOMMON: Color = Color::new(0.3, 0.7, 0.3, 1.0);
    pub const RARE: Color = Color::new(0.3, 0.5, 0.9, 1.0);
    pub const EPIC: Color = Color::new(0.6, 0.3, 0.9, 1.0);
    pub const LEGENDARY: Color = Color::new(0.9, 0.6, 0.2, 1.0);
}

#[cfg(test)]
mod tests;
