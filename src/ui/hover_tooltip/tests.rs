use super::*;

fn hovered(tooltip: &mut HoverTooltip, now: f64) {
    tooltip.hover("widget", "hello", vec2(10.0, 20.0), now);
}

#[test]
fn stays_hidden_until_delay_elapses() {
    let mut tooltip = HoverTooltip::with_timings(0.5, 0.1, 0.2);
    hovered(&mut tooltip, 0.0);
    assert!(tooltip.visible(0.4).is_none());
    hovered(&mut tooltip, 0.7);
    let draw = tooltip.visible(0.7).expect("visible after delay");
    assert!((draw.alpha - 1.0).abs() < 1e-4);
    assert_eq!(draw.text, "hello");
}

#[test]
fn fades_in_after_delay() {
    let mut tooltip = HoverTooltip::with_timings(0.5, 0.1, 0.2);
    hovered(&mut tooltip, 0.0);
    hovered(&mut tooltip, 0.55);
    let draw = tooltip.visible(0.55).expect("in fade-in window");
    assert!(draw.alpha > 0.4 && draw.alpha < 0.6, "alpha {}", draw.alpha);
}

#[test]
fn fades_out_then_expires_after_leaving() {
    let mut tooltip = HoverTooltip::with_timings(0.5, 0.1, 0.2);
    hovered(&mut tooltip, 0.0);
    hovered(&mut tooltip, 1.0); // fully visible, cursor leaves here
    let draw = tooltip.visible(1.1).expect("still fading out");
    assert!(draw.alpha > 0.4 && draw.alpha < 0.6, "alpha {}", draw.alpha);
    assert!(tooltip.visible(1.3).is_none());
}

#[test]
fn hovering_new_id_restarts_delay() {
    let mut tooltip = HoverTooltip::with_timings(0.5, 0.1, 0.2);
    tooltip.hover("a", "first", vec2(0.0, 0.0), 0.0);
    tooltip.hover("b", "second", vec2(0.0, 0.0), 1.0);
    assert!(tooltip.visible(1.2).is_none());
    tooltip.hover("b", "second", vec2(0.0, 0.0), 1.6);
    let draw = tooltip.visible(1.6).expect("second tooltip visible");
    assert_eq!(draw.text, "second");
}

#[test]
fn rehover_refreshes_text_without_restarting() {
    let mut tooltip = HoverTooltip::with_timings(0.5, 0.1, 0.2);
    tooltip.hover("a", "old", vec2(0.0, 0.0), 0.0);
    tooltip.hover("a", "new", vec2(0.0, 0.0), 0.7);
    let draw = tooltip.visible(0.7).expect("visible");
    assert_eq!(draw.text, "new");
}

#[test]
fn hover_rect_only_registers_inside() {
    let mut tooltip = HoverTooltip::with_timings(0.0, 0.1, 0.2);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert!(!tooltip.hover_rect("a", "hi", rect, vec2(20.0, 5.0), 0.0));
    assert!(tooltip.visible(0.0).is_none());
    assert!(tooltip.hover_rect("a", "hi", rect, vec2(5.0, 5.0), 0.0));
    assert!(tooltip.visible(0.05).is_some());
}

#[test]
fn dismiss_hides_immediately() {
    let mut tooltip = HoverTooltip::with_timings(0.0, 0.01, 0.2);
    hovered(&mut tooltip, 0.0);
    assert!(tooltip.visible(0.1).is_some());
    tooltip.dismiss();
    assert!(tooltip.visible(0.1).is_none());
}
