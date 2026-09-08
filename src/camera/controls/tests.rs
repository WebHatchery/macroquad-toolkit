use super::*;

#[test]
fn camera_respects_speed_bounds_and_capture_stops_inertia() {
    let mut camera = Camera2D::default();
    camera.set_bounds(Some(super::super::CameraBounds::new(
        vec2(-10.0, -10.0),
        vec2(10.0, 10.0),
    )));
    let mut driver = CameraController::default();
    let prefs = CameraPreferences {
        pan_speed: 2.0,
        smoothing: 0.1,
        ..Default::default()
    };
    let area = Rect::new(0.0, 0.0, 800.0, 600.0);
    driver.update_2d(
        &mut camera,
        &prefs,
        CameraFrame {
            pan: Vec2::X,
            ..Default::default()
        },
        area,
        0.1,
    );
    assert_eq!(camera.target.x, 10.0);
    let before = camera.target;
    driver.update_2d(
        &mut camera,
        &prefs,
        CameraFrame {
            captured: true,
            pan: -Vec2::X,
            ..Default::default()
        },
        area,
        0.1,
    );
    assert_eq!(camera.target, before);
    assert_eq!(driver.velocity, Vec2::ZERO);
}

#[test]
fn zoom_keeps_pointer_world_position_and_touch_does_not_edge_scroll() {
    let mut camera = Camera2D::default();
    let mut driver = CameraController::default();
    let prefs = CameraPreferences {
        edge_scrolling: true,
        ..Default::default()
    };
    let area = Rect::new(20.0, 40.0, 800.0, 600.0);
    let point = vec2(21.0, 300.0);
    let world = camera.target + (point - area.center()) / camera.zoom;
    driver.update_2d(
        &mut camera,
        &prefs,
        CameraFrame {
            wheel: 1.0,
            pointer: Some(point),
            ..Default::default()
        },
        area,
        0.1,
    );
    let after = camera.target + (point - area.center()) / camera.zoom;
    assert!((world - after).length() < 0.001);
    let before = camera.target;
    driver.update_2d(
        &mut camera,
        &prefs,
        CameraFrame {
            pointer: Some(point),
            ..Default::default()
        },
        area,
        0.1,
    );
    assert_eq!(before, camera.target);
    assert_eq!(edge_direction(vec2(-1.0, 50.0), area), Vec2::ZERO);
}
