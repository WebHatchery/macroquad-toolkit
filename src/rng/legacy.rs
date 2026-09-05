//! Compatibility for the LCG streams already shipped by Final Landing and Hatchspire.

use serde::{Deserialize, Serialize};

/// Legacy multiply/add/upper-32-bit generator. Prefer `SeededRng` for new code.
/// Increment `1` preserves Final Landing; `1_442_695_040_888_963_407` preserves
/// Hatchspire. Range conversion and game seed defaults remain caller-owned.
/// The increment is part of the algorithm, not serialized state: never change
/// it while restoring a saved stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyLcg64<const INCREMENT: u64> {
    state: u64,
}

impl<const INCREMENT: u64> LegacyLcg64<INCREMENT> {
    /// Both legacy games map seed zero to one. This is not state restoration.
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    /// Restore all 64 state bits exactly, including zero (valid for an LCG).
    pub fn from_state(state: u64) -> Self {
        Self { state }
    }

    pub fn state(&self) -> u64 {
        self.state
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(INCREMENT);
        (self.state >> 32) as u32
    }
}

#[cfg(test)]
mod tests;
