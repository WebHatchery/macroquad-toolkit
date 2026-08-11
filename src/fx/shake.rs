//! Trauma-based screen shake (Nystrom model).

use macroquad::prelude::Vec2;

use crate::rng;

/// Screen shake driven by an accumulating, decaying trauma value.
///
/// Shake magnitude is `trauma²`, so small hits barely register while large
/// ones are violent, and everything settles smoothly. Add the [`offset`] to
/// camera target or draw positions each frame.
///
/// Extracted from scrapyard's renderer trauma, nightmare_shift's
/// `ScreenShake`, alchemy_tower's camera shake, and kaiju_sim's impact shake.
///
/// [`offset`]: ScreenShake::offset
///
/// ```
/// use macroquad_toolkit::fx::ScreenShake;
///
/// let mut shake = ScreenShake::new(12.0);
/// shake.add_trauma(0.5);
/// shake.update(1.0 / 60.0);
/// let offset = shake.offset();
/// assert!(offset.x.abs() <= 12.0 && offset.y.abs() <= 12.0);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ScreenShake {
    trauma: f32,
    /// Maximum pixel offset at full trauma.
    pub max_offset: f32,
    /// Trauma drained per second.
    pub decay_rate: f32,
}

impl ScreenShake {
    /// Creates a shake with the given maximum pixel offset and a default
    /// decay of 1.5 trauma/second.
    pub fn new(max_offset: f32) -> Self {
        Self {
            trauma: 0.0,
            max_offset,
            decay_rate: 1.5,
        }
    }

    /// Adds trauma (clamped to `[0, 1]`). Stacks with existing trauma.
    pub fn add_trauma(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount.max(0.0)).min(1.0);
    }

    /// Convenience for one-off timed shakes: sets trauma to `intensity` and
    /// tunes decay so the shake lasts roughly `duration` seconds.
    pub fn shake(&mut self, intensity: f32, duration: f32) {
        let intensity = intensity.clamp(0.0, 1.0);
        self.trauma = self.trauma.max(intensity);
        if duration > 0.0 {
            self.decay_rate = intensity / duration;
        }
    }

    /// Decays trauma by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        self.trauma = (self.trauma - self.decay_rate * dt).max(0.0);
    }

    /// Current trauma in `[0, 1]`.
    pub fn trauma(&self) -> f32 {
        self.trauma
    }

    /// True while any shake is still active.
    pub fn is_active(&self) -> bool {
        self.trauma > 0.0
    }

    /// Stops the shake immediately.
    pub fn clear(&mut self) {
        self.trauma = 0.0;
    }

    /// A random offset for this frame, scaled by `trauma²` and `max_offset`.
    pub fn offset(&self) -> Vec2 {
        if self.trauma <= 0.0 {
            return Vec2::ZERO;
        }
        let magnitude = self.trauma * self.trauma * self.max_offset;
        Vec2::new(
            rng::gen_range(-1.0f32, 1.0) * magnitude,
            rng::gen_range(-1.0f32, 1.0) * magnitude,
        )
    }
}

#[cfg(test)]
mod tests;
