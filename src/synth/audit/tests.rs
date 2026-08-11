use super::*;

fn config() -> SynthConfig {
    SynthConfig::default()
}

/// The transform itself, against a signal whose answer is known by hand.
#[test]
fn a_pure_tone_lands_in_one_bin() {
    let rate = 22_050u32;
    let frame = 1024;
    // Exactly 64 cycles in the frame, so it sits on bin 64 with no leakage.
    let hz = 64.0 * rate as f32 / frame as f32;
    let samples: Vec<f32> = (0..frame)
        .map(|i| (std::f32::consts::TAU * hz * i as f32 / rate as f32).sin())
        .collect();
    let bins = spectrum(&samples, 0, frame);

    let loudest = bins
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(k, _)| k)
        .unwrap();
    assert_eq!(loudest, 64, "a tone at bin 64 was found at {}", loudest);
    // And nothing anywhere else: a bin two away should be far down.
    assert!(bins[70] < bins[64] * 0.01);
}

#[test]
fn the_transform_is_reversible_enough_to_trust() {
    // Parseval: the energy in the bins matches the energy in the samples.
    let frame = 256;
    let samples: Vec<f32> = (0..frame)
        .map(|i| ((i * 7) % 13) as f32 / 13.0 - 0.5)
        .collect();
    let mut re = samples.clone();
    let mut im = vec![0.0; frame];
    fft(&mut re, &mut im);
    let time: f32 = samples.iter().map(|s| s * s).sum();
    let freq: f32 = re
        .iter()
        .zip(&im)
        .map(|(r, i)| (r * r + i * i) / frame as f32)
        .sum();
    assert!((time - freq).abs() / time < 0.001, "{} vs {}", time, freq);
}

/// A sine has no partials to fold, at any pitch.
#[test]
fn a_sine_never_aliases() {
    for hz in [200.0, 1200.0, 4000.0, 9000.0] {
        assert!(folded_partials(hz, Wave::Sine, 22_050).is_empty(), "{}", hz);
        let voice = Voice::tone(0.0, 0.3, hz, 0.5);
        assert!(aliasing(&voice, &config(), -60.0).is_empty(), "{}", hz);
    }
}

/// The disproof: hand-build the naive oscillator this toolkit used to have
/// and check the measurement finds its reflections.
///
/// Without this the audit could be broken rather than the sound clean, and
/// the two look identical from the outside. A phase-threshold square at
/// 1200Hz in a 22kHz stream must show energy where `folded_partials` says.
#[test]
fn the_measurement_finds_a_naive_oscillator_folding() {
    let rate = 22_050u32;
    let hz = 1200.0f32;
    let naive: Vec<f32> = (0..4096)
        .map(|i| {
            let phase = (hz * i as f32 / rate as f32).fract();
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        })
        .collect();
    let bins = spectrum(&naive, 0, 2048);
    let hz_per_bin = rate as f32 / 2048.0;
    let fundamental = magnitude_near(&bins, hz, hz_per_bin);

    let folds = folded_partials(hz, Wave::Square, rate);
    assert!(!folds.is_empty(), "1200Hz must fold in a 22kHz stream");
    let loudest = folds
        .iter()
        .map(|&(_, folded, _)| {
            20.0 * (magnitude_near(&bins, folded, hz_per_bin) / fundamental)
                .max(1e-9)
                .log10()
        })
        .fold(f32::MIN, f32::max);
    assert!(
        loudest > -32.0,
        "a naive square's loudest reflection measured {:.0}dB under the              note — the detector cannot see aliasing that is certainly there",
        loudest
    );
}

/// And the oscillator this toolkit actually has does not fold at all.
///
/// Same pitch, same shape, same measurement as the test above. The only
/// difference is that the partials above Nyquist are never generated.
#[test]
fn the_band_limited_oscillator_does_not_fold() {
    let voice = Voice::tone(0.0, 0.5, 1200.0, 0.9)
        .wave(Wave::Square)
        .attack(0.5);
    let measured = measured_aliasing(&voice, &config());
    assert!(!measured.is_empty(), "nothing was predicted to measure");
    let loudest = measured.iter().map(|&(_, db)| db).fold(f32::MIN, f32::max);
    assert!(
        loudest < -60.0,
        "the loudest reflection is {:.0}dB under the note; band-limiting              should leave nothing there at all",
        loudest
    );
    assert!(aliasing(&voice, &config(), -32.0).is_empty());
}

/// A low square keeps every partial under Nyquist and is clean.
#[test]
fn a_low_square_has_room_for_its_partials() {
    assert!(aliasing(
        &Voice::tone(0.0, 0.3, 90.0, 0.5).wave(Wave::Square),
        &config(),
        -40.0
    )
    .is_empty());
}

#[test]
fn measuring_finds_the_shape_of_a_buffer() {
    let flat = vec![0.5f32; 100];
    let m = measure(&flat, 100);
    assert!((m.peak - 0.5).abs() < 1e-6);
    assert!((m.dc_offset - 0.5).abs() < 1e-6, "a constant is all offset");
    assert_eq!(m.clipped, 0);

    let hot = vec![1.4f32; 10];
    assert_eq!(measure(&hot, 100).clipped, 10);

    let mut stepped = vec![0.0f32; 50];
    stepped[25] = 0.9;
    assert!(measure(&stepped, 100).worst_step >= 0.9);
}
