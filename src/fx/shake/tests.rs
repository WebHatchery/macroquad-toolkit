use super::*;

#[test]
fn trauma_clamps_and_decays() {
    let mut shake = ScreenShake::new(10.0);
    shake.add_trauma(0.8);
    shake.add_trauma(0.8);
    assert!((shake.trauma() - 1.0).abs() < 1e-6);
    shake.update(1.0);
    assert!(shake.trauma() < 1.0);
    shake.update(10.0);
    assert!(!shake.is_active());
    assert_eq!(shake.offset(), Vec2::ZERO);
}

#[test]
fn offset_bounded_by_max_offset() {
    let mut shake = ScreenShake::new(5.0);
    shake.add_trauma(1.0);
    for _ in 0..50 {
        let offset = shake.offset();
        assert!(offset.x.abs() <= 5.0 + 1e-4);
        assert!(offset.y.abs() <= 5.0 + 1e-4);
    }
}

#[test]
fn timed_shake_lasts_roughly_duration() {
    let mut shake = ScreenShake::new(10.0);
    shake.shake(0.6, 0.5);
    for _ in 0..29 {
        shake.update(1.0 / 60.0);
    }
    assert!(
        shake.is_active(),
        "should still be shaking just before 0.5s"
    );
    for _ in 0..5 {
        shake.update(1.0 / 60.0);
    }
    assert!(!shake.is_active(), "should settle right after 0.5s");
}
