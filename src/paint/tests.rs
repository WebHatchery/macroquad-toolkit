use super::*;

#[test]
fn a_filled_rectangle_covers_exactly_its_area() {
    let mut buffer = Buffer::new(40, 40);
    buffer.rect(vec2(10.0, 10.0), vec2(20.0, 20.0), WHITE);
    // 400 of 1600 pixels.
    assert!(
        (buffer.coverage() - 0.25).abs() < 0.02,
        "{}",
        buffer.coverage()
    );
}

#[test]
fn a_triangle_fills_about_half_its_bounding_box() {
    let mut buffer = Buffer::new(40, 40);
    buffer.tri(vec2(0.0, 0.0), vec2(40.0, 0.0), vec2(0.0, 40.0), WHITE);
    assert!(
        (buffer.coverage() - 0.5).abs() < 0.05,
        "{}",
        buffer.coverage()
    );
}

#[test]
fn winding_order_does_not_matter() {
    // The art is not consistent about it, and a rasteriser that cared would
    // silently drop half the facets.
    let mut clockwise = Buffer::new(32, 32);
    let mut widdershins = Buffer::new(32, 32);
    clockwise.tri(vec2(2.0, 2.0), vec2(30.0, 2.0), vec2(16.0, 30.0), WHITE);
    widdershins.tri(vec2(16.0, 30.0), vec2(30.0, 2.0), vec2(2.0, 2.0), WHITE);
    assert_eq!(clockwise.silhouette_difference(&widdershins), 0.0);
}

#[test]
fn a_circle_fills_pi_over_four_of_its_box() {
    let mut buffer = Buffer::new(64, 64);
    buffer.circle(vec2(32.0, 32.0), 32.0, WHITE);
    let expected = std::f32::consts::FRAC_PI_4;
    assert!(
        (buffer.coverage() - expected).abs() < 0.02,
        "{} against {}",
        buffer.coverage(),
        expected
    );
}

#[test]
fn alpha_blends_rather_than_replacing() {
    let mut buffer = Buffer::new(8, 8);
    buffer.rect(vec2(0.0, 0.0), vec2(8.0, 8.0), BLACK);
    buffer.rect(
        vec2(0.0, 0.0),
        vec2(8.0, 8.0),
        Color::new(1.0, 1.0, 1.0, 0.5),
    );
    let pixel = buffer.at(4, 4);
    assert!((pixel[0] - 0.5).abs() < 0.01, "{:?}", pixel);
}

#[test]
fn drawing_outside_the_buffer_is_ignored_rather_than_panicking() {
    let mut buffer = Buffer::new(16, 16);
    buffer.rect(vec2(-100.0, -100.0), vec2(50.0, 50.0), WHITE);
    buffer.circle(vec2(500.0, 500.0), 20.0, WHITE);
    assert_eq!(buffer.coverage(), 0.0);
}
