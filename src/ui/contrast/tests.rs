//! Unit tests for contrast and luminance helpers.

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
