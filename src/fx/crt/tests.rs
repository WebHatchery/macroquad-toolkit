use super::*;

#[test]
fn flicker_stays_in_unit_range() {
    for i in 0..1000 {
        let f = flicker_factor(i as f32 * 0.013);
        assert!((0.0..=1.0).contains(&f), "flicker out of range: {f}");
    }
}

#[test]
fn scan_band_wraps_within_span() {
    let (h, band) = (720.0, 100.0);
    for i in 0..2000 {
        let y = scan_band_y(i as f32 * 0.05, h, 0.2, band);
        assert!(y >= -band - 0.001, "band above wrap: {y}");
        assert!(y <= h + 0.001, "band below screen: {y}");
    }
}

#[test]
fn presets_disable_nothing_by_default() {
    for style in [CrtStyle::amber(), CrtStyle::green()] {
        assert!(style.scanline_alpha > 0.0);
        assert!(style.vignette_alpha > 0.0);
        assert!(style.corner_radius > 0.0);
    }
}
