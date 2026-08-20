use super::*;

#[test]
fn macroquad_font_style_never_resolves_the_registered_toolkit_font() {
    let style = TextStyle::new(18.0, WHITE).with_macroquad_font();

    assert!(style.resolved_font().is_none());
    assert!(style.params().font.is_none());
}

#[test]
fn font_size_conversion_clamps_invalid_sizes_to_macroquad_range() {
    assert_eq!(font_size_u16(0.0), 1);
    assert_eq!(font_size_u16(-20.0), 1);
    assert_eq!(font_size_u16(24.4), 24);
    assert_eq!(font_size_u16(24.6), 25);
}
