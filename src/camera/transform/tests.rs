use super::*;

fn near(a: Vec2, b: Vec2) {
    assert!((a - b).length() < 0.001, "{a:?} != {b:?}");
}

#[test]
fn round_trips_with_panel_offsets_and_after_resize() {
    let camera = CameraTransform::new(vec2(-130.0, 81.0), 1.7).unwrap();
    for view in [
        Rect::new(180.0, 90.0, 700.0, 400.0),
        Rect::new(40.0, 55.0, 300.0, 650.0),
    ] {
        near(
            camera.world_to_screen(view, camera.target()).unwrap(),
            center(view),
        );
        for world in [vec2(-500.0, 200.0), Vec2::ZERO, vec2(190.0, -31.0)] {
            let screen = camera.world_to_screen(view, world).unwrap();
            near(camera.screen_to_world(view, screen).unwrap(), world);
        }
    }
}

#[test]
fn anchor_is_fixed_even_when_zoom_hits_limits() {
    let mut camera = CameraTransform::new(Vec2::ZERO, 1.0).unwrap();
    let view = Rect::new(260.0, 40.0, 600.0, 400.0);
    let anchor = vec2(310.0, 315.0);
    let world = camera.screen_to_world(view, anchor).unwrap();
    for factor in [1.3, 100.0, 0.0001] {
        assert!(camera.zoom_at(view, anchor, factor, (0.5, 2.5)));
        near(camera.screen_to_world(view, anchor).unwrap(), world);
    }
    assert_eq!(camera.zoom(), 0.5);
}

#[test]
fn keep_visible_handles_small_and_large_maps() {
    let view = Rect::new(80.0, 40.0, 400.0, 300.0);
    for map in [
        Rect::new(0.0, 0.0, 1000.0, 900.0),
        Rect::new(0.0, 0.0, 20.0, 15.0),
    ] {
        let bounds = CameraBounds::from_rect(map);
        let mut camera = CameraTransform::new(vec2(9000.0, -9000.0), 2.0).unwrap();
        assert!(camera.constrain(
            view,
            bounds,
            CameraBoundsPolicy::KeepVisible { pixels: 64.0 }
        ));
        let min = camera.world_to_screen(view, bounds.min).unwrap();
        let max = camera.world_to_screen(view, bounds.max).unwrap();
        let overlap_x = max.x.min(view.x + view.w) - min.x.max(view.x);
        let overlap_y = max.y.min(view.y + view.h) - min.y.max(view.y);
        assert!(overlap_x >= 64.0_f32.min(map.w * 2.0) - 0.001);
        assert!(overlap_y >= 64.0_f32.min(map.h * 2.0) - 0.001);
    }
}

#[test]
fn gesture_composes_drag_and_zoom_without_claiming_taps() {
    let view = Rect::new(100.0, 40.0, 600.0, 400.0);
    let mut camera = CameraTransform::new(Vec2::ZERO, 1.0).unwrap();
    let mut frame = TouchGestureFrame {
        pan: vec2(10.0, 20.0),
        center: center(view),
        scale: 2.0,
        ..Default::default()
    };
    let before = camera;
    assert!(!camera.apply_gesture(view, &frame, (0.5, 3.0)));
    assert_eq!(camera, before);
    frame.claimed = true;
    assert!(camera.apply_gesture(view, &frame, (0.5, 3.0)));
    near(camera.target(), vec2(-10.0, -20.0));
    assert_eq!(camera.zoom(), 2.0);
}

#[test]
fn invalid_input_does_not_poison_camera() {
    assert!(CameraTransform::new(Vec2::ZERO, 0.0).is_err());
    let mut camera = CameraTransform::new(Vec2::ZERO, 1.0).unwrap();
    let before = camera;
    let view = Rect::new(0.0, 0.0, 600.0, 400.0);
    assert!(!camera.zoom_at(view, Vec2::ZERO, f32::NAN, (0.5, 2.0)));
    assert!(!camera.zoom_at(view, Vec2::ZERO, 2.0, (2.0, 0.5)));
    assert!(!camera.pan_screen(vec2(f32::INFINITY, 0.0)));
    assert!(camera
        .screen_to_world(Rect::new(0.0, 0.0, 0.0, 10.0), Vec2::ZERO)
        .is_none());
    let frame = TouchGestureFrame {
        claimed: true,
        pan: vec2(20.0, 5.0),
        scale: f32::NAN,
        ..Default::default()
    };
    assert!(!camera.apply_gesture(view, &frame, (0.5, 2.0)));
    assert_eq!(before, camera);
}
