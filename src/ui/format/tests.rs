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
