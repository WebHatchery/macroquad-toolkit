use super::*;

fn any_color() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}

#[test]
fn texts_rise_and_expire() {
    let mut layer = FloatingTextLayer::new();
    layer.spawn("+5", Vec2::new(10.0, 100.0), any_color());
    layer.update(0.5);
    let text = &layer.texts()[0];
    assert!(text.position.y < 100.0, "text should rise");
    assert!(text.life_fraction() < 1.0);
    layer.update(layer.default_lifetime);
    assert!(layer.is_empty());
}

#[test]
fn cap_drops_oldest() {
    let mut layer = FloatingTextLayer {
        max_active: 3,
        ..Default::default()
    };
    for i in 0..5 {
        layer.spawn(format!("t{i}"), Vec2::ZERO, any_color());
    }
    assert_eq!(layer.count(), 3);
    assert_eq!(layer.texts()[0].text, "t2", "oldest entries drop first");
}

#[test]
fn drag_slows_velocity() {
    let mut layer = FloatingTextLayer {
        drag: 0.5,
        ..Default::default()
    };
    let mut custom = FloatingText::new("x", Vec2::ZERO, any_color(), 16.0, 5.0, 0.0);
    custom.velocity = Vec2::new(100.0, 0.0);
    layer.push(custom);
    layer.update(1.0);
    assert!(layer.texts()[0].velocity.x < 100.0);
}
