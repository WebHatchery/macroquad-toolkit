use super::*;

/// A 1280x720 logical UI in a 1280x720 logical window covers the whole
/// framebuffer, whatever the display scaling. Regression test for the
/// letterbox rendering shrunk into the bottom-left corner on scaled
/// displays: `glViewport` takes physical pixels, but `screen_width()` and
/// `offset`/`scale` are logical.
#[test]
fn viewport_is_physical_pixels() {
    let ui = VirtualUi {
        logical_width: 1280.0,
        logical_height: 720.0,
        scale: 1.0,
        offset: Vec2::ZERO,
    };

    assert_eq!(ui.viewport_for_dpi(1.0), (0, 0, 1280, 720));
    // 150% scaling: the framebuffer is 1920x1080, so the viewport must be
    // too -- not the logical 1280x720, which would cover only 4/9 of it.
    assert_eq!(ui.viewport_for_dpi(1.5), (0, 0, 1920, 1080));
    assert_eq!(ui.viewport_for_dpi(2.0), (0, 0, 2560, 1440));
}

/// Letterbox offsets have to scale too, or the bars land in the wrong place.
#[test]
fn viewport_offset_scales_with_dpi() {
    let ui = VirtualUi {
        logical_width: 1280.0,
        logical_height: 720.0,
        scale: 0.5,
        offset: vec2(100.0, 40.0),
    };

    assert_eq!(ui.viewport_for_dpi(1.0), (100, 40, 640, 360));
    assert_eq!(ui.viewport_for_dpi(2.0), (200, 80, 1280, 720));
}

/// Mouse mapping stays in logical space and must not pick up the DPI scale
/// -- `mouse_position()` is already logical.
#[test]
fn screen_to_ui_is_dpi_independent() {
    let ui = VirtualUi {
        logical_width: 1280.0,
        logical_height: 720.0,
        scale: 0.5,
        offset: vec2(100.0, 40.0),
    };

    assert_eq!(ui.screen_to_ui(vec2(100.0, 40.0)), Vec2::ZERO);
    assert_eq!(ui.screen_to_ui(vec2(420.0, 220.0)), vec2(640.0, 360.0));
    assert_eq!(ui.ui_to_screen(vec2(640.0, 360.0)), vec2(420.0, 220.0));
}

#[test]
fn injected_screen_size_preserves_aspect_and_rejects_letterbox_input() {
    let ui = VirtualUi::from_screen_size(1280.0, 720.0, 1024.0, 768.0);

    assert!((ui.scale - 0.8).abs() < f32::EPSILON);
    assert_eq!(ui.offset, vec2(0.0, 96.0));
    assert_eq!(ui.viewport_for_dpi(1.0), (0, 96, 1024, 576));
    assert_eq!(ui.screen_to_ui_checked(vec2(512.0, 10.0)), None);
    assert_eq!(
        ui.screen_to_ui_checked(ui.ui_to_screen(vec2(640.0, 360.0))),
        Some(vec2(640.0, 360.0))
    );
}
