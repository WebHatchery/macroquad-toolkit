//! Whether text can actually be read off what it is drawn on.
//!
//! # The measurement nobody makes
//!
//! Picking UI colours is done by eye, on a good monitor, by someone who chose
//! them and therefore knows what they say. None of that is true of the player.
//! Dim grey on near-black looks tasteful in the editor and disappears on a
//! laptop screen in daylight, and the author is the last person able to notice,
//! because they can read it from memory.
//!
//! Contrast is one of the few things about a visual design that is genuinely
//! objective. [`ratio`] is the WCAG figure: the relative luminance of the
//! lighter colour over the darker, both offset, giving 1.0 for two identical
//! colours and 21.0 for black on white.
//!
//! # The thresholds, and why the low one is used here
//!
//! WCAG asks for **4.5:1** for body text and allows **3.0:1** for large text
//! — 18pt and up, or 14pt bold. A game UI is not a document and the standard
//! was not written for one, but the number does not care what the text is for:
//! below about 3:1 a lot of people cannot read it at all, and between 3 and 4.5
//! it depends on the screen.
//!
//! So [`Level::Large`] is the floor this crate treats as a defect and
//! [`Level::Body`] is what small text should reach. What a game does with the
//! band between them is a design decision; what it should not do is remain
//! unaware of which of its labels sit there.
//!
//! # Alpha
//!
//! A colour drawn at less than full alpha is composited over its background
//! first, because a 60%-alpha white on near-black is not white — it is grey, and
//! measuring the un-composited colour would flatter it by a wide margin. That is
//! the mistake this module would otherwise make on every dimmed label in a game.

use macroquad::prelude::Color;

/// The two WCAG thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// 4.5:1 — what small text should reach.
    Body,
    /// 3.0:1 — the floor, allowed for large text and treated as the line below
    /// which anything is a defect.
    Large,
}

impl Level {
    pub fn ratio(self) -> f32 {
        match self {
            Level::Body => 4.5,
            Level::Large => 3.0,
        }
    }

    /// The size at or above which WCAG calls text large, in points.
    pub const LARGE_TEXT: f32 = 18.0;

    /// Which threshold applies to text of this size.
    pub fn for_size(font_size: f32) -> Self {
        if font_size >= Self::LARGE_TEXT {
            Level::Large
        } else {
            Level::Body
        }
    }
}

/// Relative luminance, as WCAG defines it.
///
/// Not the same as perceived brightness and not a simple average: the
/// coefficients are the sRGB primaries' contribution to luminance, and each
/// channel is linearised first because sRGB is stored gamma-encoded. Averaging
/// the raw channels instead — the obvious shortcut — overstates blue by a
/// factor of twelve.
pub fn relative_luminance(color: Color) -> f32 {
    fn linear(channel: f32) -> f32 {
        let c = channel.clamp(0.0, 1.0);
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

/// Composite `over` onto `under` using `over`'s alpha.
///
/// A 60%-alpha label is not its own colour. Measuring the un-composited value
/// would flatter every dimmed piece of text in a UI.
pub fn flatten(over: Color, under: Color) -> Color {
    let a = over.a.clamp(0.0, 1.0);
    Color::new(
        over.r * a + under.r * (1.0 - a),
        over.g * a + under.g * (1.0 - a),
        over.b * a + under.b * (1.0 - a),
        1.0,
    )
}

/// Contrast ratio between two colours, 1.0 to 21.0.
///
/// `foreground` is composited over `background` first, so alpha is accounted
/// for. The background's own alpha is ignored: something is behind it, and this
/// crate cannot know what.
pub fn ratio(foreground: Color, background: Color) -> f32 {
    let fg = relative_luminance(flatten(foreground, background));
    let bg = relative_luminance(background);
    let (lighter, darker) = if fg > bg { (fg, bg) } else { (bg, fg) };
    (lighter + 0.05) / (darker + 0.05)
}

/// Darken `background` until `foreground` can be read off it.
///
/// The alternative is hand-picking a fill for every button tone and hoping
/// nobody adjusts one later. Deriving it from the requirement means a fill
/// **cannot** be unreadable: pick the colour you want, and it is darkened only
/// as far as it has to be, which for an already-legible pairing is not at all.
///
/// Darkens rather than lightens because the surfaces this is for are fills a
/// light label sits on; scaling toward black keeps the hue and drops the
/// luminance, which is exactly the axis contrast is measured on.
pub fn darken_until(background: Color, foreground: Color, font_size: f32) -> Color {
    let required = Level::for_size(font_size).ratio();
    if ratio(foreground, background) >= required {
        return background;
    }
    // Thirty-two steps to black. Enough resolution that the result is within a
    // percent of the lightest colour that works, and bounded so a pairing that
    // can never pass — white text on white, asked for at any size — terminates
    // at black rather than looping.
    let mut best = background;
    for step in 1..=32 {
        let factor = 1.0 - step as f32 / 32.0;
        best = Color::new(
            background.r * factor,
            background.g * factor,
            background.b * factor,
            background.a,
        );
        if ratio(foreground, best) >= required {
            break;
        }
    }
    best
}

/// Does this pairing meet the threshold for text of this size?
pub fn passes(foreground: Color, background: Color, font_size: f32) -> bool {
    ratio(foreground, background) >= Level::for_size(font_size).ratio()
}

#[cfg(test)]
mod tests;
