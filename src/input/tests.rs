use super::*;

#[test]
fn menu_cursor_wraps_both_directions() {
    let mut cursor = MenuCursor::new(3);
    cursor.select_prev();
    assert_eq!(cursor.index(), 2);
    cursor.select_next();
    assert_eq!(cursor.index(), 0);
    cursor.select_next();
    assert_eq!(cursor.index(), 1);
}

#[test]
fn menu_cursor_navigate_reports_movement() {
    let mut cursor = MenuCursor::new(2);
    assert!(!cursor.navigate(0));
    assert!(cursor.navigate(1));
    assert_eq!(cursor.index(), 1);
    assert!(cursor.navigate(-1));
    assert_eq!(cursor.index(), 0);
}

#[test]
fn menu_cursor_clamps_on_resize_and_set() {
    let mut cursor = MenuCursor::new(5);
    cursor.set_index(4);
    cursor.set_len(3);
    assert_eq!(cursor.index(), 2);
    cursor.set_index(99);
    assert_eq!(cursor.index(), 2);
}

#[test]
fn empty_menu_cursor_is_inert() {
    let mut cursor = MenuCursor::new(0);
    assert!(cursor.is_empty());
    assert!(!cursor.navigate(1));
    cursor.select_prev();
    assert_eq!(cursor.index(), 0);
}

#[test]
fn hit_test_returns_first_matching_target() {
    let targets = [
        HitTarget::new(Rect::new(0.0, 0.0, 10.0, 10.0), "first"),
        HitTarget::new(Rect::new(0.0, 0.0, 20.0, 20.0), "second"),
    ];

    assert_eq!(hit_test(targets, vec2(5.0, 5.0)), Some("first"));
}

#[test]
fn hit_test_ignores_points_outside_targets() {
    let targets = [HitTarget::new(Rect::new(0.0, 0.0, 10.0, 10.0), 7)];

    assert_eq!(hit_test(targets, vec2(12.0, 5.0)), None);
}

#[test]
fn semantic_gamepad_frame_defaults_to_disconnected_and_idle() {
    assert_eq!(
        GamepadFrame::default(),
        GamepadFrame {
            connected: false,
            confirm: false,
            cancel: false,
            secondary: false,
            tertiary: false,
            menu: false,
            next: false,
            previous: false,
            up: false,
            down: false,
            left: false,
            right: false,
        }
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_gamepad_poller_is_a_safe_no_op() {
    let mut input = GamepadInput::new();
    assert_eq!(input.capture(), GamepadFrame::default());
    input.rumble(120, 0.5, 0.8);
}
