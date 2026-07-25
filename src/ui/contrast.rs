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
mod tests {
    use super::*;

    const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);

    #[test]
    fn the_extremes_are_where_the_standard_puts_them() {
        assert!((ratio(WHITE, BLACK) - 21.0).abs() < 0.01);
        assert!((ratio(BLACK, WHITE) - 21.0).abs() < 0.01);
        assert!((ratio(WHITE, WHITE) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn the_ratio_does_not_care_which_way_round_it_is_asked() {
        let a = Color::new(0.2, 0.6, 0.3, 1.0);
        let b = Color::new(0.9, 0.85, 0.4, 1.0);
        assert!((ratio(a, b) - ratio(b, a)).abs() < 1e-4);
    }

    #[test]
    fn luminance_weights_the_channels_rather_than_averaging_them() {
        // The shortcut this exists to avoid: averaging raw channels overstates
        // blue enormously. Full green is far brighter than full blue.
        let green = relative_luminance(Color::new(0.0, 1.0, 0.0, 1.0));
        let blue = relative_luminance(Color::new(0.0, 0.0, 1.0, 1.0));
        assert!(green > blue * 9.0, "green {} blue {}", green, blue);
    }

    #[test]
    fn a_translucent_colour_is_measured_where_it_lands() {
        // A 40%-alpha white on black is grey. Measuring the un-composited white
        // would claim 21:1 for something barely legible.
        let ghost = Color::new(1.0, 1.0, 1.0, 0.4);
        let honest = ratio(ghost, BLACK);
        let flattering = ratio(Color::new(1.0, 1.0, 1.0, 1.0), BLACK);
        assert!(
            honest < flattering * 0.5,
            "{} against {}",
            honest,
            flattering
        );
        assert!(honest > 1.0);
    }

    #[test]
    fn a_fully_transparent_colour_has_no_contrast_at_all() {
        let invisible = Color::new(1.0, 1.0, 1.0, 0.0);
        assert!((ratio(invisible, BLACK) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn large_text_is_held_to_the_lower_bar() {
        assert_eq!(Level::for_size(21.0), Level::Large);
        assert_eq!(Level::for_size(18.0), Level::Large);
        assert_eq!(Level::for_size(17.9), Level::Body);
        assert!(Level::Large.ratio() < Level::Body.ratio());
    }

    #[test]
    fn passing_follows_the_size() {
        // A pairing between the two thresholds passes as large text and fails as
        // small — which is the entire reason the size is an argument.
        let stone = Color::new(0.098, 0.086, 0.098, 1.0);
        let mid = Color::new(0.42, 0.40, 0.38, 1.0);
        let r = ratio(mid, stone);
        assert!(
            (3.0..4.5).contains(&r),
            "pick a colour in the band; this one is {}",
            r
        );
        assert!(passes(mid, stone, 20.0));
        assert!(!passes(mid, stone, 14.0));
    }

    #[test]
    fn a_legible_pairing_is_left_alone() {
        let dark = Color::new(0.1, 0.1, 0.12, 1.0);
        assert_eq!(darken_until(dark, WHITE, 14.0), dark);
    }

    #[test]
    fn an_illegible_fill_is_darkened_until_it_reads() {
        // The failure this exists for: white on a saturated mid-tone, which is
        // every bright button in every game and measures around 2:1.
        let green = Color::new(0.30, 0.75, 0.40, 1.0);
        assert!(!passes(WHITE, green, 17.0));

        let fixed = darken_until(green, WHITE, 17.0);
        assert!(passes(WHITE, fixed, 17.0), "{:?}", fixed);
        // Only as far as it has to go: still recognisably the same colour.
        assert!(fixed.g > fixed.r && fixed.g > fixed.b, "{:?}", fixed);
    }

    #[test]
    fn darkening_keeps_the_hue_and_the_alpha() {
        let blue = Color::new(0.25, 0.45, 0.85, 0.9);
        let fixed = darken_until(blue, WHITE, 14.0);
        assert!((fixed.a - 0.9).abs() < 1e-6);
        // Channel *order* is what a hue is, at this resolution.
        assert!(fixed.b > fixed.g && fixed.g > fixed.r);
    }

    #[test]
    fn a_pairing_that_can_never_pass_terminates_rather_than_looping() {
        // Dark text is the case darkening cannot help: the background is
        // already as close to the foreground as it can get, and every step
        // makes it worse. It has to stop at black rather than search forever.
        let fixed = darken_until(Color::new(0.05, 0.05, 0.05, 1.0), BLACK, 14.0);
        assert!(relative_luminance(fixed) < 0.01, "{:?}", fixed);
    }

    #[test]
    fn white_on_white_is_fixable_by_darkening_and_gets_fixed() {
        // Worth stating, because it looks like the impossible case and is not:
        // darkening the *background* is exactly the remedy.
        let fixed = darken_until(WHITE, WHITE, 14.0);
        assert!(passes(WHITE, fixed, 14.0), "{:?}", fixed);
    }

    #[test]
    fn every_tone_a_game_might_pick_ends_up_readable() {
        let steps = [0.0, 0.25, 0.5, 0.75, 1.0];
        for r in steps {
            for g in steps {
                for b in steps {
                    let fill = Color::new(r, g, b, 1.0);
                    let fixed = darken_until(fill, WHITE, 17.0);
                    assert!(
                        passes(WHITE, fixed, 17.0) || relative_luminance(fixed) < 0.01,
                        "{:?} -> {:?}",
                        fill,
                        fixed
                    );
                }
            }
        }
    }

    #[test]
    fn the_ratio_never_leaves_its_range() {
        let steps = [0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0];
        for r in steps {
            for g in steps {
                for b in steps {
                    let color = Color::new(r, g, b, 1.0);
                    for other in [BLACK, WHITE, Color::new(0.5, 0.5, 0.5, 1.0)] {
                        let value = ratio(color, other);
                        assert!((1.0..=21.0).contains(&value), "{}", value);
                    }
                }
            }
        }
    }
}
