use super::*;

fn chime() -> Vec<Voice> {
    vec![
        Voice::tone(0.0, 0.10, 1568.0, 0.34).wave(Wave::Triangle),
        Voice::tone(0.0, 0.16, 2349.0, 0.18).wave(Wave::Triangle),
        Voice::tone(0.0, 0.03, 3200.0, 0.14).wave(Wave::Noise),
    ]
}

#[test]
fn the_container_is_a_well_formed_wav() {
    let config = SynthConfig::default();
    let bytes = render_wav(&chime(), &config, 7);

    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(&bytes[36..40], b"data");

    // PCM, mono, at the configured rate and 16 bits.
    assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 1);
    assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1);
    assert_eq!(
        u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
        config.sample_rate
    );
    assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 16);
}

#[test]
fn the_declared_sizes_match_the_real_payload() {
    // A player that trusted the header over the buffer would read past the
    // end, and the failure would look like a corrupt file rather than a
    // arithmetic slip here.
    let bytes = render_wav(&chime(), &SynthConfig::default(), 7);

    let riff = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let data = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]) as usize;
    assert_eq!(riff, bytes.len() - 8);
    assert_eq!(data, bytes.len() - 44);
}

#[test]
fn synthesis_is_deterministic() {
    let config = SynthConfig::default();
    assert_eq!(
        render_wav(&chime(), &config, 99),
        render_wav(&chime(), &config, 99)
    );
}

#[test]
fn a_different_seed_changes_only_the_noise() {
    // Tonal voices must not move with the seed, or a game could not tune one
    // effect without disturbing the rest.
    let config = SynthConfig::default();
    let tonal = vec![Voice::tone(0.0, 0.2, 440.0, 0.5)];
    assert_eq!(
        render_wav(&tonal, &config, 1),
        render_wav(&tonal, &config, 2)
    );

    let noisy = vec![Voice::tone(0.0, 0.2, 440.0, 0.5).wave(Wave::Noise)];
    assert_ne!(
        render_wav(&noisy, &config, 1),
        render_wav(&noisy, &config, 2)
    );
}

#[test]
fn nothing_clips() {
    // The limiter exists, but reaching it means the mix is already crunchy.
    let wave = render_waveform(&chime(), &SynthConfig::default(), 7);
    assert!(wave.iter().all(|sample| sample.abs() < 0.999));
}

#[test]
fn an_envelope_opens_and_decays_to_silence() {
    let voice = Voice::tone(0.0, 1.0, 440.0, 1.0).attack(0.25);

    assert!(voice.envelope(0.0) < 0.01);
    assert!((voice.envelope(0.25) - 1.0).abs() < 0.01);
    assert!(voice.envelope(0.5) < 1.0);
    assert!(voice.envelope(1.0) < 0.01);
}

#[test]
fn a_glide_is_geometric() {
    // The midpoint of a sweep is the geometric mean, not the arithmetic one.
    // Linear would spend most of the sweep in the top octave and lurch.
    let voice = Voice::tone(0.0, 1.0, 100.0, 1.0).glide(1600.0);
    assert!((voice.frequency(0.0) - 100.0).abs() < 0.1);
    assert!((voice.frequency(0.5) - 400.0).abs() < 1.0);
    assert!((voice.frequency(1.0) - 1600.0).abs() < 1.0);
}

#[test]
fn an_effect_is_as_long_as_its_last_voice() {
    let config = SynthConfig::default();
    let voices = vec![
        Voice::tone(0.0, 0.1, 440.0, 0.5),
        Voice::tone(0.4, 0.2, 880.0, 0.5),
    ];
    let expected = (0.6 * config.sample_rate as f32).ceil() as usize;
    assert_eq!(render_waveform(&voices, &config, 1).len(), expected);
}

#[test]
fn an_empty_effect_still_renders_a_valid_file() {
    // Rather than a zero-length buffer a player would reject.
    let bytes = render_wav(&[], &SynthConfig::default(), 1);
    assert_eq!(&bytes[0..4], b"RIFF");
    assert!(bytes.len() > 44);
}
