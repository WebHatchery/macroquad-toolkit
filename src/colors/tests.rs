use super::*;

fn assert_close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "expected {b}, got {a}");
}

#[test]
fn with_alpha_replaces_only_alpha() {
    let c = with_alpha(Color::new(0.2, 0.4, 0.6, 1.0), 0.25);
    assert_close(c.r, 0.2);
    assert_close(c.g, 0.4);
    assert_close(c.b, 0.6);
    assert_close(c.a, 0.25);
}

#[test]
fn lighten_and_darken_clamp() {
    let c = lighten(Color::new(0.9, 0.5, 0.1, 0.8), 0.2);
    assert_close(c.r, 1.0);
    assert_close(c.g, 0.7);
    assert_close(c.a, 0.8);
    let d = darken(Color::new(0.1, 0.5, 0.9, 0.8), 0.2);
    assert_close(d.r, 0.0);
    assert_close(d.g, 0.3);
}

#[test]
fn shade_and_tint_blend_multiplicatively() {
    let c = Color::new(0.8, 0.4, 0.2, 0.7);
    let shaded = shade(c, 0.5);
    assert_close(shaded.r, 0.4);
    assert_close(shaded.g, 0.2);
    assert_close(shaded.a, 0.7);
    let tinted = tint(c, 0.5);
    assert_close(tinted.r, 0.9);
    assert_close(tinted.b, 0.6);
    assert_close(tinted.a, 0.7);
}

#[test]
fn mix_interpolates_and_clamps_t() {
    let a = Color::new(0.0, 0.0, 0.0, 0.0);
    let b = Color::new(1.0, 1.0, 1.0, 1.0);
    let half = mix(a, b, 0.5);
    assert_close(half.r, 0.5);
    assert_close(half.a, 0.5);
    let over = mix(a, b, 2.0);
    assert_close(over.r, 1.0);
}

#[test]
fn hsv_round_trip() {
    for &(r, g, b) in &[
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (0.0, 0.0, 1.0),
        (0.3, 0.6, 0.9),
        (0.5, 0.5, 0.5),
    ] {
        let original = Color::new(r, g, b, 1.0);
        let (h, s, v) = rgb_to_hsv(original);
        let back = hsv_to_rgb(h, s, v);
        assert_close(back.r, r);
        assert_close(back.g, g);
        assert_close(back.b, b);
    }
}

#[test]
fn shift_hue_rotates_primary() {
    let red = Color::new(1.0, 0.0, 0.0, 0.5);
    let green = shift_hue(red, 120.0);
    assert_close(green.g, 1.0);
    assert_close(green.r, 0.0);
    assert_close(green.a, 0.5);
}
