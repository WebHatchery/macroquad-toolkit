use super::*;

#[test]
fn color_variation_is_deterministic_per_seed() {
    let v1 = ColorVariation::from_seed(12345, 1.0);
    let v2 = ColorVariation::from_seed(12345, 1.0);
    let v3 = ColorVariation::from_seed(54321, 1.0);

    assert_eq!(v1.hue_shift, v2.hue_shift);
    assert_eq!(v1.saturation, v2.saturation);
    assert_ne!(v1.hue_shift, v3.hue_shift);
}

#[test]
fn zero_strength_variation_is_identity() {
    let v = ColorVariation::from_seed(7, 0.0);
    assert_eq!(v.hue_shift, 0.0);
    assert_eq!(v.saturation, 1.0);
    assert_eq!(v.brightness, 1.0);
}

#[test]
fn color_region_matches_with_hue_wrap() {
    let region = ColorRegion::new("test", 350.0, 10.0, 0.5, 1.0, 1.0);

    assert!(region.matches(0.0, 0.7));
    assert!(region.matches(355.0, 0.7));
    assert!(!region.matches(180.0, 0.7));
    assert!(!region.matches(0.0, 0.3));
}

#[test]
fn entity_variation_covers_every_region() {
    let config = SpriteVariationConfig::default();
    let variation = EntityVisualVariation::from_seed(99, &config);
    assert!(variation.region_variations.contains_key("primary"));
    assert!(variation.region_variations.contains_key("secondary"));
}

#[test]
fn recolor_image_shifts_matching_pixels_and_keeps_others() {
    // Saturated red pixel (matches), gray pixel (saturation 0, no match),
    // and a transparent pixel.
    let mut base = Image::gen_image_color(3, 1, Color::new(0.0, 0.0, 0.0, 0.0));
    base.set_pixel(0, 0, Color::new(1.0, 0.0, 0.0, 1.0));
    base.set_pixel(1, 0, Color::new(0.5, 0.5, 0.5, 1.0));

    let config = SpriteVariationConfig {
        color_regions: vec![ColorRegion::new("hot", 330.0, 30.0, 0.5, 1.0, 1.0)],
        variation_strength: 1.0,
    };
    let variation = EntityVisualVariation {
        region_variations: HashMap::from([(
            "hot".to_string(),
            ColorVariation {
                hue_shift: 120.0,
                saturation: 1.0,
                brightness: 1.0,
            },
        )]),
    };

    let result = recolor_image(&base, &config, &variation);
    let shifted = result.get_pixel(0, 0);
    assert!(shifted.g > 0.9, "red should rotate to green");
    assert!(shifted.r < 0.1);
    assert!((result.get_pixel(1, 0).r - 0.5).abs() < 0.01);
    assert_eq!(result.get_pixel(2, 0).a, 0.0);
}
