//! Runtime sprite recoloring with a per-seed texture cache.
//!
//! Generates unique per-entity visual variations at spawn time by shifting
//! hue/saturation/brightness inside named color regions of a base sprite,
//! so one image file yields many distinct-looking entities. Extracted from
//! dungeon_manager's `sprite_variation.rs`; the RNG and HSV conversions
//! delegate to [`crate::rng::SeededRng`] and [`crate::colors`].
//!
//! ```
//! use macroquad_toolkit::sprite::{ColorRegion, SpriteVariationConfig};
//!
//! // "Robes are blue-to-purple and may vary a lot; skin barely varies."
//! let config = SpriteVariationConfig {
//!     color_regions: vec![
//!         ColorRegion::new("robe", 220.0, 280.0, 0.3, 1.0, 1.5),
//!         ColorRegion::new("skin", 20.0, 50.0, 0.2, 0.6, 0.3),
//!     ],
//!     variation_strength: 0.8,
//! };
//! assert!(config.color_regions[0].matches(250.0, 0.9));
//! ```

use std::collections::HashMap;

use macroquad::prelude::{Color, Image, Texture2D};

use crate::colors::{hsv_to_rgb, rgb_to_hsv};
use crate::rng::SeededRng;

/// Seed for generating deterministic variations.
pub type VariationSeed = u64;

/// A hue/saturation/brightness adjustment applied to one sprite region.
#[derive(Debug, Clone, Copy)]
pub struct ColorVariation {
    /// Hue shift in degrees (-180 to 180).
    pub hue_shift: f32,
    /// Saturation multiplier (1.0 = no change).
    pub saturation: f32,
    /// Brightness multiplier (1.0 = no change).
    pub brightness: f32,
}

impl Default for ColorVariation {
    fn default() -> Self {
        Self {
            hue_shift: 0.0,
            saturation: 1.0,
            brightness: 1.0,
        }
    }
}

impl ColorVariation {
    /// Creates a deterministic random variation from a seed. At strength 1.0
    /// the hue shifts up to ±30°, saturation ±20%, brightness ±15%.
    pub fn from_seed(seed: u64, variation_strength: f32) -> Self {
        let mut rng = SeededRng::new(seed);
        Self {
            hue_shift: rng.range_f32(-30.0, 30.0) * variation_strength,
            saturation: 1.0 + rng.range_f32(-0.2, 0.2) * variation_strength,
            brightness: 1.0 + rng.range_f32(-0.15, 0.15) * variation_strength,
        }
    }
}

/// Defines which parts of a sprite can be varied.
#[derive(Debug, Clone)]
pub struct SpriteVariationConfig {
    /// Color regions that can be varied.
    pub color_regions: Vec<ColorRegion>,
    /// Overall variation strength (0.0 to 1.0).
    pub variation_strength: f32,
}

impl Default for SpriteVariationConfig {
    /// A generic two-region config that varies most saturated colors —
    /// a reasonable fallback for sprites without a tailored config.
    fn default() -> Self {
        Self {
            color_regions: vec![
                ColorRegion::new("primary", 0.0, 360.0, 0.2, 1.0, 1.0),
                ColorRegion::new("secondary", 0.0, 360.0, 0.2, 1.0, 0.8),
            ],
            variation_strength: 0.5,
        }
    }
}

/// A region of color that can be varied, selected by hue/saturation range.
#[derive(Debug, Clone)]
pub struct ColorRegion {
    /// Name of this region (e.g., "robe", "staff", "skin").
    pub name: String,
    /// Target hue range in degrees (0-360). `hue_min > hue_max` wraps
    /// around 0° (e.g., red: 350-10).
    pub hue_min: f32,
    pub hue_max: f32,
    /// Target saturation range (0.0 to 1.0).
    pub sat_min: f32,
    pub sat_max: f32,
    /// How strongly this region varies relative to the config strength.
    pub variation_scale: f32,
}

impl ColorRegion {
    pub fn new(
        name: &str,
        hue_min: f32,
        hue_max: f32,
        sat_min: f32,
        sat_max: f32,
        variation_scale: f32,
    ) -> Self {
        Self {
            name: name.to_string(),
            hue_min,
            hue_max,
            sat_min,
            sat_max,
            variation_scale,
        }
    }

    /// Checks if a color (hue in degrees, saturation 0-1) falls in this region.
    pub fn matches(&self, hue: f32, saturation: f32) -> bool {
        let hue_match = if self.hue_min <= self.hue_max {
            hue >= self.hue_min && hue <= self.hue_max
        } else {
            hue >= self.hue_min || hue <= self.hue_max
        };
        let sat_match = saturation >= self.sat_min && saturation <= self.sat_max;
        hue_match && sat_match
    }
}

/// Pre-computed per-region variations for an individual entity.
#[derive(Debug, Clone, Default)]
pub struct EntityVisualVariation {
    pub region_variations: HashMap<String, ColorVariation>,
}

impl EntityVisualVariation {
    /// Generates one variation per config region, deterministically from `seed`.
    pub fn from_seed(seed: VariationSeed, config: &SpriteVariationConfig) -> Self {
        let mut region_variations = HashMap::new();
        let mut region_seed = seed;

        for region in &config.color_regions {
            let variation = ColorVariation::from_seed(
                region_seed,
                config.variation_strength * region.variation_scale,
            );
            region_variations.insert(region.name.clone(), variation);
            region_seed = region_seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
        }

        Self { region_variations }
    }
}

/// Recolors an image: every non-transparent pixel whose hue/saturation falls
/// in a config region gets that region's variation applied. Pixels outside
/// all regions are unchanged.
pub fn recolor_image(
    base: &Image,
    config: &SpriteVariationConfig,
    variation: &EntityVisualVariation,
) -> Image {
    let width = base.width();
    let height = base.height();
    let mut new_image =
        Image::gen_image_color(width as u16, height as u16, Color::new(0.0, 0.0, 0.0, 0.0));

    for y in 0..height {
        for x in 0..width {
            let pixel = base.get_pixel(x as u32, y as u32);

            if pixel.a < 0.01 {
                new_image.set_pixel(x as u32, y as u32, pixel);
                continue;
            }

            let (h, s, v) = rgb_to_hsv(pixel);

            let region_variation = config.color_regions.iter().find_map(|region| {
                region
                    .matches(h, s)
                    .then(|| variation.region_variations.get(&region.name))
                    .flatten()
            });

            let new_pixel = if let Some(var) = region_variation {
                let new_h = (h + var.hue_shift).rem_euclid(360.0);
                let new_s = (s * var.saturation).clamp(0.0, 1.0);
                let new_v = (v * var.brightness).clamp(0.0, 1.0);
                let rgb = hsv_to_rgb(new_h, new_s, new_v);
                Color::new(rgb.r, rgb.g, rgb.b, pixel.a)
            } else {
                pixel
            };

            new_image.set_pixel(x as u32, y as u32, new_pixel);
        }
    }

    new_image
}

/// Caches recolored textures by (sprite id, seed) so each unique variation
/// is generated once. Register a [`SpriteVariationConfig`] per sprite id;
/// unregistered ids use the fallback config.
#[derive(Default)]
pub struct SpriteVariationCache {
    cache: HashMap<(String, VariationSeed), Texture2D>,
    configs: HashMap<String, SpriteVariationConfig>,
    fallback: SpriteVariationConfig,
}

impl SpriteVariationCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the variation config used for `sprite_id`.
    pub fn register(&mut self, sprite_id: impl Into<String>, config: SpriteVariationConfig) {
        self.configs.insert(sprite_id.into(), config);
    }

    /// Replaces the fallback config used for unregistered sprite ids.
    pub fn set_fallback(&mut self, config: SpriteVariationConfig) {
        self.fallback = config;
    }

    /// Returns the config that would be used for `sprite_id`.
    pub fn config_for(&self, sprite_id: &str) -> &SpriteVariationConfig {
        self.configs.get(sprite_id).unwrap_or(&self.fallback)
    }

    /// Gets the cached varied texture for (`sprite_id`, `seed`), generating
    /// and caching it from `base_texture` on first request.
    pub fn get_or_create(
        &mut self,
        sprite_id: &str,
        seed: VariationSeed,
        base_texture: &Texture2D,
    ) -> Texture2D {
        let key = (sprite_id.to_string(), seed);
        if let Some(texture) = self.cache.get(&key) {
            return texture.clone();
        }

        let config = self.config_for(sprite_id);
        let variation = EntityVisualVariation::from_seed(seed, config);
        let image = base_texture.get_texture_data();
        let new_texture = Texture2D::from_image(&recolor_image(&image, config, &variation));

        self.cache.insert(key, new_texture.clone());
        new_texture
    }

    /// Drops all cached textures (e.g. after replacing configs).
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Number of cached texture variations.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// True when no texture variations are cached.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests;
