//! Regression tests for pointer self-occlusion.

use super::*;

/// The fault, reproduced: a button declaring a region over itself used to
/// erase itself from the neighbour list, every button, every frame.
#[test]
fn a_button_does_not_hide_itself() {
    let button = Rect::new(100.0, 200.0, 108.0, 28.0);
    begin_target_audit();
    begin_target_frame();
    note_neighbour(button);
    note_target("button", button);

    // Exactly what `virtual_button` does after drawing its fill.
    occlude(button);

    end_frame_neighbours();
    assert!(
        neighbours_warm(),
        "a button erased itself from the neighbour list, so nothing is ever warm"
    );
    assert!(
        !overlapping_targets().is_empty() || smallest_touchable_width(1280.0).is_some(),
        "the button also vanished from the target audit"
    );
}

/// And the rule it must not break: a panel painted over a control really
/// does hide it, which is why occlusion exists at all.
#[test]
fn a_panel_still_hides_what_is_under_it() {
    let button = Rect::new(100.0, 200.0, 108.0, 28.0);
    let panel = Rect::new(0.0, 0.0, 640.0, 480.0);
    begin_target_audit();
    begin_target_frame();
    note_neighbour(button);
    note_target("button", button);

    occlude(panel);

    end_frame_neighbours();
    assert!(
        !neighbours_warm(),
        "a panel covering a control left it in the neighbour list"
    );
}

/// Half a pixel of rounding is the same rectangle; ten is not.
#[test]
fn the_same_rectangle_is_recognised_through_rounding() {
    let a = Rect::new(10.0, 20.0, 100.0, 40.0);
    assert!(is_the_same(a, Rect::new(10.2, 19.8, 100.1, 40.2)));
    assert!(!is_the_same(a, Rect::new(10.0, 20.0, 110.0, 40.0)));
}
