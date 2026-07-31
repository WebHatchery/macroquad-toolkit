//! Unit tests for layout bounds arithmetic.

use super::*;

fn panel() -> Rect {
    Rect::new(100.0, 50.0, 400.0, 300.0)
}

#[test]
fn nothing_is_recorded_until_the_audit_starts() {
    let _ = take_audit();
    let _region = Region::new(panel());
    note("far too wide for this", 110.0, 9_000.0);
    assert!(take_audit().is_empty());
}

#[test]
fn text_that_fits_is_not_a_finding() {
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    note("short", 110.0, 60.0);
    assert!(take_audit().is_empty());
}

#[test]
fn text_past_the_edge_is_recorded_with_how_far() {
    begin_audit();
    {
        let _region = Region::new(panel());
        // 110 + 500 runs 110 past a right edge of 500.
        note("a long line", 110.0, 500.0);
    }
    let found = take_audit();
    assert_eq!(found.len(), 1);
    let Finding::Overflow {
        text, available, ..
    } = &found[0]
    else {
        panic!("expected an overflow, got {:?}", found[0]);
    };
    assert_eq!(text, "a long line");
    assert!((available - 390.0).abs() < 1e-3);
    assert!(
        found[0].describe().contains("110px"),
        "{}",
        found[0].describe()
    );
}

#[test]
fn a_hair_over_is_rounding_rather_than_a_defect() {
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    note("just about", 110.0, 391.0);
    assert!(take_audit().is_empty());
}

/// The fault §5.46 exposed and §5.47 exists for.
///
/// A title drawn straight through a button never crossed its *region's*
/// edge, so the overflow check called it clean. Overflow past a boundary and
/// collision with a sibling are different questions.
#[test]
fn text_drawn_across_a_control_is_reported() {
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    note_control("a button", Rect::new(200.0, 100.0, 100.0, 40.0));
    note_extent("a title", Rect::new(150.0, 105.0, 120.0, 30.0));

    let found = take_audit();
    assert_eq!(found.len(), 1, "{:?}", found);
    assert!(
        found[0].describe().contains("overlaps"),
        "{}",
        found[0].describe()
    );
}

#[test]
fn a_controls_own_label_is_not_a_collision() {
    // Every button draws its text inside itself. Reporting that would make
    // the check useless from the first frame.
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    note_control("a button", Rect::new(200.0, 100.0, 100.0, 40.0));
    note_extent("Press", Rect::new(220.0, 112.0, 60.0, 16.0));
    assert!(take_audit().is_empty());
}

#[test]
fn two_strings_over_one_another_are_reported() {
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    note_extent("first", Rect::new(150.0, 100.0, 100.0, 20.0));
    note_extent("second", Rect::new(200.0, 105.0, 100.0, 20.0));
    assert_eq!(take_audit().len(), 1);
}

#[test]
fn text_that_merely_touches_is_not_a_collision() {
    // Adjacent labels share an edge constantly; a hairline is rounding.
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    note_extent("left", Rect::new(150.0, 100.0, 100.0, 20.0));
    note_extent("right", Rect::new(250.0, 100.0, 100.0, 20.0));
    assert!(take_audit().is_empty());
}

/// The five findings this check produced on its first real run were all
/// this: an overlay's text "colliding" with a panel underneath that the
/// player cannot see.
#[test]
fn a_panel_hides_what_is_underneath_it() {
    begin_audit();
    begin_collision_audit();
    {
        let _behind = Region::on(panel(), Color::new(0.1, 0.1, 0.1, 1.0));
        note_extent("underneath", Rect::new(150.0, 100.0, 200.0, 20.0));
    }
    {
        // An overlay painting its own surface over the same place.
        let _over = Region::on(
            Rect::new(140.0, 90.0, 240.0, 60.0),
            Color::new(0.1, 0.1, 0.1, 1.0),
        );
        note_extent("on top", Rect::new(150.0, 105.0, 200.0, 20.0));
    }
    assert!(take_audit().is_empty());
}

#[test]
fn a_region_that_names_no_surface_hides_nothing() {
    // `Region::new` says where, not what on — it has painted nothing, so it
    // cannot be covering anything.
    begin_audit();
    begin_collision_audit();
    let _behind = Region::new(panel());
    note_extent("underneath", Rect::new(150.0, 100.0, 200.0, 20.0));
    {
        let _over = Region::new(Rect::new(140.0, 90.0, 240.0, 60.0));
        note_extent("on top", Rect::new(150.0, 105.0, 200.0, 20.0));
    }
    assert_eq!(take_audit().len(), 1);
}

#[test]
fn a_decorative_stroke_does_not_collide_with_the_label_it_serves() {
    // The meter draws its label four times in near-black behind itself to
    // keep it readable. Reporting that is reporting the fix as the fault.
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    {
        let _stroke = Decorative::new();
        note_extent("Hoard 0/15", Rect::new(150.0, 100.0, 100.0, 20.0));
    }
    note_extent("Hoard 0/15", Rect::new(151.0, 101.0, 100.0, 20.0));
    assert!(take_audit().is_empty());
}

#[test]
fn a_decorative_draw_is_not_judged_on_contrast() {
    // An outline is deliberately invisible against a dark panel. Reporting
    // it would be reporting the fix as the fault.
    begin_audit();
    begin_collision_audit();
    let _region = Region::on(panel(), Color::new(0.07, 0.06, 0.07, 1.0));
    {
        let _stroke = Decorative::new();
        note_contrast("outlined", Color::new(0.0, 0.0, 0.0, 0.75), 14.0);
    }
    assert!(take_audit().is_empty());
}

/// A modal's edge landing mid-label hides the tail of it. That is what an
/// overlay is for, and it must not be reported.
#[test]
fn a_panel_edge_that_hides_the_tail_of_a_label_is_not_a_finding() {
    let label = Rect::new(30.0, 100.0, 110.0, 20.0);
    let panel = Rect::new(90.0, 96.0, 1100.0, 500.0);
    assert!(Region::hidden_by(panel, label));
}

/// A modal's *top* edge landing inside the line severs every glyph, which
/// is what three overlays were doing to the cabinet name unnoticed.
#[test]
fn a_panel_edge_that_cuts_across_a_line_is_a_finding() {
    let title = Rect::new(26.0, 28.0, 304.0, 38.0);
    let panel = Rect::new(180.0, 44.0, 920.0, 632.0);
    assert!(!Region::hidden_by(panel, title));
    // And from below, which severs the tops of the letters instead.
    let from_below = Rect::new(180.0, -200.0, 920.0, 240.0);
    assert!(!Region::hidden_by(from_below, title));
}

#[test]
fn a_surface_that_covers_a_label_outright_hides_it() {
    let label = Rect::new(200.0, 300.0, 80.0, 18.0);
    assert!(Region::hidden_by(
        Rect::new(100.0, 200.0, 600.0, 400.0),
        label
    ));
    // And one nowhere near it neither hides nor cuts.
    assert!(!Region::hidden_by(Rect::new(0.0, 0.0, 50.0, 50.0), label));
}

#[test]
fn the_label_itself_is_still_judged() {
    // The guard covers the stroke and nothing after it.
    begin_audit();
    begin_collision_audit();
    let _region = Region::on(panel(), Color::new(0.07, 0.06, 0.07, 1.0));
    {
        let _stroke = Decorative::new();
        note_contrast("outlined", Color::new(0.0, 0.0, 0.0, 0.75), 14.0);
    }
    note_contrast("label", Color::new(0.10, 0.09, 0.10, 1.0), 14.0);
    assert_eq!(take_audit().len(), 1);
}

#[test]
fn contrast_is_not_judged_where_no_surface_was_declared() {
    // `Region::new` says where, not what on. Guessing would invent findings.
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    note_contrast("anything", Color::new(0.1, 0.1, 0.1, 1.0), 14.0);
    assert!(take_audit().is_empty());
}

#[test]
fn a_draw_outside_every_region_is_not_a_finding() {
    // The reels and the celebration cards paint the whole screen on purpose.
    begin_audit();
    note("full bleed", 0.0, 5_000.0);
    assert!(take_audit().is_empty());
}

#[test]
fn the_innermost_region_is_the_one_that_counts() {
    begin_audit();
    let _outer = Region::new(panel());
    {
        let _inner = Region::new(Rect::new(100.0, 50.0, 120.0, 40.0));
        note("inside the column", 110.0, 200.0);
    }
    // And the outer bound applies again once the inner one is gone.
    note("inside the panel", 110.0, 200.0);

    let found = take_audit();
    assert_eq!(found.len(), 1, "{:?}", found);
    assert_eq!(found[0].text(), "inside the column");
}

#[test]
fn a_region_is_popped_even_when_the_panel_returns_early() {
    // Half the panels in a game bail out when they have nothing to show. A
    // stack that leaked would bound every later draw by a dead panel.
    fn draws_nothing() {
        let _region = Region::new(panel());
        #[allow(clippy::needless_return)]
        return;
    }
    let before = STATE.with(|state| state.borrow().stack.len());
    draws_nothing();
    assert_eq!(STATE.with(|state| state.borrow().stack.len()), before);
    assert!(current().is_none());
}

#[test]
fn an_inset_region_is_clipped_to_the_one_around_it() {
    let _outer = Region::new(panel());
    // Asking for more room than the panel has must not grant it.
    let _inner = Region::inset(Rect::new(100.0, 50.0, 9_000.0, 40.0));
    assert!((current().unwrap().right() - panel().right()).abs() < 1e-3);
}

#[test]
fn an_inset_with_no_region_around_it_stands_alone() {
    let _ = take_audit();
    let rect = Rect::new(0.0, 0.0, 80.0, 20.0);
    let _inner = Region::inset(rect);
    assert!((current().unwrap().w - 80.0).abs() < 1e-3);
}

#[test]
fn the_same_overflow_is_reported_once_rather_than_every_frame() {
    // The audit runs while the game is drawing, which redraws everything
    // sixty times a second.
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    for _ in 0..60 {
        note("a long line", 110.0, 500.0);
    }
    assert_eq!(take_audit().len(), 1);
}

#[test]
fn taking_the_audit_stops_it() {
    begin_audit();
    begin_collision_audit();
    let _region = Region::new(panel());
    note("a long line", 110.0, 500.0);
    assert_eq!(take_audit().len(), 1);

    note("another long line", 110.0, 500.0);
    assert!(take_audit().is_empty());
    assert!(!auditing());
}
