use super::*;

#[test]
fn max_offset_clamps_to_zero_when_content_fits() {
    let view = Rect::new(0.0, 0.0, 100.0, 200.0);
    assert_eq!(ScrollArea::max_offset(view, 150.0), 0.0);
    assert_eq!(ScrollArea::max_offset(view, 500.0), 300.0);
}

#[test]
fn set_offset_never_negative() {
    let mut area = ScrollArea::new();
    area.set_offset(-50.0);
    assert_eq!(area.offset(), 0.0);
    area.set_offset(120.0);
    assert_eq!(area.offset(), 120.0);
}

const VIEW: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 200.0,
    h: 100.0,
};
const CONTENT: f32 = 500.0;

fn at(y: f32, down: bool, pressed: bool) -> ScrollInput {
    ScrollInput {
        // Left of the scrollbar gutter, so these are presses on the list.
        pointer: vec2(50.0, y),
        down,
        pressed,
        wheel: 0.0,
        dt: 1.0 / 60.0,
    }
}

/// Press, drag, lift — the gesture the whole feature exists for.
#[test]
fn dragging_the_content_scrolls_it() {
    let mut area = ScrollArea::new();
    area.update_with(VIEW, CONTENT, at(80.0, true, true));
    assert_eq!(area.offset(), 0.0, "the press alone must not move anything");

    // Up the screen, past the threshold: the content follows the finger.
    area.update_with(VIEW, CONTENT, at(60.0, true, false));
    assert!(area.offset() > 0.0, "{}", area.offset());
    let after_first = area.offset();
    area.update_with(VIEW, CONTENT, at(40.0, true, false));
    assert!(area.offset() > after_first, "{}", area.offset());
}

/// The threshold is the whole point: a tap must not scroll.
#[test]
fn a_press_that_barely_moves_is_not_a_scroll() {
    let mut area = ScrollArea::new();
    area.update_with(VIEW, CONTENT, at(80.0, true, true));
    area.update_with(VIEW, CONTENT, at(80.0 - PAN_THRESHOLD + 1.0, true, false));
    assert_eq!(area.offset(), 0.0);
    assert!(!area.absorbs_press());
}

/// The gesture and the controls under it want the same pixels, and the
/// scroll wins — a swipe that lifts over a button must not press it.
#[test]
fn a_swipe_takes_the_release_from_the_controls_underneath() {
    let mut area = ScrollArea::new();
    area.update_with(VIEW, CONTENT, at(80.0, true, true));
    area.update_with(VIEW, CONTENT, at(40.0, true, false));
    assert!(area.absorbs_press(), "mid-drag");

    // The lift itself: still absorbed, because this is the frame the button
    // underneath would otherwise fire on.
    area.update_with(VIEW, CONTENT, at(40.0, false, false));
    assert!(area.absorbs_press(), "on release");

    // And released again the next frame, or nothing would ever be clickable.
    area.update_with(VIEW, CONTENT, at(40.0, false, false));
    assert!(!area.absorbs_press(), "the frame after");
}

#[test]
fn a_tap_leaves_the_press_for_the_controls_underneath() {
    let mut area = ScrollArea::new();
    area.update_with(VIEW, CONTENT, at(80.0, true, true));
    area.update_with(VIEW, CONTENT, at(80.0, false, false));
    assert!(!area.absorbs_press());
}

#[test]
fn a_released_drag_coasts_and_then_stops() {
    let mut area = ScrollArea::new();
    area.update_with(VIEW, CONTENT, at(90.0, true, true));
    for step in 1..=4 {
        area.update_with(VIEW, CONTENT, at(90.0 - step as f32 * 20.0, true, false));
    }
    let at_release = area.offset();
    area.update_with(VIEW, CONTENT, at(10.0, false, false));
    assert!(area.offset() > at_release, "the fling should carry on");

    // And settle, rather than creeping forever.
    for _ in 0..600 {
        area.update_with(VIEW, CONTENT, at(10.0, false, false));
    }
    let settled = area.offset();
    area.update_with(VIEW, CONTENT, at(10.0, false, false));
    assert_eq!(area.offset(), settled);
}

#[test]
fn a_fling_stops_at_the_end_of_the_content() {
    let mut area = ScrollArea::new();
    area.update_with(VIEW, CONTENT, at(95.0, true, true));
    for step in 1..=8 {
        area.update_with(VIEW, CONTENT, at(95.0 - step as f32 * 11.0, true, false));
    }
    for _ in 0..600 {
        area.update_with(VIEW, CONTENT, at(5.0, false, false));
    }
    assert!(area.offset() <= ScrollArea::max_offset(VIEW, CONTENT) + 0.01);
    assert!(area.offset() >= 0.0);
}

/// Content that fits has nothing to scroll, so it must not eat presses —
/// otherwise a short list becomes unclickable for the price of a feature it
/// never uses.
#[test]
fn a_region_with_nothing_to_scroll_never_absorbs_a_press() {
    let mut area = ScrollArea::new();
    area.update_with(VIEW, 60.0, at(80.0, true, true));
    area.update_with(VIEW, 60.0, at(20.0, true, false));
    assert_eq!(area.offset(), 0.0);
    assert!(!area.absorbs_press());
}

/// A press in the right-edge gutter is aiming at the handle, and dragging
/// it must still track the handle rather than the content — the two read
/// opposite ways round.
#[test]
fn the_scrollbar_handle_still_wins_the_gutter() {
    let mut area = ScrollArea::new();
    let gutter = ScrollInput {
        pointer: vec2(VIEW.right() - 2.0, 90.0),
        down: true,
        pressed: true,
        wheel: 0.0,
        dt: 1.0 / 60.0,
    };
    area.update_with(VIEW, CONTENT, gutter);
    // Handle dragged to the bottom of the track: the end of the content,
    // where content-dragging the same way would have gone to the start.
    assert!(area.offset() > ScrollArea::max_offset(VIEW, CONTENT) * 0.5);
    assert!(area.absorbs_press());
}

#[test]
fn the_wheel_still_scrolls_for_a_mouse() {
    let mut area = ScrollArea::new();
    area.update_with(
        VIEW,
        CONTENT,
        ScrollInput {
            pointer: vec2(50.0, 50.0),
            down: false,
            pressed: false,
            wheel: -1.0,
            dt: 1.0 / 60.0,
        },
    );
    assert!(area.offset() > 0.0);
}
