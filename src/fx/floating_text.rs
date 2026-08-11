//! Rising, fading transient text: damage numbers, resource gains, and
//! short world-anchored feedback messages.

use macroquad::prelude::{draw_text_ex, Color, Vec2, BLACK};

use crate::colors::multiply_alpha;
use crate::ui::ui_text_params;

/// One floating text instance managed by a [`FloatingTextLayer`].
#[derive(Debug, Clone)]
pub struct FloatingText {
    pub text: String,
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: Color,
    pub font_size: f32,
    life: f32,
    max_life: f32,
}

impl FloatingText {
    /// Creates a floating text with an upward drift.
    pub fn new(
        text: impl Into<String>,
        position: Vec2,
        color: Color,
        font_size: f32,
        lifetime: f32,
        rise_speed: f32,
    ) -> Self {
        Self {
            text: text.into(),
            position,
            velocity: Vec2::new(0.0, -rise_speed),
            color,
            font_size,
            life: lifetime,
            max_life: lifetime.max(0.001),
        }
    }

    /// Fraction of life remaining, 1.0 (fresh) down to 0.0 (expired).
    pub fn life_fraction(&self) -> f32 {
        (self.life / self.max_life).clamp(0.0, 1.0)
    }

    /// True once the text has expired.
    pub fn is_expired(&self) -> bool {
        self.life <= 0.0
    }
}

/// Manages a capped collection of floating texts: spawn, rise/fade update,
/// and shadowed rendering.
///
/// Draw inside your world camera for world-anchored numbers, or outside it
/// for screen-space feedback. Extracted from apartment's FloatingText,
/// carriage_run's FloatText, dungeon_core's room effects, feast_frenzy's
/// floaters, and kaiju_sim's DamageNumberManager.
///
/// ```
/// use macroquad::prelude::*;
/// use macroquad_toolkit::fx::FloatingTextLayer;
///
/// let mut damage_numbers = FloatingTextLayer::new();
/// damage_numbers.spawn("-12", vec2(100.0, 80.0), RED);
/// damage_numbers.update(1.0 / 60.0);
/// assert_eq!(damage_numbers.count(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct FloatingTextLayer {
    texts: Vec<FloatingText>,
    /// Oldest entries are dropped beyond this cap.
    pub max_active: usize,
    /// Lifetime in seconds used by [`spawn`](Self::spawn).
    pub default_lifetime: f32,
    /// Upward speed in pixels/second used by [`spawn`](Self::spawn).
    pub default_rise_speed: f32,
    /// Font size used by [`spawn`](Self::spawn).
    pub default_font_size: f32,
    /// Fraction of velocity kept per second (1.0 keeps full speed).
    pub drag: f32,
    /// Draw a dark drop shadow behind each text.
    pub shadow: bool,
}

impl Default for FloatingTextLayer {
    fn default() -> Self {
        Self {
            texts: Vec::new(),
            max_active: 48,
            default_lifetime: 1.2,
            default_rise_speed: 28.0,
            default_font_size: 18.0,
            drag: 1.0,
            shadow: true,
        }
    }
}

impl FloatingTextLayer {
    /// Creates a layer with sensible defaults (1.2s life, 28px/s rise, cap 48).
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns a text at `position` using the layer defaults.
    pub fn spawn(&mut self, text: impl Into<String>, position: Vec2, color: Color) {
        self.push(FloatingText::new(
            text,
            position,
            color,
            self.default_font_size,
            self.default_lifetime,
            self.default_rise_speed,
        ));
    }

    /// Spawns a fully custom floating text.
    pub fn push(&mut self, text: FloatingText) {
        self.texts.push(text);
        if self.texts.len() > self.max_active {
            let excess = self.texts.len() - self.max_active;
            self.texts.drain(..excess);
        }
    }

    /// Moves, decelerates, ages, and culls all texts.
    pub fn update(&mut self, dt: f32) {
        for text in &mut self.texts {
            if self.drag < 1.0 {
                text.velocity *= self.drag.max(0.0001).powf(dt);
            }
            text.position += text.velocity * dt;
            text.life -= dt;
        }
        self.texts.retain(|text| !text.is_expired());
    }

    /// Draws every text, fading with remaining life.
    pub fn draw(&self) {
        for text in &self.texts {
            let fade = text.life_fraction();
            if self.shadow {
                draw_text_ex(
                    &text.text,
                    text.position.x + 1.0,
                    text.position.y + 1.0,
                    ui_text_params(text.font_size, multiply_alpha(BLACK, fade * 0.8)),
                );
            }
            draw_text_ex(
                &text.text,
                text.position.x,
                text.position.y,
                ui_text_params(text.font_size, multiply_alpha(text.color, fade)),
            );
        }
    }

    /// Number of live texts.
    pub fn count(&self) -> usize {
        self.texts.len()
    }

    /// True when nothing is on screen.
    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    /// Removes all texts.
    pub fn clear(&mut self) {
        self.texts.clear();
    }

    /// Read access to live texts (e.g. for custom rendering).
    pub fn texts(&self) -> &[FloatingText] {
        &self.texts
    }
}

#[cfg(test)]
mod tests;
