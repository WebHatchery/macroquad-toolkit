use super::*;
fn camera(yaw: f32) -> Camera3D {
    let target = vec3(0.0, 0.0, 0.0);
    let distance = 20.0;
    Camera3D {
        position: vec3(yaw.cos() * distance, 14.0, yaw.sin() * distance),
        target,
        up: vec3(0.0, 1.0, 0.0),
        projection: Projection::Orthographics,
        fovy: 20.0,
        aspect: Some(16.0 / 9.0),
        ..Default::default()
    }
}
#[test]
fn center_ray_hits_origin_from_every_quarter_turn() {
    for quarter in 0..4 {
        let ray = screen_ray(
            &camera(quarter as f32 * std::f32::consts::FRAC_PI_2),
            vec2(640.0, 360.0),
            Some((0, 0, 1280, 720)),
        );
        assert!(
            Aabb3::from_center_size(Vec3::ZERO, vec3(2.0, 2.0, 2.0))
                .intersect(ray)
                .is_some(),
            "quarter {quarter}: {ray:?}"
        );
    }
}
#[test]
fn viewport_offset_is_respected() {
    let ray = screen_ray(&camera(0.0), vec2(740.0, 410.0), Some((100, 50, 1280, 720)));
    assert!(Aabb3::from_center_size(Vec3::ZERO, vec3(2.0, 2.0, 2.0))
        .intersect(ray)
        .is_some());
}
#[test]
fn parallel_ray_misses_and_hits_correctly() {
    let box3 = Aabb3::from_center_size(Vec3::ZERO, Vec3::ONE);
    assert!(box3
        .intersect(Ray3 {
            origin: vec3(2.0, 0.0, 0.0),
            direction: vec3(0.0, 1.0, 0.0)
        })
        .is_none());
    assert_eq!(
        box3.intersect(Ray3 {
            origin: vec3(0.0, 0.0, -2.0),
            direction: vec3(0.0, 0.0, 1.0)
        }),
        Some(1.5)
    );
}
