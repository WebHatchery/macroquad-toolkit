use super::*;

fn blank(width: u16, height: u16) -> Image {
    Image::gen_image_color(width, height, Color::new(0.0, 0.0, 0.0, 0.0))
}

fn assert_close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "expected {b}, got {a}");
}

#[test]
fn set_pixel_safe_ignores_out_of_bounds() {
    let red = Color::new(1.0, 0.0, 0.0, 1.0);
    let mut image = blank(4, 4);
    set_pixel_safe(&mut image, -1, 0, red);
    set_pixel_safe(&mut image, 0, 4, red);
    set_pixel_safe(&mut image, 2, 2, red);
    assert_close(image.get_pixel(2, 2).r, 1.0);
    assert_close(image.get_pixel(0, 0).a, 0.0);
}

#[test]
fn fill_rect_clips_to_image() {
    let mut image = blank(4, 4);
    fill_rect(&mut image, 2, 2, 10, 10, macroquad::prelude::WHITE);
    assert_close(image.get_pixel(3, 3).a, 1.0);
    assert_close(image.get_pixel(1, 1).a, 0.0);
}

#[test]
fn fill_circle_covers_center_not_corners() {
    let mut image = blank(9, 9);
    fill_circle(&mut image, 4, 4, 3, macroquad::prelude::WHITE);
    assert_close(image.get_pixel(4, 4).a, 1.0);
    assert_close(image.get_pixel(4, 1).a, 1.0);
    assert_close(image.get_pixel(0, 0).a, 0.0);
}

#[test]
fn fill_ellipse_respects_radii() {
    let mut image = blank(11, 11);
    fill_ellipse(&mut image, 5, 5, 4, 2, macroquad::prelude::WHITE);
    assert_close(image.get_pixel(9, 5).a, 1.0);
    assert_close(image.get_pixel(5, 7).a, 1.0);
    assert_close(image.get_pixel(5, 9).a, 0.0);
}

#[test]
fn line_connects_endpoints() {
    let mut image = blank(8, 8);
    draw_line_pixels(&mut image, 0, 0, 7, 7, macroquad::prelude::WHITE);
    assert_close(image.get_pixel(0, 0).a, 1.0);
    assert_close(image.get_pixel(7, 7).a, 1.0);
    assert_close(image.get_pixel(3, 3).a, 1.0);
    assert_close(image.get_pixel(7, 0).a, 0.0);
}

#[test]
fn add_noise_is_deterministic_and_skips_transparent() {
    let mut a = Image::gen_image_color(8, 8, Color::new(0.5, 0.5, 0.5, 1.0));
    let mut b = Image::gen_image_color(8, 8, Color::new(0.5, 0.5, 0.5, 1.0));
    add_noise(&mut a, 42, 0.3);
    add_noise(&mut b, 42, 0.3);
    for y in 0..8u32 {
        for x in 0..8u32 {
            assert_close(a.get_pixel(x, y).r, b.get_pixel(x, y).r);
        }
    }

    let mut clear = blank(4, 4);
    add_noise(&mut clear, 42, 0.5);
    assert_close(clear.get_pixel(1, 1).a, 0.0);
    assert_close(clear.get_pixel(1, 1).r, 0.0);
}
