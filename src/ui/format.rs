//! Turning numbers and names into the strings a player reads.
//!
//! # Why this is not in `font`
//!
//! It was, and `font.rs`'s own module comment confessed it: "UI font loading,
//! text measurement/layout/drawing, **and value formatting**". Two
//! responsibilities in one file, joined only by both being vaguely about text —
//! and between them they carried it past the 800-line limit every project in
//! this workspace is held to.
//!
//! Nothing here touches a font, a glyph or macroquad. A money figure is the same
//! string whether it is drawn, logged, or asserted on in a test with no window
//! open, which is exactly why these are the functions a headless test reaches
//! for and why they belong on their own.

/// Format an integer currency value with comma separators.
pub fn format_money(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let mut digits = value.abs().to_string();
    let mut result = String::new();

    while digits.len() > 3 {
        let tail = digits.split_off(digits.len() - 3);
        if result.is_empty() {
            result = tail;
        } else {
            result = format!("{tail},{result}");
        }
    }

    if result.is_empty() {
        format!("{sign}${digits}")
    } else {
        format!("{sign}${digits},{result}")
    }
}

/// Format an integer currency value compactly for dense UI.
pub fn format_compact_money(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let abs = value.abs();
    if abs >= 1_000_000 {
        format!("{sign}${:.1}m", abs as f32 / 1_000_000.0)
    } else if abs >= 1_000 {
        format!("{sign}${}k", abs / 1_000)
    } else {
        format!("{sign}${abs}")
    }
}

/// Magnitude suffixes for [`format_amount`] / [`format_rate`], each step a
/// thousandfold: K (1e3), M (1e6), B (1e9), T (1e12), then Qa/Qi/Sx/Sp/Oc/No
/// up to 1e30. Values beyond the table fall back to scientific notation.
const AMOUNT_SUFFIXES: [&str; 11] = ["", "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No"];

/// Formats a large `f64` quantity with idle/incremental-genre magnitude
/// suffixes: integers below 1000, two decimals with a suffix up to `No`
/// (1e30), scientific notation beyond. Handles negatives, and degrades
/// `NaN`/infinity to `∞` rather than printing junk.
///
/// Unlike [`format_compact_money`], which takes an `i64` and saturates around
/// 9.2e18, this covers the full `f64` range that idle games routinely reach.
///
/// ```
/// # use macroquad_toolkit::ui::format_amount;
/// assert_eq!(format_amount(999.0), "999");
/// assert_eq!(format_amount(1_500.0), "1.50K");
/// assert_eq!(format_amount(2_340_000.0), "2.34M");
/// assert_eq!(format_amount(1e30), "1.00No");
/// ```
pub fn format_amount(value: f64) -> String {
    if !value.is_finite() {
        return "∞".to_owned();
    }
    if value < 0.0 {
        return format!("-{}", format_amount(-value));
    }
    if value < 1000.0 {
        return format!("{}", value.floor() as i64);
    }

    let tier = (value.log10() / 3.0).floor() as usize;
    if tier < AMOUNT_SUFFIXES.len() {
        let scaled = value / 1000f64.powi(tier as i32);
        format!("{:.2}{}", scaled, AMOUNT_SUFFIXES[tier])
    } else {
        format!("{:.2e}", value)
    }
}

/// Formats a per-second rate: one decimal below 1000, then the same magnitude
/// suffixes as [`format_amount`].
///
/// ```
/// # use macroquad_toolkit::ui::format_rate;
/// assert_eq!(format_rate(2.5), "2.5");
/// assert_eq!(format_rate(12_500.0), "12.50K");
/// ```
pub fn format_rate(value: f64) -> String {
    if value < 1000.0 {
        format!("{:.1}", value)
    } else {
        format_amount(value)
    }
}

/// Format elapsed seconds as `MM:SS` (e.g. `07:42`). Minutes keep growing
/// past an hour (`75:03`).
pub fn format_mmss(total_seconds: f32) -> String {
    let total = total_seconds.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// Format elapsed seconds as `H:MM:SS` once an hour is reached, otherwise
/// `MM:SS`.
pub fn format_hmmss(total_seconds: f32) -> String {
    let total = total_seconds.max(0.0) as u64;
    let hours = total / 3600;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, (total % 3600) / 60, total % 60)
    } else {
        format_mmss(total_seconds)
    }
}

/// Format an in-game clock as `HH:MM` (e.g. `08:30`). Hours wrap at 24.
pub fn format_clock(hour: u32, minute: u32) -> String {
    format!("{:02}:{:02}", hour % 24, minute % 60)
}

/// Capitalize the first character of a string
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars().collect::<Vec<_>>();
    if let Some(c) = chars.get_mut(0) {
        *c = c.to_ascii_uppercase();
    }
    chars.into_iter().collect()
}

/// Format a type_key (snake_case) into a display name (Title Case)
/// e.g., "health_potion" -> "Health Potion"
pub fn display_name(type_key: &str) -> String {
    type_key
        .split('_')
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_money_with_commas() {
        assert_eq!(format_money(0), "$0");
        assert_eq!(format_money(1_234), "$1,234");
        assert_eq!(format_money(-1_234_567), "-$1,234,567");
    }

    #[test]
    fn formats_compact_money() {
        assert_eq!(format_compact_money(999), "$999");
        assert_eq!(format_compact_money(12_000), "$12k");
        assert_eq!(format_compact_money(1_240_000), "$1.2m");
    }

    #[test]
    fn formats_amount_across_tiers() {
        assert_eq!(format_amount(0.0), "0");
        assert_eq!(format_amount(999.0), "999");
        assert_eq!(format_amount(1_500.0), "1.50K");
        assert_eq!(format_amount(2_340_000.0), "2.34M");
        assert_eq!(format_amount(1e9), "1.00B");
        assert_eq!(format_amount(1e12), "1.00T");
        assert_eq!(format_amount(1e30), "1.00No");
        assert_eq!(format_amount(-1_500.0), "-1.50K");
    }

    #[test]
    fn amount_falls_back_to_scientific_and_survives_extremes() {
        assert_eq!(format_amount(1e36), "1.00e36");
        assert_eq!(format_amount(1e300), "1.00e300");
        // f64::MAX (~1.8e308) is still finite and formats.
        assert!(format_amount(f64::MAX).contains("e308"));
        // Non-finite inputs degrade gracefully rather than printing junk.
        assert_eq!(format_amount(f64::INFINITY), "∞");
        assert_eq!(format_amount(f64::NAN), "∞");
    }

    #[test]
    fn formats_rate_with_one_decimal_then_suffixes() {
        assert_eq!(format_rate(2.5), "2.5");
        assert_eq!(format_rate(12_500.0), "12.50K");
    }

    #[test]
    fn formats_durations() {
        assert_eq!(format_mmss(0.0), "00:00");
        assert_eq!(format_mmss(462.9), "07:42");
        assert_eq!(format_mmss(4503.0), "75:03");
        assert_eq!(format_mmss(-5.0), "00:00");
        assert_eq!(format_hmmss(462.9), "07:42");
        assert_eq!(format_hmmss(4503.0), "1:15:03");
    }

    #[test]
    fn formats_clock() {
        assert_eq!(format_clock(8, 30), "08:30");
        assert_eq!(format_clock(25, 61), "01:01");
    }
}
