use super::*;

#[test]
fn cooldown_cycle() {
    let mut cd = Cooldown::new(1.0);
    assert!(cd.is_ready());
    assert!(cd.try_trigger());
    assert!(!cd.is_ready());
    assert!((cd.fraction_remaining() - 1.0).abs() < 1e-6);
    cd.tick(0.4);
    assert!((cd.fraction_elapsed() - 0.4).abs() < 1e-6);
    cd.tick(0.7);
    assert!(cd.is_ready());
}

#[test]
fn armed_cooldown_starts_unready() {
    let cd = Cooldown::new_armed(2.0);
    assert!(!cd.is_ready());
}

#[test]
fn timer_reports_finish_once() {
    let mut timer = Timer::new(1.0);
    assert!(!timer.tick(0.5));
    assert!((timer.progress() - 0.5).abs() < 1e-6);
    assert!(timer.tick(0.6));
    assert!(timer.finished());
    assert!(!timer.tick(0.1), "already-finished tick must not re-fire");
}

#[test]
fn zero_duration_timer_is_finished() {
    let timer = Timer::new(0.0);
    assert!(timer.finished());
    assert!((timer.progress() - 1.0).abs() < 1e-6);
}

#[test]
fn interval_timer_accumulates_fractions() {
    let mut ticker = IntervalTimer::new(0.1);
    assert_eq!(ticker.tick(0.05), 0);
    assert_eq!(ticker.tick(0.05), 1);
    assert_eq!(ticker.tick(0.35), 3);
    // Leftover 0.05 carries into the next tick.
    assert_eq!(ticker.tick(0.05), 1);
}

#[test]
fn timeline_walks_phases() {
    let mut timeline = Timeline::new(vec![("a", 1.0), ("b", 2.0)]);
    let (phase, progress) = timeline.current().unwrap();
    assert_eq!(*phase, "a");
    assert!(progress.abs() < 1e-6);

    timeline.advance(1.5);
    let (phase, progress) = timeline.current().unwrap();
    assert_eq!(*phase, "b");
    assert!((progress - 0.25).abs() < 1e-6);

    timeline.advance(10.0);
    assert!(timeline.finished());
    assert!(timeline.current().is_none());
    assert!((timeline.overall_progress() - 1.0).abs() < 1e-6);
}

#[test]
fn empty_timeline_is_finished() {
    let timeline: Timeline<&str> = Timeline::new(vec![]);
    assert!(timeline.finished());
    assert!(timeline.current().is_none());
}
