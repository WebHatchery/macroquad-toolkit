use super::*;

#[test]
fn projectile_lerps_toward_target() {
    let mut p = TravelingProjectile::new(vec2(0.0, 0.0), vec2(10.0, 20.0), 1.0, ());
    assert_eq!(p.position(), vec2(0.0, 0.0));
    assert!(p.update(0.5));
    assert_eq!(p.position(), vec2(5.0, 10.0));
    assert!(!p.update(0.6));
    assert_eq!(p.position(), vec2(10.0, 20.0));
}

#[test]
fn travel_ratio_caps_distance() {
    let mut p =
        TravelingProjectile::new(vec2(0.0, 0.0), vec2(10.0, 0.0), 1.0, ()).with_travel_ratio(0.3);
    p.update(1.0);
    assert!((p.position().x - 3.0).abs() < 1e-4);
}

#[test]
fn layer_returns_payloads_on_arrival() {
    let mut layer = ProjectileLayer::new();
    layer.spawn(vec2(0.0, 0.0), vec2(1.0, 0.0), 0.2, "fast");
    layer.spawn(vec2(0.0, 0.0), vec2(1.0, 0.0), 1.0, "slow");

    let first = layer.update(0.3);
    assert_eq!(first, vec!["fast"]);
    assert_eq!(layer.len(), 1);

    let second = layer.update(1.0);
    assert_eq!(second, vec!["slow"]);
    assert!(layer.is_empty());
}

#[test]
fn layer_round_trips_through_serde() {
    let mut layer = ProjectileLayer::new();
    layer.spawn(vec2(1.0, 2.0), vec2(3.0, 4.0), 0.5, 42u32);
    let json = serde_json::to_string(&layer).unwrap();
    let mut restored: ProjectileLayer<u32> = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored.update(1.0), vec![42]);
}
