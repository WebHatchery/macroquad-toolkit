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
mod tests {
    use super::SeededRng;

    #[test]
    fn seeded_rng_is_repeatable() {
        let mut a = SeededRng::new(42);
        let mut b = SeededRng::new(42);

        assert_eq!(a.next_u64(), b.next_u64());
        assert_eq!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn seeded_rng_below_stays_in_range() {
        let mut rng = SeededRng::new(11);
        assert_eq!(rng.below(0), 0);
        for _ in 0..256 {
            assert!(rng.below(7) < 7);
        }
    }

    #[test]
    fn seeded_rng_chance_extremes() {
        let mut rng = SeededRng::new(3);
        assert!(!rng.chance(0.0));
        assert!(rng.chance(1.0));
    }

    #[test]
    fn seeded_rng_float_is_unit_interval() {
        let mut rng = SeededRng::new(7);
        for _ in 0..16 {
            let value = rng.next_f32();
            assert!((0.0..=1.0).contains(&value));
        }
    }

    #[test]
    fn range_i32_respects_bounds_and_survives_empty_ranges() {
        let mut rng = SeededRng::new(5);
        assert_eq!(rng.range_i32(3, 3), 3);
        assert_eq!(rng.range_i32(5, 2), 5);
        for _ in 0..128 {
            let value = rng.range_i32(-2, 4);
            assert!((-2..4).contains(&value));
        }
    }

    #[test]
    fn shuffle_is_a_repeatable_permutation() {
        let mut first: Vec<u32> = (0..10).collect();
        let mut second: Vec<u32> = (0..10).collect();
        SeededRng::new(9).shuffle(&mut first);
        SeededRng::new(9).shuffle(&mut second);
        assert_eq!(first, second, "the same seed must shuffle the same way");

        let mut sorted = first.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..10).collect::<Vec<u32>>(), "elements were lost");
    }
}

#[cfg(test)]
mod zero_state_tests {
    use super::*;

    /// `SeededRng::new` xors its seed with the golden-ratio constant, so that
    /// exact seed used to leave the state at zero — and an xorshift at zero
    /// never leaves it. Every draw came back 0 with nothing to indicate a fault.
    #[test]
    fn the_one_seed_that_zeroes_the_state_still_generates() {
        let mut rng = SeededRng::new(0x9E37_79B9_7F4A_7C15);
        let draws: Vec<u64> = (0..8).map(|_| rng.next_u64()).collect();

        assert!(
            draws.iter().any(|value| *value != 0),
            "the generator is stuck at zero"
        );
        assert!(
            draws.windows(2).any(|pair| pair[0] != pair[1]),
            "the generator repeats"
        );
    }

    #[test]
    fn no_seed_leaves_the_generator_stuck() {
        for seed in (0..2_000u64).chain([u64::MAX, 0x9E37_79B9_7F4A_7C15]) {
            let mut rng = SeededRng::new(seed);
            let a = rng.next_u64();
            let b = rng.next_u64();
            assert!(a != 0 || b != 0, "seed {} produced a dead stream", seed);
        }
    }

    #[test]
    fn ordinary_seeds_are_unchanged_by_the_zero_state_guard() {
        // The fix must not move any existing game's stream. These are the first
        // draws for a handful of everyday seeds, recorded from the behaviour
        // before the guard was added.
        for seed in [0u64, 1, 42, 1234, 0xD2A6_0F1E] {
            let expected = {
                let init = seed ^ 0x9E3779B97F4A7C15;
                let mut x = init;
                x ^= x >> 12;
                x ^= x << 25;
                x ^= x >> 27;
                x.wrapping_mul(0x2545F4914F6CDD1D)
            };
            assert_eq!(SeededRng::new(seed).next_u64(), expected, "seed {}", seed);
        }
    }
}

#[cfg(test)]
mod replay {
    use super::*;

    /// The property a commitment scheme rests on: the state is the future.
    #[test]
    fn a_captured_state_replays_the_same_stream() {
        let mut live = SeededRng::new(0xD2A6_0F1E);
        for _ in 0..17 {
            live.next_u64();
        }

        let captured = live.state();
        let ahead: Vec<u64> = (0..64).map(|_| live.next_u64()).collect();

        let mut replayed = SeededRng::from_state(captured);
        let again: Vec<u64> = (0..64).map(|_| replayed.next_u64()).collect();
        assert_eq!(
            ahead, again,
            "a restored generator diverged from the original"
        );
    }

    /// `from_state` must not mix. Feeding a seed to `new` and the same number
    /// to `from_state` are different requests and have to stay different.
    #[test]
    fn from_state_does_not_mix_the_way_new_does() {
        let seeded = SeededRng::new(12_345);
        assert_ne!(seeded.state(), 12_345, "new stopped mixing its seed");
        assert_eq!(SeededRng::from_state(12_345).state(), 12_345);
        assert_eq!(
            SeededRng::from_state(seeded.state()).state(),
            seeded.state(),
            "a round trip through from_state must be the identity"
        );
    }

    /// Zero is the fixed point, on this door as much as on the other one.
    #[test]
    fn a_zero_state_is_refused_here_too() {
        let mut zeroed = SeededRng::from_state(0);
        assert_ne!(zeroed.state(), 0);
        let draws: Vec<u64> = (0..8).map(|_| zeroed.next_u64()).collect();
        assert!(
            draws.iter().any(|value| *value != 0),
            "a zero state produced a dead stream"
        );
    }
}
