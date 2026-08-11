use super::*;

#[test]
fn smoothing_converges_to_steady_frame_time() {
    let mut overlay = DebugOverlay::new();
    for _ in 0..500 {
        overlay.record_frame(1.0 / 30.0);
    }
    assert!((overlay.fps() - 30.0).abs() < 1.0);
    assert!((overlay.frame_ms() - 33.33).abs() < 1.0);
}

#[test]
fn fps_color_thresholds() {
    assert_eq!(DebugOverlay::fps_color(60.0), dark::POSITIVE);
    assert_eq!(DebugOverlay::fps_color(40.0), dark::WARNING);
    assert_eq!(DebugOverlay::fps_color(20.0), dark::NEGATIVE);
}

#[test]
fn zero_dt_frames_are_ignored() {
    let mut overlay = DebugOverlay::new();
    let before = overlay.fps();
    overlay.record_frame(0.0);
    assert_eq!(overlay.fps(), before);
}
