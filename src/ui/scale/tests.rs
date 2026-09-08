use super::*;
use macroquad::prelude::vec2;

#[test]
fn scaling_resizes_layout_and_keeps_input_aligned() {
    for scale in [0.75, 1.0, 1.25, 1.5] {
        let ui = VirtualUi::from_scaled_screen_size(1200.0, 900.0, scale, 320.0, 480.0);
        assert_eq!(ui.logical_width, 1200.0 / scale);
        assert_eq!(ui.scale, scale);
        assert_eq!(ui.viewport_for_dpi(2.0), (0, 0, 2400, 1800));
        let point = vec2(140.0, 210.0);
        assert_eq!(ui.screen_to_ui_checked(ui.ui_to_screen(point)), Some(point));
    }
}

#[test]
fn small_windows_fit_minimum_layout_without_clipping() {
    let ui = VirtualUi::from_scaled_screen_size(320.0, 480.0, 1.5, 320.0, 480.0);
    assert_eq!(ui.logical_width, 320.0);
    assert_eq!(ui.logical_height, 480.0);
    assert_eq!(ui.viewport_for_dpi(1.0), (0, 0, 320, 480));
}

#[test]
fn invalid_preferences_are_safe() {
    for scale in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(sanitize_ui_scale(scale), 1.0);
    }
    assert_eq!(sanitize_ui_scale(-1.0), MIN_UI_SCALE);
    assert_eq!(sanitize_ui_scale(9.0), MAX_UI_SCALE);
}
