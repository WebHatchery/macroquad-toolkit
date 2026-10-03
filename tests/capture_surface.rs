//! Public capture-surface dimension resolution tests.

use macroquad_toolkit::capture::capture_surface_size;

#[test]
fn capture_surface_dimensions_use_overrides_and_reject_nonpositive_sizes() {
    let prefix = format!("MQT_CAPTURE_SURFACE_TEST_{}", std::process::id());
    let width_name = format!("{prefix}_WINDOW_WIDTH");
    let height_name = format!("{prefix}_WINDOW_HEIGHT");
    let previous_width = std::env::var_os(&width_name);
    let previous_height = std::env::var_os(&height_name);

    std::env::remove_var(&width_name);
    std::env::remove_var(&height_name);
    assert_eq!(
        capture_surface_size(&prefix, (1280, 720)).unwrap(),
        (1280, 720)
    );

    std::env::set_var(&width_name, "1920");
    std::env::set_var(&height_name, "1080");
    assert_eq!(
        capture_surface_size(&prefix, (1280, 720)).unwrap(),
        (1920, 1080)
    );

    std::env::set_var(&width_name, "0");
    assert!(capture_surface_size(&prefix, (1280, 720)).is_err());

    match previous_width {
        Some(value) => std::env::set_var(&width_name, value),
        None => std::env::remove_var(&width_name),
    }
    match previous_height {
        Some(value) => std::env::set_var(&height_name, value),
        None => std::env::remove_var(&height_name),
    }
}
