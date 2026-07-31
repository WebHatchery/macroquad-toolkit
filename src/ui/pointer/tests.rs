//! Unit tests for pointer capture and hit-testing.

use super::*;

fn reset() {
    begin_target_audit();
}

#[test]
fn nothing_is_measured_until_the_audit_starts() {
    end_target_audit();
    AUDIT.with(|audit| {
        let mut audit = audit.borrow_mut();
        audit.seen.clear();
        audit.areas.clear();
    });
    note_target("button", Rect::new(0.0, 0.0, 10.0, 10.0));
    assert!(smallest_touchable_width(1280.0).is_none());
}

#[test]
fn the_smaller_side_is_the_target() {
    // A control 200 wide and 20 tall is a 20-pixel target however generous
    // it looks lying down.
    reset();
    note_target("wide and flat", Rect::new(0.0, 0.0, 200.0, 20.0));
    let (width, label) = smallest_touchable_width(1280.0).unwrap();
    assert_eq!(label, "wide and flat");
    // Grown to 44 tall, so it asks for the design width and no more.
    assert!((width - 1280.0).abs() < 1.0, "{}", width);
}

#[test]
fn the_worst_control_sets_the_number() {
    reset();
    note_target("generous", Rect::new(0.0, 0.0, 120.0, 90.0));
    note_target("mean", Rect::new(0.0, 0.0, 30.0, 24.0));
    note_target("middling", Rect::new(0.0, 0.0, 80.0, 40.0));
    // Everything below the standard is grown to exactly it, so they tie —
    // and the answer is the design width, which is the point.
    let (width, _) = smallest_touchable_width(1280.0).unwrap();
    assert!((width - 1280.0).abs() < 1.0, "{}", width);
}

#[test]
fn a_bigger_control_asks_for_a_smaller_window() {
    // The property that makes the number actionable: it goes down when the
    // layout is improved.
    reset();
    note_target("small", Rect::new(0.0, 0.0, 30.0, 30.0));
    let (tight, _) = smallest_touchable_width(1280.0).unwrap();

    reset();
    note_target("large", Rect::new(0.0, 0.0, 60.0, 60.0));
    let (roomy, _) = smallest_touchable_width(1280.0).unwrap();

    assert!(roomy < tight, "{} against {}", roomy, tight);
}

#[test]
fn a_control_already_at_the_standard_asks_for_the_logical_width() {
    // A control exactly MIN_TARGET across needs the window to be exactly the
    // logical size — one logical pixel to one CSS pixel.
    reset();
    note_target("just right", Rect::new(0.0, 0.0, MIN_TARGET, MIN_TARGET));
    let (width, _) = smallest_touchable_width(1280.0).unwrap();
    assert!((width - 1280.0).abs() < 0.01, "{}", width);
}

#[test]
fn everything_below_the_standard_is_listed_worst_first() {
    // One number says how bad it is; this says what to fix, in order.
    reset();
    note_target("fine", Rect::new(0.0, 0.0, 60.0, 60.0));
    note_target("small", Rect::new(0.0, 0.0, 90.0, 30.0));
    note_target("smallest", Rect::new(0.0, 0.0, 20.0, 90.0));

    let under = undersized_targets();
    assert_eq!(under.len(), 2, "{:?}", under);
    assert_eq!(under[0].1, "smallest");
    assert_eq!(under[1].1, "small");
}

#[test]
fn a_small_control_gets_a_generous_hit_area_around_its_centre() {
    let drawn = Rect::new(100.0, 100.0, 20.0, 20.0);
    let touch = touch_area(drawn);
    assert!((touch.w - MIN_TARGET).abs() < 1e-3);
    assert!((touch.h - MIN_TARGET).abs() < 1e-3);
    // Grown around the middle, not from a corner, or the target would sit
    // off to one side of the thing it belongs to.
    assert!((touch.center().x - drawn.center().x).abs() < 1e-3);
    assert!((touch.center().y - drawn.center().y).abs() < 1e-3);
}

#[test]
fn a_control_already_large_enough_is_untouched() {
    let drawn = Rect::new(0.0, 0.0, 200.0, 70.0);
    let touch = touch_area(drawn);
    assert!((touch.w - 200.0).abs() < 1e-3);
    assert!((touch.h - 70.0).abs() < 1e-3);
}

#[test]
fn only_the_short_side_grows() {
    let touch = touch_area(Rect::new(0.0, 0.0, 300.0, 28.0));
    assert!((touch.w - 300.0).abs() < 1e-3);
    assert!((touch.h - MIN_TARGET).abs() < 1e-3);
}

#[test]
fn growth_stops_halfway_to_a_neighbour() {
    // The fault the verification harness found: on a narrow screen the
    // controls sit closer together, and growing every one to the standard
    // made them overlap by thousands of square pixels.
    let a = Rect::new(0.0, 0.0, 100.0, 28.0);
    let b = Rect::new(0.0, 36.0, 100.0, 28.0);
    let grown = touch_area_among(a, &[a, b]);
    // 8px gap, so 4px each way, not the 8 the standard would want.
    assert!((grown.bottom() - 32.0).abs() < 0.01, "{:?}", grown);
    assert!(grown.bottom() <= b.y);
}

#[test]
fn a_control_with_room_still_takes_the_whole_standard() {
    let lonely = Rect::new(0.0, 0.0, 20.0, 20.0);
    let far = Rect::new(0.0, 400.0, 20.0, 20.0);
    let grown = touch_area_among(lonely, &[lonely, far]);
    assert!((grown.h - MIN_TARGET).abs() < 0.01, "{:?}", grown);
}

#[test]
fn neighbours_that_share_no_band_do_not_constrain() {
    // A control in another column cannot be run into vertically.
    let here = Rect::new(0.0, 0.0, 20.0, 20.0);
    let elsewhere = Rect::new(500.0, 22.0, 20.0, 20.0);
    let grown = touch_area_among(here, &[here, elsewhere]);
    assert!((grown.h - MIN_TARGET).abs() < 0.01, "{:?}", grown);
}

#[test]
fn grown_areas_never_overlap_however_tight_the_row() {
    // The property, stated directly: whatever the spacing, no two hit areas
    // may claim the same pixel.
    for gap in [0.0f32, 2.0, 6.0, 20.0, 60.0] {
        let rects: Vec<Rect> = (0..5)
            .map(|i| Rect::new(0.0, i as f32 * (28.0 + gap), 100.0, 28.0))
            .collect();
        let grown: Vec<Rect> = rects.iter().map(|r| touch_area_among(*r, &rects)).collect();
        for (i, a) in grown.iter().enumerate() {
            for b in grown.iter().skip(i + 1) {
                let h = (a.bottom().min(b.bottom()) - a.y.max(b.y)).max(0.0);
                assert!(h <= 0.01, "gap {} left {:?} over {:?}", gap, a, b);
            }
        }
    }
}

/// The cost of growing hit areas, made visible.
///
/// A press landing on the wrong control is worse than one landing on
/// nothing, so this must not be something anyone has to remember to check.
#[test]
fn neighbours_that_now_overlap_are_reported() {
    reset();
    // Two 28-tall controls eight pixels apart: fine as drawn, overlapping
    // once both are grown to forty-four.
    note_target("upper", Rect::new(0.0, 0.0, 100.0, 28.0));
    note_target("lower", Rect::new(0.0, 36.0, 100.0, 28.0));
    let clashes = overlapping_targets();
    assert_eq!(clashes.len(), 1, "{:?}", clashes);
    assert_eq!(clashes[0].0, "upper");
    assert_eq!(clashes[0].1, "lower");
}

#[test]
fn controls_with_room_between_them_do_not_clash() {
    reset();
    note_target("upper", Rect::new(0.0, 0.0, 100.0, 28.0));
    note_target("lower", Rect::new(0.0, 60.0, 100.0, 28.0));
    assert!(overlapping_targets().is_empty());
}

#[test]
fn a_control_with_no_size_is_ignored_rather_than_dividing_by_zero() {
    reset();
    note_target("collapsed", Rect::new(0.0, 0.0, 0.0, 40.0));
    assert!(smallest_touchable_width(1280.0).is_none());
}

#[test]
fn a_mouse_pointer_hovers_and_a_finger_does_not() {
    // Not a behaviour test — there is no input here — but the contract is
    // worth pinning: a UI that lights up under a finger is showing the
    // player what they have already committed to.
    let mouse = Pointer {
        hovering: true,
        ..Pointer::default()
    };
    let finger = Pointer::default();
    assert!(mouse.hovering);
    assert!(!finger.hovering);
}

/// The fault a tablet found: a touch arrives in physical pixels and the
/// cursor in logical ones, so on a 2x screen every finger landed twice as
/// far into the layout as it really was.
#[test]
fn a_touch_is_brought_into_the_same_units_as_the_cursor() {
    // What the platform hands over on an iPad for a finger at (300, 200).
    assert_eq!(logical_touch(vec2(600.0, 400.0), 2.0), vec2(300.0, 200.0));
    // A 3x phone screen, and a 150% desktop.
    assert_eq!(logical_touch(vec2(900.0, 600.0), 3.0), vec2(300.0, 200.0));
    assert_eq!(logical_touch(vec2(450.0, 300.0), 1.5), vec2(300.0, 200.0));
}

#[test]
fn an_unscaled_display_is_left_exactly_alone() {
    // The case every desktop is in, and the reason this went unnoticed.
    let raw = vec2(317.0, 42.5);
    assert_eq!(logical_touch(raw, 1.0), raw);
}

/// A scale of zero would silently teleport every touch to infinity. Nothing
/// should report one, but a hit test is not the place to find out.
#[test]
fn a_nonsense_scale_is_ignored_rather_than_dividing_by_zero() {
    let raw = vec2(100.0, 50.0);
    assert_eq!(logical_touch(raw, 0.0), raw);
}

#[test]
fn a_finger_is_held_until_it_lifts_or_is_taken_away() {
    assert!(finger_is_on_the_glass(TouchPhase::Started));
    assert!(finger_is_on_the_glass(TouchPhase::Moved));
    assert!(finger_is_on_the_glass(TouchPhase::Stationary));
    assert!(!finger_is_on_the_glass(TouchPhase::Ended));
    // The one that is easy to miss: the system took the gesture, so there
    // is nothing on the glass even though nobody lifted anything.
    assert!(!finger_is_on_the_glass(TouchPhase::Cancelled));
}

#[test]
fn a_finger_inside_a_control_counts_as_pressing_it() {
    // A touch has no separate "button down" to read, so being there is
    // being on it.
    let rect = Rect::new(10.0, 10.0, 100.0, 40.0);
    let finger = Pointer {
        position: vec2(50.0, 30.0),
        released: false,
        down: true,
        hovering: false,
    };
    assert!(finger.pressing(rect));

    let elsewhere = Pointer {
        position: vec2(500.0, 500.0),
        ..finger
    };
    assert!(!elsewhere.pressing(rect));
}
