use super::*;

fn touch(id: u64, x: f32, y: f32, phase: TouchPhase) -> GestureTouch {
    GestureTouch::new(id, vec2(x, y), phase)
}

#[test]
fn a_short_single_contact_becomes_one_tap_on_release() {
    let mut gesture = TouchGesture::new();
    let began = gesture.update_with(&[touch(7, 20.0, 30.0, TouchPhase::Started)]);
    assert!(began.active);
    assert_eq!(began.tap, None);

    let ended = gesture.update_with(&[touch(7, 22.0, 31.0, TouchPhase::Ended)]);
    assert_eq!(ended.tap, Some(vec2(22.0, 31.0)));
    assert!(!ended.claimed);

    assert_eq!(gesture.update_with(&[]).tap, None);
}

#[test]
fn a_single_finger_pan_waits_until_the_deadzone_is_crossed() {
    let mut gesture = TouchGesture::new();
    gesture.update_with(&[touch(1, 40.0, 40.0, TouchPhase::Started)]);
    let jitter = gesture.update_with(&[touch(1, 44.0, 40.0, TouchPhase::Moved)]);
    assert!(!jitter.claimed);
    assert_eq!(jitter.pan, Vec2::ZERO);

    let drag = gesture.update_with(&[touch(1, 50.0, 40.0, TouchPhase::Moved)]);
    assert!(drag.claimed);
    assert_eq!(drag.pan, vec2(6.0, 0.0));

    let ended = gesture.update_with(&[touch(1, 50.0, 40.0, TouchPhase::Ended)]);
    assert!(ended.claimed);
    assert_eq!(ended.tap, None);
}

#[test]
fn two_fingers_pan_and_pinch_around_their_center() {
    let mut gesture = TouchGesture::new();
    gesture.update_with(&[
        touch(1, 20.0, 40.0, TouchPhase::Started),
        touch(2, 60.0, 40.0, TouchPhase::Started),
    ]);
    let moved = gesture.update_with(&[
        touch(1, 10.0, 50.0, TouchPhase::Moved),
        touch(2, 70.0, 50.0, TouchPhase::Moved),
    ]);
    assert_eq!(moved.pan, vec2(0.0, 10.0));
    assert!((moved.scale - 1.5).abs() < 1e-6);
    assert_eq!(moved.center, vec2(40.0, 50.0));
    assert!(moved.claimed);
}

#[test]
fn adding_or_removing_a_finger_does_not_jump_the_view() {
    let mut gesture = TouchGesture::new();
    gesture.update_with(&[touch(1, 20.0, 20.0, TouchPhase::Started)]);
    let added = gesture.update_with(&[
        touch(1, 20.0, 20.0, TouchPhase::Stationary),
        touch(2, 80.0, 20.0, TouchPhase::Started),
    ]);
    assert_eq!(added.pan, Vec2::ZERO);
    assert_eq!(added.scale, 1.0);

    let removed = gesture.update_with(&[
        touch(1, 20.0, 20.0, TouchPhase::Stationary),
        touch(2, 80.0, 20.0, TouchPhase::Ended),
    ]);
    assert_eq!(removed.pan, Vec2::ZERO);
    assert_eq!(removed.scale, 1.0);
    assert!(removed.claimed);
}

#[test]
fn a_cancelled_contact_never_activates_a_control() {
    let mut gesture = TouchGesture::new();
    gesture.update_with(&[touch(1, 20.0, 20.0, TouchPhase::Started)]);
    let cancelled = gesture.update_with(&[touch(1, 20.0, 20.0, TouchPhase::Cancelled)]);
    assert_eq!(cancelled.tap, None);
}
