//! Writing music for [`crate::synth`] to render.
//!
//! # Why this is separate from the synth
//!
//! `synth` knows about tones: a frequency, an envelope, a waveform. That is the
//! right vocabulary for a blip and the wrong one for eight bars of anything —
//! writing a bass line as a list of hertz values and offsets in seconds means
//! every edit is arithmetic, and a wrong note is indistinguishable from a typo.
//!
//! So this module adds the layer above: a scale, a tempo, and notes placed on
//! beats by degree. [`lay`] turns those into the [`Voice`](crate::synth::Voice)
//! list the synth already renders, and nothing in `synth` had to change.
//!
//! # Loops that can be layered
//!
//! The intended use is **vertical remixing**: several tracks, all the same
//! length, all started together and left running, with only their volumes moving
//! as the game changes. It costs nothing at runtime and it cannot glitch,
//! because no sound is ever started or stopped in response to gameplay.
//!
//! That only works if the tracks stay in phase forever, which is why
//! [`Timing::samples`] exists and why a caller is expected to assert every track
//! renders to exactly that many. One sample of drift per loop is inaudible on
//! the first pass and a disaster on the fiftieth.
//!
//! [`seam`] and [`peak`] are the other two things that go wrong and cannot be
//! heard until they already have: a loop that does not meet itself clicks once
//! per repeat, and four tracks that are each fine on their own clip when summed.

use crate::synth::{render_waveform, SynthConfig, Voice, Wave};

/// Semitones above A4 (440 Hz), the reference every pitch here is derived from.
pub fn semitone_hz(semitones: i32) -> f32 {
    440.0 * 2.0f32.powf(semitones as f32 / 12.0)
}

/// A key: where it starts and which steps it takes.
///
/// Held as intervals rather than a note list so a degree above the octave keeps
/// working — degree 7 is the root an octave up, degree -1 the seventh below,
/// and a melody can run off either end without special-casing.
#[derive(Debug, Clone, Copy)]
pub struct Scale {
    /// Semitones from A4 to the tonic. Negative goes down, which is where most
    /// of a bass line lives.
    pub root: i32,
    pub steps: &'static [i32],
}

/// The natural minor scale. Dark by default, which suits most game music that
/// is not trying to be cheerful.
pub const MINOR: &[i32] = &[0, 2, 3, 5, 7, 8, 10];
/// The major scale.
pub const MAJOR: &[i32] = &[0, 2, 4, 5, 7, 9, 11];
/// Minor pentatonic — the five degrees that cannot clash with each other, which
/// is what an arpeggio over a changing bass wants.
pub const PENTATONIC_MINOR: &[i32] = &[0, 3, 5, 7, 10];

impl Scale {
    pub const fn new(root: i32, steps: &'static [i32]) -> Self {
        Self { root, steps }
    }

    /// Semitones above A4 for a scale degree, wrapping into octaves.
    pub fn semitones(&self, degree: i32) -> i32 {
        let len = self.steps.len() as i32;
        // `rem_euclid` so a negative degree walks *down* the scale rather than
        // reflecting: degree -1 must be the step below the root, not above it.
        let octave = degree.div_euclid(len);
        let step = self.steps[degree.rem_euclid(len) as usize];
        self.root + octave * 12 + step
    }

    pub fn hz(&self, degree: i32) -> f32 {
        semitone_hz(self.semitones(degree))
    }

    /// Every pitch class this scale contains, as semitones from its tonic.
    /// What a test uses to prove a written part stays in key.
    pub fn contains_semitone(&self, semitone: i32) -> bool {
        let relative = (semitone - self.root).rem_euclid(12);
        self.steps
            .iter()
            .any(|step| step.rem_euclid(12) == relative)
    }
}

/// Tempo and length. A loop is always a whole number of bars.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub bpm: f32,
    pub beats_per_bar: u32,
    pub bars: u32,
}

impl Timing {
    pub fn seconds_per_beat(&self) -> f32 {
        60.0 / self.bpm
    }

    pub fn beats(&self) -> f32 {
        (self.beats_per_bar * self.bars) as f32
    }

    pub fn seconds(&self) -> f32 {
        self.beats() * self.seconds_per_beat()
    }

    /// Samples in one loop, and the number every track must render to.
    ///
    /// Rounded once, here, rather than derived independently per track — two
    /// tracks that each rounded their own length could differ by a sample and
    /// drift apart over a few minutes of play.
    pub fn samples(&self, config: &SynthConfig) -> usize {
        ((self.seconds() * config.sample_rate as f32).round() as usize).max(1)
    }
}

/// One note, placed by beat and pitched by scale degree.
#[derive(Debug, Clone, Copy)]
pub struct Note {
    pub beat: f32,
    pub degree: i32,
    /// Octaves up or down from where the scale's root sits.
    pub octave: i32,
    pub beats: f32,
    pub gain: f32,
}

impl Note {
    pub const fn new(beat: f32, degree: i32, beats: f32) -> Self {
        Self {
            beat,
            degree,
            octave: 0,
            beats,
            gain: 1.0,
        }
    }

    pub const fn octave(mut self, octave: i32) -> Self {
        self.octave = octave;
        self
    }

    pub const fn gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }
}

/// How a track is played, over and above its notes.
#[derive(Debug, Clone, Copy)]
pub struct Timbre {
    pub wave: Wave,
    pub gain: f32,
    /// Fraction of the note spent rising. A long attack is what makes a pad a
    /// pad rather than a stab.
    pub attack: f32,
    /// Fraction of a note's length actually sounded. Below 1.0 leaves a gap
    /// before the next note, which is the difference between a line that
    /// articulates and one that smears.
    pub sustain: f32,
}

impl Timbre {
    pub const fn new(wave: Wave, gain: f32) -> Self {
        Self {
            wave,
            gain,
            attack: 0.02,
            sustain: 0.9,
        }
    }

    pub const fn attack(mut self, attack: f32) -> Self {
        self.attack = attack;
        self
    }

    pub const fn sustain(mut self, sustain: f32) -> Self {
        self.sustain = sustain;
        self
    }
}

/// Turn written notes into voices the synth can render.
///
/// A note that would sound past the end of the loop is **truncated rather than
/// dropped**: a pad held across the final bar line should still fill the bar it
/// was written in, and dropping it would leave an audible hole exactly where the
/// loop is most exposed.
pub fn lay(notes: &[Note], scale: &Scale, timing: &Timing, timbre: &Timbre) -> Vec<Voice> {
    let spb = timing.seconds_per_beat();
    let total = timing.seconds();

    notes
        .iter()
        .filter_map(|note| {
            let start = note.beat * spb;
            if start >= total {
                return None;
            }
            let full = note.beats * spb * timbre.sustain;
            let duration = full.min(total - start);
            if duration <= 0.0 {
                return None;
            }
            Some(
                Voice::tone(
                    start,
                    duration,
                    scale.hz(note.degree) * 2.0f32.powi(note.octave),
                    note.gain * timbre.gain,
                )
                .wave(timbre.wave)
                .attack(duration * timbre.attack),
            )
        })
        .collect()
}

/// Render a track to exactly `timing.samples()` samples.
///
/// The synth sizes its buffer from the last voice that sounds, which for a track
/// whose final bar is silent is short. Padding here rather than asking every
/// caller to remember it is what keeps a set of tracks in phase.
pub fn render_track(
    voices: &[Voice],
    scale_config: &SynthConfig,
    timing: &Timing,
    seed: u64,
) -> Vec<f32> {
    let mut samples = render_waveform(voices, scale_config, seed);
    samples.resize(timing.samples(&SynthConfig { ..*scale_config }), 0.0);
    samples
}

/// How far the loop is from meeting itself, in amplitude.
///
/// A loop whose last sample is nowhere near its first steps discontinuously
/// every time it wraps, which is heard as a click once per repeat — quiet enough
/// to miss on the first pass and impossible to ignore after five minutes.
pub fn seam(samples: &[f32]) -> f32 {
    match (samples.first(), samples.last()) {
        (Some(first), Some(last)) => (first - last).abs(),
        _ => 0.0,
    }
}

/// Loudest sample in a track, ignoring sign.
pub fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |worst, s| worst.max(s.abs()))
}

/// Loudest sample of several tracks played together at the given gains.
///
/// The failure this exists for: four tracks that each peak at a comfortable 0.6
/// sum to 2.4 and clip into a buzz the moment they all play at once — which is
/// the one arrangement nobody auditions, because it only happens in the game.
pub fn mixed_peak(tracks: &[(&[f32], f32)]) -> f32 {
    let length = tracks.iter().map(|(t, _)| t.len()).max().unwrap_or(0);
    (0..length)
        .map(|index| {
            tracks
                .iter()
                .map(|(track, gain)| track.get(index).copied().unwrap_or(0.0) * gain)
                .sum::<f32>()
                .abs()
        })
        .fold(0.0f32, f32::max)
}

#[cfg(test)]
mod tests {
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
}
