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
mod tests;
