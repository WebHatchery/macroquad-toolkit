//! Short-lived visual projectiles that lerp from a start point to a target
//! over a fixed duration, delivering a payload on arrival.
//!
//! Extracted from dungeon_manager's projectile system, rebuilt on
//! [`Timer`](crate::timing::Timer). The payload is whatever your game needs
//! to resolve the hit (damage event, entity ids, effect key); the layer
//! returns arrived payloads from [`ProjectileLayer::update`].
//!
//! ```
//! use macroquad::prelude::vec2;
//! use macroquad_toolkit::fx::ProjectileLayer;
//!
//! struct Impact { damage: f32 }
//!
//! let mut projectiles = ProjectileLayer::new();
//! projectiles.spawn(vec2(0.0, 0.0), vec2(10.0, 0.0), 0.3, Impact { damage: 5.0 });
//! let impacts = projectiles.update(0.4);
//! assert_eq!(impacts.len(), 1);
//! assert_eq!(impacts[0].damage, 5.0);
//! ```

use macroquad::prelude::{vec2, Vec2};
use serde::{Deserialize, Serialize};

use crate::timing::Timer;

/// One projectile in flight, carrying a payload delivered on arrival.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelingProjectile<T> {
    #[serde(with = "vec2_tuple")]
    pub start: Vec2,
    #[serde(with = "vec2_tuple")]
    pub end: Vec2,
    /// Delivered by [`ProjectileLayer::update`] when the projectile arrives.
    pub payload: T,
    timer: Timer,
    travel_ratio: f32,
}

impl<T> TravelingProjectile<T> {
    /// Creates a projectile that travels from `start` to `end` over
    /// `duration` seconds.
    pub fn new(start: Vec2, end: Vec2, duration: f32, payload: T) -> Self {
        Self {
            start,
            end,
            payload,
            timer: Timer::new(duration),
            travel_ratio: 1.0,
        }
    }

    /// Caps travel at a fraction of the start→end distance — e.g. `0.3` for
    /// a melee slash that stays near the attacker while its timer runs.
    pub fn with_travel_ratio(mut self, ratio: f32) -> Self {
        self.travel_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    /// Current interpolated position.
    pub fn position(&self) -> Vec2 {
        let t = self.timer.progress() * self.travel_ratio;
        self.start + (self.end - self.start) * t
    }

    /// Timer progress from 0.0 (launched) to 1.0 (arrived).
    pub fn progress(&self) -> f32 {
        self.timer.progress()
    }

    /// Advances the projectile. Returns true while still in flight.
    pub fn update(&mut self, dt: f32) -> bool {
        self.timer.tick(dt);
        !self.timer.finished()
    }
}

/// Manages all in-flight projectiles: spawn, advance, and collect the
/// payloads of projectiles that arrived this frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileLayer<T> {
    projectiles: Vec<TravelingProjectile<T>>,
}

impl<T> Default for ProjectileLayer<T> {
    fn default() -> Self {
        Self {
            projectiles: Vec::new(),
        }
    }
}

impl<T> ProjectileLayer<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns a projectile traveling from `start` to `end` over `duration`
    /// seconds.
    pub fn spawn(&mut self, start: Vec2, end: Vec2, duration: f32, payload: T) {
        self.push(TravelingProjectile::new(start, end, duration, payload));
    }

    /// Adds a pre-configured projectile (e.g. one with a travel ratio).
    pub fn push(&mut self, projectile: TravelingProjectile<T>) {
        self.projectiles.push(projectile);
    }

    /// Advances every projectile, removing arrived ones and returning their
    /// payloads in spawn order.
    pub fn update(&mut self, dt: f32) -> Vec<T> {
        let mut arrived = Vec::new();
        let mut kept = Vec::with_capacity(self.projectiles.len());
        for mut projectile in self.projectiles.drain(..) {
            if projectile.update(dt) {
                kept.push(projectile);
            } else {
                arrived.push(projectile.payload);
            }
        }
        self.projectiles = kept;
        arrived
    }

    /// In-flight projectiles, for rendering.
    pub fn iter(&self) -> impl Iterator<Item = &TravelingProjectile<T>> {
        self.projectiles.iter()
    }

    /// Number of in-flight projectiles.
    pub fn len(&self) -> usize {
        self.projectiles.len()
    }

    /// True when nothing is in flight.
    pub fn is_empty(&self) -> bool {
        self.projectiles.is_empty()
    }

    /// Removes all projectiles without delivering payloads.
    pub fn clear(&mut self) {
        self.projectiles.clear();
    }
}

/// Serializes `Vec2` as an `(x, y)` tuple, since macroquad's glam does not
/// enable serde support.
mod vec2_tuple {
    use super::{vec2, Vec2};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec2, serializer: S) -> Result<S::Ok, S::Error> {
        (v.x, v.y).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec2, D::Error> {
        let (x, y) = <(f32, f32)>::deserialize(deserializer)?;
        Ok(vec2(x, y))
    }
}

#[cfg(test)]
mod tests;
