//! Focused rules for straight-alpha compositing, tinting, and edge-safe reduction.

use macroquad::prelude::{Color, Image};
use macroquad_toolkit::image_composition::{alpha_over, downsample_alpha_aware, tint_with_mask};

fn image(width: u16, height: u16, pixels: &[[u8; 4]]) -> Image {
    assert_eq!(usize::from(width) * usize::from(height), pixels.len());
    Image {
        bytes: pixels.iter().flatten().copied().collect(),
        width,
        height,
    }
}

#[test]
fn alpha_over_uses_straight_alpha_and_clears_empty_edges() {
    let mut destination = image(2, 1, &[[255, 0, 0, 128], [12, 34, 56, 0]]);
    let source = image(2, 1, &[[0, 0, 255, 128], [90, 80, 70, 0]]);

    alpha_over(&mut destination, &source).unwrap();

    let blended = destination.get_pixel(0, 0);
    assert!((blended.r - 85.0 / 255.0).abs() < 0.01);
    assert!(blended.g < 0.01);
    assert!((blended.b - 170.0 / 255.0).abs() < 0.01);
    assert!((blended.a - 192.0 / 255.0).abs() < 0.01);
    assert_eq!(destination.get_pixel(1, 0), Color::new(0.0, 0.0, 0.0, 0.0));
}

#[test]
fn mask_tint_combines_grayscale_shading_and_mask_coverage() {
    let fill = image(2, 1, &[[128, 128, 128, 255], [255, 255, 255, 128]]);
    let mask = image(2, 1, &[[255, 255, 255, 255], [128, 128, 128, 128]]);
    let tinted = tint_with_mask(&fill, &mask, Color::new(0.8, 0.4, 0.2, 0.5)).unwrap();

    let shaded = tinted.get_pixel(0, 0);
    assert!((shaded.r - (128.0 / 255.0) * 0.8).abs() < 0.01);
    assert!((shaded.g - (128.0 / 255.0) * 0.4).abs() < 0.01);
    assert!((shaded.a - 0.5).abs() < 0.01);

    let clipped = tinted.get_pixel(1, 0);
    assert!((clipped.r - 0.8).abs() < 0.01);
    assert!((clipped.a - (128.0 / 255.0) * (128.0 / 255.0) * (128.0 / 255.0) * 0.5).abs() < 0.01);
}

#[test]
fn downsampling_keeps_transparent_edge_color_clean_over_light_and_dark_surfaces() {
    let source = image(2, 1, &[[255, 0, 0, 255], [0, 0, 255, 0]]);
    let edge = downsample_alpha_aware(&source, 1, 1).unwrap();
    let pixel = edge.get_pixel(0, 0);
    assert!(pixel.r > 0.99);
    assert!(pixel.g < 0.01);
    assert!(pixel.b < 0.01);
    assert!((pixel.a - 0.5).abs() < 0.01);

    let light = Image::gen_image_color(1, 1, Color::new(1.0, 1.0, 1.0, 1.0));
    let dark = Image::gen_image_color(1, 1, Color::new(0.1, 0.1, 0.1, 1.0));
    let mut over_light = light;
    let mut over_dark = dark;
    alpha_over(&mut over_light, &edge).unwrap();
    alpha_over(&mut over_dark, &edge).unwrap();
    assert!(over_light.get_pixel(0, 0).b < 0.51);
    assert!(over_dark.get_pixel(0, 0).b < 0.06);
}

#[test]
fn area_filter_preserves_coverage_and_validates_dimensions() {
    let source = image(
        2,
        2,
        &[
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 255],
        ],
    );
    let reduced = downsample_alpha_aware(&source, 1, 1).unwrap();
    let pixel = reduced.get_pixel(0, 0);

    assert!((pixel.r - 0.5).abs() < 0.01);
    assert!((pixel.g - 0.5).abs() < 0.01);
    assert!((pixel.b - 0.5).abs() < 0.01);
    assert!(pixel.a > 0.99);

    let mut destination = image(1, 1, &[[0, 0, 0, 0]]);
    let mismatched = image(2, 1, &[[0, 0, 0, 0], [0, 0, 0, 0]]);
    assert!(alpha_over(&mut destination, &mismatched).is_err());
    assert!(tint_with_mask(&destination, &mismatched, Color::new(1.0, 1.0, 1.0, 1.0)).is_err());
    assert!(downsample_alpha_aware(&destination, 2, 1).is_err());
    assert!(downsample_alpha_aware(&destination, 0, 1).is_err());
}
