use super::*;

fn feel() -> StripFeel {
    StripFeel::default()
}

fn run(strip: &mut StripAnimation) {
    for _ in 0..900 {
        strip.tick(1.0 / 60.0);
    }
}

#[test]
fn a_strip_lands_exactly_on_its_target() {
    // Every index, deliberately: the bug this guards against only showed on
    // odd ones.
    for index in 0..6 {
        for target in [0usize, 1, 17, 39] {
            let mut strip = StripAnimation::new(40, 7, target, index, 1.0, false, &feel());
            run(&mut strip);

            assert!(strip.settled());
            assert_eq!(
                strip.position().round() as usize % 40,
                target,
                "strip {} missed its stop",
                index
            );
        }
    }
}

#[test]
fn every_strip_travels_a_whole_number_of_revolutions() {
    for index in 0..6 {
        let strip = StripAnimation::new(40, 7, 7, index, 1.0, false, &feel());
        assert_eq!(
            strip.travel_symbols() % strip.strip_len(),
            0.0,
            "strip {} travels a fractional strip",
            index
        );
    }
}

#[test]
fn a_strip_lands_on_target_from_a_ragged_frame_rate() {
    let mut strip = StripAnimation::new(40, 3, 22, 0, 1.0, false, &feel());
    for dt in [
        0.004, 0.1, 0.017, 0.05, 0.2, 0.033, 0.4, 0.016, 0.12, 0.008, 0.25, 0.031,
    ] {
        strip.tick(dt);
    }
    assert!(strip.settled());
    assert_eq!(strip.position().round() as usize % 40, 22);
}

#[test]
fn the_bounce_never_changes_where_a_strip_stops() {
    for target in [0usize, 3, 19, 39] {
        let mut strip = StripAnimation::new(40, 11, target, 1, 1.0, false, &feel());
        run(&mut strip);
        assert_eq!(strip.position().round() as usize % 40, target);
    }
}

#[test]
fn a_strip_that_does_not_move_still_turns_a_full_revolution() {
    let strip = StripAnimation::new(40, 12, 12, 0, 1.0, false, &feel());
    assert!(strip.travel_symbols() >= 40.0);
}

#[test]
fn a_held_strip_takes_longer_and_still_lands_right() {
    let plain = StripAnimation::new(40, 0, 19, 2, 1.0, false, &feel());
    let mut held = StripAnimation::new(40, 0, 19, 2, 1.0, true, &feel());
    assert!(held.progress() <= plain.progress());

    run(&mut held);
    assert!(held.is_held());
    assert_eq!(held.position().round() as usize % 40, 19);
}

#[test]
fn strips_report_stopping_once_each_in_order() {
    let mut spinner = StripSpinner::new(&[40; 5], &[0; 5], &[5; 5], 1.0, &[false; 5], &feel());
    let mut order = Vec::new();
    for _ in 0..900 {
        order.extend(spinner.tick(1.0 / 60.0));
    }

    assert_eq!(order, vec![0, 1, 2, 3, 4]);
    assert!(spinner.all_settled());
    assert!(spinner.tick(1.0).is_empty(), "a stop was announced twice");
}

#[test]
fn no_time_scale_drops_a_stop() {
    // Compressing the durations far enough that two strips land in one
    // frame would lose a notification, and a caller counting them would
    // hang waiting for the last.
    for scale in [0.05, 0.25, 1.0, 4.0] {
        let mut spinner =
            StripSpinner::new(&[40; 5], &[0; 5], &[9; 5], scale, &[false; 5], &feel());
        let mut seen = Vec::new();
        for _ in 0..2_000 {
            seen.extend(spinner.tick(1.0 / 60.0));
        }
        assert_eq!(seen, vec![0, 1, 2, 3, 4], "scale {}", scale);
    }
}

#[test]
fn speed_falls_to_nothing_as_a_strip_settles() {
    let mut strip = StripAnimation::new(40, 0, 20, 0, 1.0, false, &feel());
    let opening = strip.speed();
    for _ in 0..30 {
        strip.tick(1.0 / 60.0);
    }
    assert!(strip.speed() < opening);
    run(&mut strip);
    assert_eq!(strip.speed(), 0.0);
}

#[test]
fn blur_offsets_straddle_the_position_and_sum_to_nothing() {
    let offsets = blur_offsets(0.8, 5);
    assert_eq!(offsets.len(), 5);
    assert!(offsets.iter().sum::<f32>().abs() < 1e-5);
    assert!(offsets[0] < 0.0 && offsets[4] > 0.0);

    // One pass is no smear at all.
    assert_eq!(blur_offsets(0.8, 1), vec![0.0]);
}
