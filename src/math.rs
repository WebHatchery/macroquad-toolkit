//! Interpolation, easing, and small numeric helpers.
//!
//! Extracted from hand-rolled copies across the games: `lerp`/`approach`
//! (kaiju_sim, iron_fauna), cubic ease-out (monsterhall), exponential tweens
//! (apartment), and the pervasive `(get_time() * k).sin()` pulse idiom
//! (ai_defense, nanite_swarm, nightmare_shift, monsterhall).

use macroquad::time::get_time;

/// Linear interpolation from `a` to `b` by `t` (unclamped).
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Inverse of [`lerp`]: where `value` sits between `a` and `b`, clamped to `[0, 1]`.
/// Returns 0.0 when `a == b`.
pub fn inv_lerp(a: f32, b: f32, value: f32) -> f32 {
    if (b - a).abs() <= f32::EPSILON {
        0.0
    } else {
        ((value - a) / (b - a)).clamp(0.0, 1.0)
    }
}

/// Remaps `value` from range `[in_min, in_max]` to `[out_min, out_max]`, clamped.
pub fn remap(value: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    lerp(out_min, out_max, inv_lerp(in_min, in_max, value))
}

/// Clamps a value to `[0, 1]`.
pub fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

/// Hermite smoothstep: 0 at `edge0`, 1 at `edge1`, smooth in between.
pub fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = inv_lerp(edge0, edge1, value);
    t * t * (3.0 - 2.0 * t)
}

/// Moves `current` toward `target` by at most `max_delta`, without overshooting.
pub fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + max_delta.copysign(target - current)
    }
}

/// Frame-rate-independent exponential approach toward `target`.
/// `sharpness` is roughly "fraction converged per second" scale; higher is snappier.
pub fn exp_approach(current: f32, target: f32, sharpness: f32, dt: f32) -> f32 {
    lerp(current, target, 1.0 - (-sharpness * dt).exp())
}

/// Quadratic ease-in: slow start.
pub fn ease_in_quad(t: f32) -> f32 {
    let t = clamp01(t);
    t * t
}

/// Quadratic ease-out: slow end.
pub fn ease_out_quad(t: f32) -> f32 {
    let t = clamp01(t);
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Cubic ease-in: slower start.
pub fn ease_in_cubic(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * t
}

/// Cubic ease-out: slower end.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = clamp01(t);
    1.0 - (1.0 - t).powi(3)
}

/// Quadratic ease-in-out.
pub fn ease_in_out_quad(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// Ease-out with a single overshoot bounce ("back" easing), for pop-in effects.
pub fn ease_out_back(t: f32) -> f32 {
    let t = clamp01(t);
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

/// Sine oscillation mapped to `[0, 1]` at time `t` (seconds) and `speed` (radians/sec).
/// Pure variant of [`pulse01`] for tests and fixed-timestep code.
pub fn pulse01_at(t: f64, speed: f32) -> f32 {
    (((t as f32 * speed).sin()) + 1.0) * 0.5
}

/// Sine oscillation mapped to `[0, 1]` using the current [`get_time`].
/// Replaces the common `(get_time() * k).sin()` glow/pulse idiom.
pub fn pulse01(speed: f32) -> f32 {
    pulse01_at(get_time(), speed)
}

/// Sine oscillation between `min` and `max` using the current [`get_time`].
pub fn pulse_range(speed: f32, min: f32, max: f32) -> f32 {
    lerp(min, max, pulse01(speed))
}

/// Vertical bobbing offset in `[-amplitude, amplitude]` using the current [`get_time`].
pub fn bob(speed: f32, amplitude: f32) -> f32 {
    (get_time() as f32 * speed).sin() * amplitude
}

/// Square-wave blink: `true` for the first half of each cycle, `false` for the
/// second. `t` is elapsed seconds and `hz` is blinks per second. Replaces the
/// `(t * k).fract() < 0.5` cursor/caret idiom; robust for any finite `t`.
pub fn blink(t: f32, hz: f32) -> bool {
    (t * hz).rem_euclid(1.0) < 0.5
}

/// FNV-1a hash of a string. Useful for deriving stable procedural seeds from ids.
pub fn hash_str(s: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for byte in s.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// A value that eases toward a target over time with exponential smoothing.
///
/// Extracted from apartment's panel slide-in tween. Call [`Tween::update`]
/// once per frame; `current()` snaps to the target when close enough.
///
/// ```
/// use macroquad_toolkit::math::Tween;
///
/// let mut slide = Tween::new(0.0, 8.0);
/// slide.set_target(100.0);
/// slide.update(0.5);
/// assert!(slide.current() > 90.0 && slide.current() <= 100.0);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Tween {
    current: f32,
    target: f32,
    /// Exponential sharpness (fraction converged per second scale).
    pub sharpness: f32,
    /// Distance below which `current` snaps exactly to `target`.
    pub snap_distance: f32,
}

impl Tween {
    /// Creates a tween resting at `value` with the given `sharpness`.
    pub fn new(value: f32, sharpness: f32) -> Self {
        Self {
            current: value,
            target: value,
            sharpness,
            snap_distance: 0.5,
        }
    }

    /// Sets the value the tween eases toward.
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Immediately jumps to `value` (no easing).
    pub fn snap_to(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    /// Current eased value.
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Target value.
    pub fn target(&self) -> f32 {
        self.target
    }

    /// True once the tween has settled on its target.
    pub fn is_settled(&self) -> bool {
        self.current == self.target
    }

    /// Advances the tween by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        self.current = exp_approach(self.current, self.target, self.sharpness, dt);
        if (self.target - self.current).abs() <= self.snap_distance {
            self.current = self.target;
        }
    }
}

#[cfg(test)]
mod tests;
