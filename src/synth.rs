//! Procedural sound synthesis: build effects in code instead of shipping files.
//!
//! [`SoundManager`](crate::sound) can only load audio from a file or an asset
//! pack, so a game that wants a blip has to find a `.wav` from somewhere. This
//! renders one from a handful of oscillators instead — no assets, nothing to
//! fetch over the wire, and the whole sound set editable as code the same way a
//! shape is editable as drawing calls.
//!
//! An effect is a list of [`Voice`]s. Each is one tone: a waveform, a pitch
//! glide, an attack/decay envelope and a start offset. They are summed and
//! written to a mono 16-bit WAV that
//! `macroquad::audio::load_sound_from_bytes` accepts directly.
//!
//! ```no_run
//! use macroquad_toolkit::synth::{render_wav, Voice, Wave, SynthConfig};
//!
//! // A short bright chime.
//! let bytes = render_wav(
//!     &[
//!         Voice::tone(0.0, 0.10, 1568.0, 0.34).wave(Wave::Triangle),
//!         Voice::tone(0.0, 0.03, 3200.0, 0.14).wave(Wave::Noise),
//!     ],
//!     &SynthConfig::default(),
//!     0xA11CE,
//! );
//! ```
//!
//! # Determinism
//!
//! Noise voices draw from a [`SeededRng`](crate::rng::SeededRng) passed a fixed
//! seed, so a given set of voices always renders byte-identical audio. That
//! matters more than it sounds: it means an effect can be checksummed in a test,
//! and a refactor that quietly changed every sound in a game would fail rather
//! than ship.

pub mod audit;

use crate::rng::SeededRng;

/// Oscillator shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Square,
    Triangle,
    Noise,
}

/// Render settings shared by every voice in an effect.
#[derive(Debug, Clone, Copy)]
pub struct SynthConfig {
    pub sample_rate: u32,
    /// Applied to the summed voices before quantising. Headroom, so several
    /// voices at once cannot clip into a crackle.
    pub master_gain: f32,
}

impl Default for SynthConfig {
    fn default() -> Self {
        Self {
            // Plenty for short effects and a quarter the size of 44.1k.
            sample_rate: 22_050,
            master_gain: 0.32,
        }
    }
}

/// One tone in an effect: a pitch glide under an attack/decay envelope.
#[derive(Debug, Clone, Copy)]
pub struct Voice {
    wave: Wave,
    /// Seconds from the start of the effect.
    start: f32,
    duration: f32,
    freq_from: f32,
    freq_to: f32,
    gain: f32,
    /// Fraction of the voice spent rising to full volume.
    attack: f32,
}

impl Voice {
    /// A steady tone. Chain [`wave`](Self::wave), [`glide`](Self::glide) and
    /// [`attack`](Self::attack) to shape it.
    pub fn tone(start: f32, duration: f32, freq: f32, gain: f32) -> Self {
        Self {
            wave: Wave::Sine,
            start,
            duration,
            freq_from: freq,
            freq_to: freq,
            gain,
            attack: 0.04,
        }
    }

    /// Sweep to another pitch over the voice's life.
    pub fn glide(mut self, to: f32) -> Self {
        self.freq_to = to;
        self
    }

    pub fn wave(mut self, wave: Wave) -> Self {
        self.wave = wave;
        self
    }

    /// Fraction of the voice spent rising to full volume, 0..1.
    pub fn attack(mut self, attack: f32) -> Self {
        self.attack = attack;
        self
    }

    pub fn start(&self) -> f32 {
        self.start
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn shape(&self) -> Wave {
        self.wave
    }

    /// The highest pitch this voice reaches while it can still be heard, and
    /// how far into the voice that is.
    ///
    /// A rising glide aliases worst at the top, but the top is also where the
    /// decay has run out — analysing the last sample would measure silence and
    /// call the sweep clean. Three quarters through is high and still audible.
    pub fn highest_audible(&self) -> (f32, f32) {
        let at = if self.freq_to > self.freq_from {
            self.duration * 0.75
        } else {
            self.duration * self.attack.clamp(0.05, 0.5)
        };
        (self.frequency(at), at)
    }

    /// Amplitude envelope: linear attack, then a curved decay to silence.
    pub fn envelope(&self, t: f32) -> f32 {
        let progress = (t / self.duration).clamp(0.0, 1.0);
        if progress < self.attack {
            progress / self.attack.max(f32::EPSILON)
        } else {
            let fall = (progress - self.attack) / (1.0 - self.attack).max(f32::EPSILON);
            (1.0 - fall).powf(2.2)
        }
    }

    /// Pitch at time `t`, glided geometrically so sweeps sound even. A linear
    /// sweep spends most of its time in the upper octave and reads as a lurch.
    pub fn frequency(&self, t: f32) -> f32 {
        let progress = (t / self.duration).clamp(0.0, 1.0);
        self.freq_from * (self.freq_to / self.freq_from).powf(progress)
    }

    fn sample(&self, t: f32, phase: f32, nyquist: f32, rng: &mut SeededRng) -> f32 {
        let shape = match self.wave {
            Wave::Sine => (phase * std::f32::consts::TAU).sin(),
            Wave::Square | Wave::Triangle => {
                band_limited(self.wave, phase, self.frequency(t), nyquist)
            }
            Wave::Noise => rng.range_f32(-1.0, 1.0),
        };
        shape * self.envelope(t) * self.gain
    }
}

/// A square or triangle built from the partials that *fit*.
///
/// # Why not the obvious shape
///
/// Comparing a phase against a threshold gives the honest waveform and an
/// infinite harmonic series with it. Sampling cannot hold a partial above
/// Nyquist and does not drop it — it reflects it back to `sample_rate - p`,
/// landing at a frequency that is not a harmonic of the note or of anything
/// else. That inharmonic partial sitting under the note *is* what "it sounds
/// nasty" means, and it is worse the brighter the sound: this game's button
/// blip, a square at 1200Hz in a 22kHz stream, was carrying a reflection only
/// 21dB under the note it was supposed to be.
///
/// So the series is summed directly and stops at Nyquist. Nothing is generated
/// that cannot be represented, so there is nothing to reflect. The cost is one
/// `sin` per partial per sample, which for a handful of effects rendered once at
/// startup is not a cost at all.
///
/// The classical series, both normalised to peak near 1:
/// square is `4/π · Σ sin(2πnφ)/n` over odd `n`; triangle is
/// `8/π² · Σ (-1)^((n-1)/2) · sin(2πnφ)/n²` over the same.
fn band_limited(wave: Wave, phase: f32, freq: f32, nyquist: f32) -> f32 {
    let angle = phase * std::f32::consts::TAU;
    let highest = if freq > 0.0 {
        (nyquist / freq) as u32
    } else {
        0
    };
    let mut sum = 0.0f32;
    let mut n = 1u32;
    while n <= highest && n <= MAX_PARTIALS {
        let partial = (angle * n as f32).sin();
        sum += match wave {
            Wave::Square => partial / n as f32,
            // Alternating sign is what makes it a triangle rather than a
            // rounded square; without it the partials pile up on one side.
            _ if ((n - 1) / 2).is_multiple_of(2) => partial / (n * n) as f32,
            _ => -partial / (n * n) as f32,
        };
        n += 2;
    }
    match wave {
        Wave::Square => sum * 4.0 / std::f32::consts::PI,
        _ => sum * 8.0 / (std::f32::consts::PI * std::f32::consts::PI),
    }
}

/// Past this the partials are inaudible and only cost time: the 129th partial
/// of a square is 42dB down, and of a triangle 84dB.
const MAX_PARTIALS: u32 = 129;

/// Sum voices into normalised samples, before quantising to 16-bit.
///
/// Exposed separately from [`render_wav`] so a tool can plot an effect without
/// decoding a WAV to do it.
pub fn render_samples(voices: &[Voice], config: &SynthConfig) -> Vec<f32> {
    let length = voices
        .iter()
        .map(|voice| voice.start + voice.duration)
        .fold(0.0f32, f32::max);
    let total = ((length * config.sample_rate as f32).ceil() as usize).max(1);

    let mut samples = vec![0.0f32; total];
    samples.shrink_to_fit();
    samples
}

/// Render an effect to a mono 16-bit WAV.
pub fn render_wav(voices: &[Voice], config: &SynthConfig, seed: u64) -> Vec<u8> {
    wav_bytes(&render_pcm(voices, config, seed), config)
}

/// The summed, gain-applied waveform in `-1.0..=1.0`, one entry per sample.
///
/// This is what [`render_wav`] quantises. A caller that wants to *look* at an
/// effect — plot its envelope, check nothing clips — wants this rather than the
/// container.
pub fn render_waveform(voices: &[Voice], config: &SynthConfig, seed: u64) -> Vec<f32> {
    let mut samples = render_unclamped(voices, config, seed);
    for sample in &mut samples {
        *sample = sample.clamp(-1.0, 1.0);
    }
    samples
}

/// The summed, gain-applied waveform *before* the limiter.
///
/// [`render_waveform`] clamps to `-1.0..=1.0`, which is what has to happen on
/// the way to 16-bit — and which means an effect mixed too hot is silently
/// flattened rather than reported. The overshoot is only visible here, so this
/// is what an audit measures (`synth::audit`).
pub fn render_unclamped(voices: &[Voice], config: &SynthConfig, seed: u64) -> Vec<f32> {
    let mut samples = render_samples(voices, config);
    let mut rng = SeededRng::new(seed);
    let nyquist = config.sample_rate as f32 * 0.5;

    for voice in voices {
        let first = (voice.start * config.sample_rate as f32) as usize;
        let count = (voice.duration * config.sample_rate as f32) as usize;
        let mut phase = 0.0f32;

        for index in 0..count {
            let Some(slot) = samples.get_mut(first + index) else {
                break;
            };
            let t = index as f32 / config.sample_rate as f32;
            *slot += voice.sample(t, phase, nyquist, &mut rng);
            phase += voice.frequency(t) / config.sample_rate as f32;
        }
    }

    for sample in &mut samples {
        *sample *= config.master_gain;
    }
    samples
}

fn render_pcm(voices: &[Voice], config: &SynthConfig, seed: u64) -> Vec<i16> {
    render_waveform(voices, config, seed)
        .iter()
        .map(|sample| (sample * i16::MAX as f32) as i16)
        .collect()
}

/// A minimal canonical WAV container: RIFF/WAVE, one `fmt ` chunk, one `data`.
pub fn wav_bytes(pcm: &[i16], config: &SynthConfig) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);

    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");

    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // PCM chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // format: PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // channels: mono
    out.extend_from_slice(&config.sample_rate.to_le_bytes());
    out.extend_from_slice(&(config.sample_rate * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in pcm {
        out.extend_from_slice(&sample.to_le_bytes());
    }

    out
}

#[cfg(test)]
mod tests;
