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
mod tests;
