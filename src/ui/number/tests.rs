//! Unit tests for numeric formatting helpers.

use super::*;

#[test]
fn small_numbers_are_left_alone() {
    for value in 0..1_000i64 {
        assert_eq!(grouped(value), value.to_string());
    }
}

#[test]
fn groups_land_every_three_digits() {
    assert_eq!(grouped(1_000), "1,000");
    assert_eq!(grouped(1_009_419), "1,009,419");
    assert_eq!(grouped(999_999), "999,999");
    assert_eq!(grouped(1_000_000_000), "1,000,000,000");
}

#[test]
fn the_sign_stays_outside_the_groups() {
    assert_eq!(grouped(-1_009_419), "-1,009,419");
    assert_eq!(grouped(-999), "-999");
    assert_eq!(grouped(-1_000), "-1,000");
}

#[test]
fn the_most_negative_integer_does_not_panic() {
    // `-i64::MIN` overflows. It is the one input that sails through every
    // test written with small numbers and then panics in a release build.
    assert_eq!(grouped(i64::MIN), "-9,223,372,036,854,775,808");
    // There is no unit above B, so the largest values stop being shortened
    // rather than being wrong. Nothing a game counts gets here; the point is
    // that reaching it does not bring the frame down.
    assert_eq!(compact(i64::MIN), "-9223372036B");
}

#[test]
fn grouping_never_loses_or_invents_a_digit() {
    let mut value = 7i64;
    for _ in 0..18 {
        for candidate in [value, -value] {
            let text = grouped(candidate);
            let digits: String = text.chars().filter(char::is_ascii_digit).collect();
            assert_eq!(digits, candidate.unsigned_abs().to_string());
        }
        value = value.saturating_mul(10).saturating_add(3);
    }
}

#[test]
fn a_separator_can_be_chosen() {
    assert_eq!(grouped_with(1_234_567, ' '), "1 234 567");
    assert_eq!(grouped_with(1_234_567, '.'), "1.234.567");
}

#[test]
fn compact_shortens_only_what_is_long_enough_to_need_it() {
    assert_eq!(compact(0), "0");
    assert_eq!(compact(999), "999");
    assert_eq!(compact(1_000), "1K");
    assert_eq!(compact(1_500), "1.5K");
    assert_eq!(compact(1_240_000), "1.2M");
    assert_eq!(compact(2_000_000_000), "2B");
}

#[test]
fn compact_never_rounds_up() {
    // A bar labelled 1.3M beside a total of 1,249,999 invites the reader to
    // think a digit went missing.
    assert_eq!(compact(1_249_999), "1.2M");
    assert_eq!(compact(1_999_999), "1.9M");
    // Three whole digits, so the decimal goes — understating by 999 and
    // never overstating, which is the property that matters.
    assert_eq!(compact(999_999), "999K");

    let mut value = 1i64;
    for _ in 0..18 {
        for candidate in [value, value - 1, value + 1] {
            if candidate <= 0 {
                continue;
            }
            let text = compact(candidate);
            let scale = match text.chars().last() {
                Some('K') => 1_000f64,
                Some('M') => 1_000_000f64,
                Some('B') => 1_000_000_000f64,
                _ => 1.0,
            };
            let shown: f64 = text.trim_end_matches(['K', 'M', 'B']).parse().unwrap();
            assert!(
                shown * scale <= candidate as f64 + 1.0,
                "{} shown as {}",
                candidate,
                text
            );
        }
        value = value.saturating_mul(10);
    }
}

#[test]
fn compact_drops_the_decimal_once_it_stops_helping() {
    // `123.4M` is wider than the number it was shortening.
    assert_eq!(compact(123_400_000), "123M");
    assert_eq!(compact(999_000_000), "999M");
    assert!(compact(123_400_000).len() <= 5);
}

#[test]
fn compact_is_never_wider_than_the_number_it_replaces() {
    let mut value = 1_000i64;
    for _ in 0..15 {
        assert!(
            compact(value).len() <= grouped(value).len(),
            "{} compacts to {} against {}",
            value,
            compact(value),
            grouped(value)
        );
        value = value.saturating_mul(7);
    }
}

#[test]
fn a_net_position_always_shows_which_way_it_went() {
    assert_eq!(signed(400), "+400");
    assert_eq!(signed(-400), "-400");
    assert_eq!(signed(0), "0");
    assert_eq!(signed(1_234_567), "+1,234,567");
}
