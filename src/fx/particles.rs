//! A small pooled particle system: radial bursts, trails, and ambient
//! weather-style emission with lifetime fading.

use macroquad::prelude::{draw_circle, Color, Vec2};

use crate::colors::multiply_alpha;
use crate::rng;

/// A single particle integrated by [`ParticleSystem::update`].
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    /// Remaining life in seconds.
    pub life: f32,
    /// Initial life in seconds (drives the alpha fade).
    pub max_life: f32,
    /// Radius in pixels.
    pub size: f32,
    pub color: Color,
    /// Fraction of velocity kept per second (1.0 = none lost, 0.0 = instant stop).
    pub drag: f32,
    /// Downward acceleration in pixels/second².
    pub gravity: f32,
    /// When true, the particle also shrinks toward zero size as it dies.
    pub shrink: bool,
}

impl Particle {
    /// Creates a particle with no drag, gravity, or shrink.
    pub fn new(position: Vec2, velocity: Vec2, life: f32, size: f32, color: Color) -> Self {
        Self {
            position,
            velocity,
            life,
            max_life: life.max(0.001),
            size,
            color,
            drag: 1.0,
            gravity: 0.0,
            shrink: false,
        }
    }

    /// Fraction of life remaining, 1.0 (fresh) down to 0.0 (dead).
    pub fn life_fraction(&self) -> f32 {
        (self.life / self.max_life).clamp(0.0, 1.0)
    }

    /// True once the particle's life has run out.
    pub fn is_dead(&self) -> bool {
        self.life <= 0.0
    }
}

/// Configuration for a radial burst spawned by [`ParticleSystem::spawn_burst`].
#[derive(Debug, Clone)]
pub struct BurstConfig {
    /// Minimum and maximum launch speed in pixels/second.
    pub speed: (f32, f32),
    /// Minimum and maximum particle radius in pixels.
    pub size: (f32, f32),
    /// Minimum and maximum lifetime in seconds.
    pub life: (f32, f32),
    /// Colors sampled per particle.
    pub colors: Vec<Color>,
    /// Center of the emission arc in radians (0 = right, -PI/2 = up).
    pub direction: f32,
    /// Total arc width in radians; `TAU` for a full circle.
    pub spread: f32,
    /// Fraction of velocity kept per second.
    pub drag: f32,
    /// Downward acceleration in pixels/second².
    pub gravity: f32,
    /// Whether particles shrink as they fade.
    pub shrink: bool,
}

impl Default for BurstConfig {
    fn default() -> Self {
        Self {
            speed: (40.0, 160.0),
            size: (1.5, 4.0),
            life: (0.3, 0.8),
            colors: vec![Color::new(1.0, 1.0, 1.0, 1.0)],
            direction: 0.0,
            spread: std::f32::consts::TAU,
            drag: 0.15,
            gravity: 0.0,
            shrink: true,
        }
    }
}

/// A pool of particles with a hard cap, integrate-and-cull update, and
/// alpha-by-lifetime circle rendering.
///
/// Extracted from kaiju_sim's pooled system, nightmare_shift's weather
/// particles, nanite_swarm's burst/trail particles, scrapyard's debris
/// bursts, and carriage_run's impact sparks.
#[derive(Debug, Clone, Default)]
pub struct ParticleSystem {
    particles: Vec<Particle>,
    max_particles: usize,
}

impl ParticleSystem {
    /// Creates a system capped at 512 live particles.
    pub fn new() -> Self {
        Self::with_capacity(512)
    }

    /// Creates a system capped at `max_particles` live particles.
    pub fn with_capacity(max_particles: usize) -> Self {
        Self {
            particles: Vec::new(),
            max_particles: max_particles.max(1),
        }
    }

    /// Adds one particle. When at capacity the oldest particle is replaced.
    pub fn spawn(&mut self, particle: Particle) {
        if self.particles.len() < self.max_particles {
            self.particles.push(particle);
        } else if let Some(oldest) = self
            .particles
            .iter_mut()
            .min_by(|a, b| a.life.total_cmp(&b.life))
        {
            *oldest = particle;
        }
    }

    /// Spawns `count` particles radiating from `origin` per `config`.
    pub fn spawn_burst(&mut self, origin: Vec2, count: usize, config: &BurstConfig) {
        for _ in 0..count {
            let angle = config.direction + rng::gen_range(-0.5f32, 0.5) * config.spread;
            let speed = rng::gen_range(config.speed.0, config.speed.1.max(config.speed.0));
            let color = rng::choose(&config.colors)
                .copied()
                .unwrap_or(Color::new(1.0, 1.0, 1.0, 1.0));
            let mut particle = Particle::new(
                origin,
                Vec2::new(angle.cos(), angle.sin()) * speed,
                rng::gen_range(config.life.0, config.life.1.max(config.life.0)),
                rng::gen_range(config.size.0, config.size.1.max(config.size.0)),
                color,
            );
            particle.drag = config.drag;
            particle.gravity = config.gravity;
            particle.shrink = config.shrink;
            self.spawn(particle);
        }
    }

    /// Integrates all particles and culls the dead.
    pub fn update(&mut self, dt: f32) {
        for particle in &mut self.particles {
            particle.life -= dt;
            particle.velocity.y += particle.gravity * dt;
            if particle.drag < 1.0 {
                // Frame-rate independent drag: keep `drag` fraction per second.
                particle.velocity *= particle.drag.max(0.0001).powf(dt);
            }
            particle.position += particle.velocity * dt;
        }
        self.particles.retain(|particle| !particle.is_dead());
    }

    /// Draws every particle as a circle fading (and optionally shrinking)
    /// with remaining life.
    pub fn draw(&self) {
        for particle in &self.particles {
            let fade = particle.life_fraction();
            let size = if particle.shrink {
                particle.size * fade
            } else {
                particle.size
            };
            draw_circle(
                particle.position.x,
                particle.position.y,
                size.max(0.1),
                multiply_alpha(particle.color, fade),
            );
        }
    }

    /// Number of live particles.
    pub fn count(&self) -> usize {
        self.particles.len()
    }

    /// True when no particles are alive.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Removes all particles.
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Read access to live particles (e.g. for custom rendering).
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
}

#[cfg(test)]
mod tests;
