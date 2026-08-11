//! Random Number Generation utilities
//!
//! Wraps macroquad::rand to provide a consistent interface and helper functions.
//! Replaces direct usage of the `rand` crate to ensure WebGL compatibility.

use macroquad::rand;
use serde::{Deserialize, Serialize};

/// Small deterministic RNG for reproducible generation.
///
/// Serializable so games can save mid-run RNG state and keep replays and
/// save/load deterministic.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        let init = seed ^ 0x9E3779B97F4A7C15;
        // An xorshift with a zero state is a fixed point: it returns 0 for
        // every draw, forever, and nothing about it looks broken from the
        // outside. Exactly one seed produces it — the golden-ratio constant
        // above, which is precisely the sort of number someone reaches for when
        // they want a "nice" fixed seed. Substituting a fallback only in that
        // one case leaves every other seed's stream byte-identical, so no
        // existing game's determinism moves.
        Self {
            state: if init == 0 {
                0xDEAD_BEEF_CAFE_F00D
            } else {
                init
            },
        }
    }

    /// The generator's whole state, which is also its whole future.
    ///
    /// An xorshift has no hidden inputs: this number and the code below decide
    /// every draw that follows. Exposing it lets a game *commit* to an outcome
    /// before revealing it — record the state, spin, and anyone holding the
    /// number can re-run the same draw and get the same answer.
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Restore a generator to a state taken from [`state`](Self::state).
    ///
    /// Deliberately not `new`: `new` mixes the seed, and a value that came back
    /// out of `state` must go back in untouched or the stream will not replay.
    /// The zero guard is the same one `new` carries and for the same reason — a
    /// zero state is an xorshift's fixed point and returns 0 forever.
    pub fn from_state(state: u64) -> Self {
        Self {
            state: if state == 0 {
                0xDEAD_BEEF_CAFE_F00D
            } else {
                state
            },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn next_f32(&mut self) -> f32 {
        let value = self.next_u64() >> 40;
        (value as f32) / ((1u64 << 24) as f32)
    }

    /// Uniform integer in [0, n). Returns 0 when n == 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// Uniform float in [low, high).
    pub fn range_f32(&mut self, low: f32, high: f32) -> f32 {
        low + self.next_f32() * (high - low)
    }

    /// True with probability `p` (0.0 to 1.0).
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }

    /// Pick a random element from a slice. None when empty.
    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            Some(&slice[self.below(slice.len())])
        }
    }

    /// Uniform integer in [low, high). Returns `low` when the range is empty,
    /// matching the module-level `gen_range` convention.
    pub fn range_i32(&mut self, low: i32, high: i32) -> i32 {
        if high <= low {
            low
        } else {
            low + self.below((high - low) as usize) as i32
        }
    }

    /// Shuffle a slice in place (Fisher-Yates), drawing from this stream.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.below(i + 1);
            slice.swap(i, j);
        }
    }
}

/// Generate a random float between 0.0 and 1.0 (exclusive)
pub fn rand() -> f32 {
    rand::gen_range(0.0, 1.0)
}

/// Seed Macroquad's shared random generator.
pub fn srand(seed: u64) {
    rand::srand(seed);
}

/// Generate a random `u64`, useful for visual seeds and IDs.
pub fn random_u64() -> u64 {
    rand::gen_range(0u64, u64::MAX)
}

/// Generate a random `u32`, useful for compact IDs and legacy helper APIs.
pub fn random_u32() -> u32 {
    rand::gen_range(0u32, u32::MAX)
}

/// Generate a random value within a range
/// Supports floats (0.0, 1.0) and integers (0, 10)
pub fn gen_range<T>(low: T, high: T) -> T
where
    T: macroquad::rand::RandomRange,
{
    rand::gen_range(low, high)
}

/// Return true with a given probability (0.0 to 1.0)
pub fn chance(probability: f32) -> bool {
    rand::gen_range(0.0, 1.0) < probability.clamp(0.0, 1.0)
}

/// Return true with a whole-number percentage chance from 0 to 100.
pub fn chance_percent(percent: i32) -> bool {
    rand::gen_range(0, 100) < percent.clamp(0, 100)
}

/// Shuffle a slice in place using Fisher-Yates algorithm
pub fn shuffle<T>(slice: &mut [T]) {
    for i in (1..slice.len()).rev() {
        // gen_range(0, i + 1) generates int in [0, i] because high is exclusive?
        // macroquad::rand::gen_range for integers is [low, high) i.e. exclusive.
        // So gen_range(0, i + 1) gives 0..=i
        let j = rand::gen_range(0, i + 1);
        slice.swap(i, j);
    }
}

/// Pick a random element from a slice
pub fn choose<T>(slice: &[T]) -> Option<&T> {
    if slice.is_empty() {
        None
    } else {
        Some(&slice[rand::gen_range(0, slice.len())])
    }
}

/// Pick up to `count` random unique elements from a slice.
pub fn choose_multiple<T>(slice: &[T], count: usize) -> Vec<&T> {
    if slice.is_empty() || count == 0 {
        return Vec::new();
    }

    let mut indices: Vec<usize> = (0..slice.len()).collect();
    shuffle(&mut indices);

    indices
        .into_iter()
        .take(count.min(slice.len()))
        .map(|index| &slice[index])
        .collect()
}

/// Pick a random mutable element from a slice
pub fn choose_mut<T>(slice: &mut [T]) -> Option<&mut T> {
    if slice.is_empty() {
        None
    } else {
        let len = slice.len();
        Some(&mut slice[rand::gen_range(0, len)])
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod zero_state_tests;

#[cfg(test)]
mod replay;
