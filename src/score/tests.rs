use super::*;

const A_MINOR: Scale = Scale::new(0, MINOR);

fn timing() -> Timing {
    Timing {
        bpm: 96.0,
        beats_per_bar: 4,
        bars: 4,
    }
}

#[test]
fn the_reference_pitch_is_where_it_should_be() {
    assert!((semitone_hz(0) - 440.0).abs() < 1e-3);
    assert!((semitone_hz(12) - 880.0).abs() < 1e-2);
    assert!((semitone_hz(-12) - 220.0).abs() < 1e-2);
}

#[test]
fn degrees_walk_the_scale_in_both_directions() {
    assert_eq!(A_MINOR.semitones(0), 0);
    assert_eq!(A_MINOR.semitones(1), 2);
    assert_eq!(A_MINOR.semitones(7), 12);
    // Downwards must step below the root rather than reflect above it.
    assert_eq!(A_MINOR.semitones(-1), -2);
    assert_eq!(A_MINOR.semitones(-7), -12);
}

#[test]
fn a_scale_recognises_its_own_notes_and_rejects_the_rest() {
    for degree in -14..14 {
        assert!(A_MINOR.contains_semitone(A_MINOR.semitones(degree)));
    }
    // The minor second and the tritone are in no natural minor scale.
    assert!(!A_MINOR.contains_semitone(1));
    assert!(!A_MINOR.contains_semitone(6));
}

#[test]
fn timing_is_consistent_with_itself() {
    let timing = timing();
    assert_eq!(timing.beats(), 16.0);
    assert!((timing.seconds() - 10.0).abs() < 1e-3);
    let config = SynthConfig::default();
    assert_eq!(
        timing.samples(&config),
        (timing.seconds() * config.sample_rate as f32).round() as usize
    );
}

#[test]
fn every_track_renders_to_exactly_one_loop() {
    // The whole reason vertical remixing works. A track whose last bar is
    // empty renders short from the synth and would drift out of phase.
    let config = SynthConfig::default();
    let timing = timing();
    let timbre = Timbre::new(Wave::Sine, 0.5);

    let full = [Note::new(0.0, 0, 1.0), Note::new(15.0, 2, 1.0)];
    let sparse = [Note::new(0.0, 0, 1.0)];
    let silent: [Note; 0] = [];

    for notes in [&full[..], &sparse[..], &silent[..]] {
        let voices = lay(notes, &A_MINOR, &timing, &timbre);
        let samples = render_track(&voices, &config, &timing, 1);
        assert_eq!(samples.len(), timing.samples(&config));
    }
}

#[test]
fn a_note_past_the_end_of_the_loop_is_dropped() {
    let timing = timing();
    let timbre = Timbre::new(Wave::Sine, 0.5);
    let voices = lay(&[Note::new(64.0, 0, 1.0)], &A_MINOR, &timing, &timbre);
    assert!(voices.is_empty());
}

#[test]
fn a_note_hanging_over_the_end_is_truncated_rather_than_dropped() {
    // Dropping it would leave a hole in the final bar, which is the most
    // exposed place in a loop.
    let timing = timing();
    let timbre = Timbre::new(Wave::Sine, 0.5);
    let voices = lay(&[Note::new(15.0, 0, 8.0)], &A_MINOR, &timing, &timbre);
    assert_eq!(voices.len(), 1);
    assert!(voices[0].start() + voices[0].duration() <= timing.seconds() + 1e-4);
    assert!(voices[0].duration() > 0.0);
}

#[test]
fn sustain_leaves_a_gap_before_the_next_note() {
    let timing = timing();
    let legato = Timbre::new(Wave::Sine, 0.5).sustain(1.0);
    let staccato = Timbre::new(Wave::Sine, 0.5).sustain(0.4);
    let note = [Note::new(0.0, 0, 1.0)];

    let long = lay(&note, &A_MINOR, &timing, &legato)[0].duration();
    let short = lay(&note, &A_MINOR, &timing, &staccato)[0].duration();
    assert!(short < long);
}

#[test]
fn peak_and_seam_measure_what_they_say() {
    assert!((peak(&[0.1, -0.7, 0.3]) - 0.7).abs() < 1e-6);
    assert!((seam(&[0.5, 0.0, 0.2]) - 0.3).abs() < 1e-6);
    assert_eq!(seam(&[]), 0.0);
    assert_eq!(peak(&[]), 0.0);
}

#[test]
fn mixing_finds_the_clip_that_no_single_track_shows() {
    // Each track is comfortable alone; together they are over the top.
    let a = vec![0.6f32; 8];
    let b = vec![0.6f32; 8];
    assert!(peak(&a) < 1.0);
    assert!(peak(&b) < 1.0);
    assert!(mixed_peak(&[(&a, 1.0), (&b, 1.0)]) > 1.0);
    // And gains are respected, so a quiet layer is not counted at full.
    assert!(mixed_peak(&[(&a, 0.5), (&b, 0.5)]) < 1.0);
}

#[test]
fn mixing_handles_tracks_of_different_lengths() {
    let long = vec![0.4f32; 16];
    let short = vec![0.4f32; 4];
    // The short one contributes nothing past its end rather than panicking.
    assert!((mixed_peak(&[(&long, 1.0), (&short, 1.0)]) - 0.8).abs() < 1e-6);
}

#[test]
fn a_rendered_track_stays_inside_the_rails() {
    let config = SynthConfig::default();
    let timing = timing();
    let timbre = Timbre::new(Wave::Triangle, 0.6);
    let notes: Vec<Note> = (0..16).map(|i| Note::new(i as f32, i % 7, 1.0)).collect();
    let samples = render_track(
        &lay(&notes, &A_MINOR, &timing, &timbre),
        &config,
        &timing,
        7,
    );

    assert!(peak(&samples) <= 1.0);
    assert!(samples.iter().all(|s| s.is_finite()));
}
