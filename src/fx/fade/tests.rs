use super::*;

#[test]
fn fade_out_then_in_completes() {
    let mut fade = ScreenFade::new(0.5);
    assert!(!fade.update(0.1), "idle fade must not complete");

    fade.fade_out();
    assert!(fade.is_fading());
    assert!(!fade.update(0.25));
    assert!((fade.alpha() - 0.5).abs() < 1e-4);
    assert!(fade.update(0.3), "should complete when alpha reaches 1");
    assert!(!fade.is_fading());
    assert!(fade.is_visible());

    fade.fade_in();
    assert!(!fade.update(0.25));
    assert!(fade.update(0.3));
    assert!(!fade.is_visible());
}

#[test]
fn begin_scene_starts_opaque() {
    let mut fade = ScreenFade::new(1.0);
    fade.begin_scene();
    assert!((fade.alpha() - 1.0).abs() < 1e-6);
    assert_eq!(fade.direction(), Some(FadeDirection::In));
}
