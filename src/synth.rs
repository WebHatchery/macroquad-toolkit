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

    fn sample(&self, t: f32, phase: f32, rng: &mut SeededRng) -> f32 {
        let shape = match self.wave {
            Wave::Sine => (phase * std::f32::consts::TAU).sin(),
            Wave::Square => {
                if phase.fract() < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Triangle => 4.0 * (phase.fract() - 0.5).abs() - 1.0,
            Wave::Noise => rng.range_f32(-1.0, 1.0),
        };
        shape * self.envelope(t) * self.gain
    }
}

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
    let mut samples = render_samples(voices, config);
    let mut rng = SeededRng::new(seed);

    for voice in voices {
        let first = (voice.start * config.sample_rate as f32) as usize;
        let count = (voice.duration * config.sample_rate as f32) as usize;
        let mut phase = 0.0f32;

        for index in 0..count {
            let Some(slot) = samples.get_mut(first + index) else {
                break;
            };
            let t = index as f32 / config.sample_rate as f32;
            *slot += voice.sample(t, phase, &mut rng);
            phase += voice.frequency(t) / config.sample_rate as f32;
        }
    }

    for sample in &mut samples {
        *sample = (*sample * config.master_gain).clamp(-1.0, 1.0);
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
mod tests {
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
}
