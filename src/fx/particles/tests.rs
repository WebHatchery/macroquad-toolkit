use super::*;

#[test]
fn particles_integrate_and_die() {
    let mut system = ParticleSystem::new();
    system.spawn(Particle::new(
        Vec2::ZERO,
        Vec2::new(10.0, 0.0),
        0.5,
        2.0,
        Color::new(1.0, 1.0, 1.0, 1.0),
    ));
    system.update(0.25);
    assert_eq!(system.count(), 1);
    let particle = system.particles()[0];
    assert!(particle.position.x > 0.0);
    assert!((particle.life_fraction() - 0.5).abs() < 1e-4);
    system.update(0.3);
    assert!(system.is_empty());
}

#[test]
fn gravity_and_drag_apply() {
    let mut system = ParticleSystem::new();
    let mut falling = Particle::new(
        Vec2::ZERO,
        Vec2::new(100.0, 0.0),
        10.0,
        2.0,
        Color::new(1.0, 1.0, 1.0, 1.0),
    );
    falling.gravity = 100.0;
    falling.drag = 0.5;
    system.spawn(falling);
    system.update(1.0);
    let particle = system.particles()[0];
    assert!(particle.velocity.y > 0.0, "gravity should pull down");
    assert!(
        particle.velocity.x < 100.0,
        "drag should slow horizontal speed"
    );
}

#[test]
fn burst_respects_capacity() {
    let mut system = ParticleSystem::with_capacity(16);
    system.spawn_burst(Vec2::ZERO, 100, &BurstConfig::default());
    assert_eq!(system.count(), 16);
    // All particles must carry burst config ranges.
    for particle in system.particles() {
        assert!(particle.size >= 1.5 && particle.size <= 4.0);
        assert!(particle.life > 0.0 && particle.life <= 0.8);
    }
}
