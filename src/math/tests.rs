use super::*;

fn assert_close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "expected {b}, got {a}");
}

#[test]
fn lerp_and_inverse() {
    assert_close(lerp(0.0, 10.0, 0.3), 3.0);
    assert_close(inv_lerp(0.0, 10.0, 3.0), 0.3);
    assert_close(inv_lerp(5.0, 5.0, 3.0), 0.0);
    assert_close(remap(5.0, 0.0, 10.0, 100.0, 200.0), 150.0);
}

#[test]
fn smoothstep_endpoints() {
    assert_close(smoothstep(0.0, 1.0, -1.0), 0.0);
    assert_close(smoothstep(0.0, 1.0, 0.5), 0.5);
    assert_close(smoothstep(0.0, 1.0, 2.0), 1.0);
}

#[test]
fn approach_does_not_overshoot() {
    assert_close(approach(0.0, 10.0, 3.0), 3.0);
    assert_close(approach(9.0, 10.0, 3.0), 10.0);
    assert_close(approach(10.0, 0.0, 4.0), 6.0);
}

#[test]
fn easing_endpoints() {
    for ease in [
        ease_in_quad,
        ease_out_quad,
        ease_in_cubic,
        ease_out_cubic,
        ease_in_out_quad,
        ease_out_back,
    ] {
        assert_close(ease(0.0), 0.0);
        assert_close(ease(1.0), 1.0);
    }
    assert!(ease_out_quad(0.5) > 0.5);
    assert!(ease_in_quad(0.5) < 0.5);
}

#[test]
fn pulse_is_bounded() {
    for i in 0..100 {
        let v = pulse01_at(i as f64 * 0.1, 3.0);
        assert!((0.0..=1.0).contains(&v));
    }
}

#[test]
fn blink_toggles_each_half_cycle() {
    // 1 Hz: on for the first half-second, off for the second.
    assert!(blink(0.0, 1.0));
    assert!(blink(0.25, 1.0));
    assert!(!blink(0.5, 1.0));
    assert!(!blink(0.75, 1.0));
    assert!(blink(1.0, 1.0)); // next cycle
                              // Robust for negative elapsed (rem_euclid, not fract).
    assert!(!blink(-0.25, 1.0));
}

#[test]
fn hash_str_is_stable_and_distinct() {
    assert_eq!(hash_str("goblin"), hash_str("goblin"));
    assert_ne!(hash_str("goblin"), hash_str("kobold"));
    // FNV-1a reference value for empty string is the offset basis.
    assert_eq!(hash_str(""), 2166136261);
}

#[test]
fn tween_settles_on_target() {
    let mut tween = Tween::new(0.0, 10.0);
    tween.set_target(50.0);
    for _ in 0..200 {
        tween.update(1.0 / 60.0);
    }
    assert!(tween.is_settled());
    assert_close(tween.current(), 50.0);
}
