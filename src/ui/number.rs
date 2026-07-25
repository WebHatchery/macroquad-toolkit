//! Writing numbers the way people read them.
//!
//! A game whose whole subject is a quantity — credits, score, population, ore —
//! spends most of its screen showing integers, and `to_string()` renders
//! `1009419` as seven characters a player has to count with their eye to know
//! whether it is one million or ten. Grouping is not decoration; it is the
//! difference between reading a number and parsing one.
//!
//! Two shapes are offered and they are for different jobs:
//!
//! - [`grouped`] keeps every digit and inserts separators. For anything the
//!   player might do arithmetic on, or care about exactly: a balance, a stake,
//!   what a spin just paid.
//! - [`compact`] trades the low digits for width — `1.2M` — and is for places
//!   where the magnitude is the message and the units are noise: an axis label,
//!   a leaderboard, a bar chart.
//!
//! Never [`compact`] for a balance. "You have 1.2M credits" reads as a rounding
//! of something the player believes they know exactly, and being unable to see
//! your own money to the credit is the kind of small dishonesty that costs more
//! trust than it saves pixels.

/// Digits per group. Three, in the writing systems this is built for.
const GROUP: usize = 3;

/// `1009419` → `1,009,419`, with the sign preserved.
///
/// Every digit is kept. The separator is a plain comma rather than a thin space
/// or a locale-aware choice, because a bitmap font in a game is unlikely to have
/// the former and this crate has no business guessing the latter.
pub fn grouped(value: i64) -> String {
    grouped_with(value, ',')
}

/// [`grouped`], with the separator chosen by the caller.
pub fn grouped_with(value: i64, separator: char) -> String {
    // `-i64::MIN` overflows, so the digits are taken from the unsigned
    // magnitude. It is the one input that would panic in a release build after
    // sailing through every test written with small numbers.
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();

    let mut out = String::with_capacity(digits.len() + digits.len() / GROUP + 1);
    if negative {
        out.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(GROUP) {
            out.push(separator);
        }
        out.push(digit);
    }
    out
}

/// `1_240_000` → `1.2M`. For width-constrained labels only.
///
/// Rounds toward zero rather than to nearest, so a compact figure is never
/// larger than the number it stands for. A bar labelled `1.3M` sitting beside a
/// total of `1,249,999` invites the reader to think they have lost a digit
/// somewhere; one labelled `1.2M` never does.
pub fn compact(value: i64) -> String {
    const UNITS: [(i64, char); 3] = [(1_000_000_000, 'B'), (1_000_000, 'M'), (1_000, 'K')];

    let magnitude = value.unsigned_abs();
    let sign = if value < 0 { "-" } else { "" };

    for (scale, suffix) in UNITS {
        let scale = scale as u128;
        if magnitude as u128 >= scale {
            let whole = magnitude as u128 / scale;
            let tenths = (magnitude as u128 % scale) * 10 / scale;
            // Past three digits the decimal is noise: `1234.5M` is wider than
            // the number it was shortening.
            if whole >= 100 || tenths == 0 {
                return format!("{}{}{}", sign, whole, suffix);
            }
            return format!("{}{}.{}{}", sign, whole, tenths, suffix);
        }
    }
    format!("{}{}", sign, magnitude)
}

/// A signed figure that always shows its sign, grouped. For a net position,
/// where `+400` and `400` say different things.
pub fn signed(value: i64) -> String {
    if value > 0 {
        format!("+{}", grouped(value))
    } else {
        grouped(value)
    }
}

#[cfg(test)]
mod tests {
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
}
