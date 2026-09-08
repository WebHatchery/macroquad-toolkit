use super::*;
use crate::input::actions::ActionDefinition;

#[test]
fn capture_preserves_other_devices_and_times_out_without_changes() {
    let mut map = ActionMap::default();
    let pad = Binding::gamepad(gamepads::Button::ActionDown);
    map.register(ActionDefinition::new(
        "go",
        "GO",
        vec![Binding::key(KeyCode::G), pad.clone()],
    ))
    .unwrap();
    let mut settings = ControlSettings::default();
    let mut panel = RebindPanel::default();
    panel.begin_capture("go", BindingKind::Keyboard);
    let mut frame = ActionSnapshot::default();
    frame.pressed.insert(Binding::key(KeyCode::H));
    panel.update_capture(&map, &mut settings, &frame, 0.1);
    assert!(!panel.is_capturing());
    assert!(map.bindings("go", &settings).contains(&pad));
    assert!(map
        .bindings("go", &settings)
        .contains(&Binding::key(KeyCode::H)));
    let before = settings.clone();
    panel.begin_capture("go", BindingKind::Mouse);
    panel.update_capture(&map, &mut settings, &ActionSnapshot::default(), 11.0);
    assert!(!panel.is_capturing());
    assert_eq!(settings, before);
}

#[test]
fn binding_roundtrip_and_invalid_values_are_sanitized() {
    let mut settings = ControlSettings::default();
    settings.bindings.insert(
        "jump".into(),
        vec![Binding::key(KeyCode::Space), Binding::Mouse("wrong".into())],
    );
    settings.dead_zone = f32::NAN;
    settings.sanitize();
    let json = serde_json::to_string(&settings).unwrap();
    assert_eq!(settings, serde_json::from_str(&json).unwrap());
    assert_eq!(settings.bindings["jump"].len(), 1);
    assert_eq!(settings.dead_zone, 0.15);
}
