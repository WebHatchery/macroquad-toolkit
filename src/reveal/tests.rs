use super::*;

#[test]
fn a_countup_starts_at_nothing_and_arrives_at_the_target() {
    let mut count = Countup::new(1_000, 0.5, 1.0);
    assert_eq!(count.value(), 0);
    assert!(!count.finished());

    assert!(count.tick(0.5), "the tick that completes it says so");
    assert_eq!(count.value(), 1_000);
    assert!(count.finished());
}

/// Eased, not linear: halfway through the time is past halfway to the
/// number. A linear count reads as a progress bar rather than a payout.
#[test]
fn a_countup_is_fastest_at_the_start() {
    let mut count = Countup::new(1_000, 1.0, 1.0);
    count.tick(0.5);
    assert!(
        count.value() > 500,
        "halfway through the time it had only reached {}",
        count.value()
    );
    assert!(count.value() < 1_000);
}

/// The failure this guards: a scale of zero is a timer that never finishes,
/// so the number would sit at nothing forever.
#[test]
fn a_countup_survives_a_nonsense_scale() {
    for scale in [0.0, -3.0, 1e9] {
        let mut count = Countup::new(500, 0.5, scale);
        for _ in 0..200 {
            count.tick(0.1);
        }
        assert!(count.finished(), "scale {} never finished", scale);
        assert_eq!(count.value(), 500);
    }
}

/// A big target must not overflow on the way. `i64::MAX` through an `f32`
/// multiply was the shape of it.
#[test]
fn a_countup_handles_a_large_target() {
    let mut count = Countup::new(9_000_000_000, 0.2, 1.0);
    count.tick(0.2);
    assert_eq!(count.value(), 9_000_000_000);
}

/// Exactly once, however many ticks arrive after it.
///
/// The return value is what makes a noise or fires an event, so a counter
/// that kept saying "finished" on every later frame would play the payout
/// sound forever.
#[test]
fn a_countup_reports_finishing_exactly_once() {
    let mut count = Countup::new(10, 0.5, 1.0);
    let mut finishes = 0;
    for _ in 0..200 {
        if count.tick(1.0 / 60.0) {
            finishes += 1;
        }
    }
    assert_eq!(finishes, 1);
}

#[test]
fn a_stepper_visits_every_stage_in_order() {
    let mut stepper = Stepper::new(4, 0.5, 1.0);
    let mut visited = vec![stepper.step()];
    for _ in 0..40 {
        if stepper.tick(0.1) {
            visited.push(stepper.step());
        }
    }
    assert_eq!(visited, vec![0, 1, 2, 3]);
}

/// The distinction the doc comment is about: the last stage gets a beat of
/// its own before the sequence is over.
#[test]
fn the_last_stage_is_held_before_the_sequence_ends() {
    let mut stepper = Stepper::new(2, 0.5, 1.0);
    assert!(stepper.tick(0.5), "moved to the last stage");
    assert_eq!(stepper.step(), 1);
    assert!(
        !stepper.finished(),
        "the last stage was over the moment it was reached, so nobody saw it"
    );

    assert!(!stepper.tick(0.5), "no new stage, but the beat has passed");
    assert!(stepper.finished());
}

/// An empty sequence is over before it starts, so callers need no special
/// case for it.
#[test]
fn a_stepper_with_nothing_to_show_is_already_finished() {
    let mut stepper = Stepper::new(0, 0.5, 1.0);
    assert!(stepper.finished());
    assert!(!stepper.tick(10.0));
    assert_eq!(stepper.step(), 0);
}

/// One stage is shown once and then done — the degenerate case either side
/// of the one above.
#[test]
fn a_stepper_with_one_stage_shows_it_and_stops() {
    let mut stepper = Stepper::new(1, 0.5, 1.0);
    assert!(!stepper.finished());
    assert!(!stepper.tick(0.5), "there is no second stage to move to");
    assert!(stepper.finished());
    assert_eq!(stepper.step(), 0);
}
