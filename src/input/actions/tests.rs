use super::*;
use macroquad::prelude::KeyCode;

fn map() -> ActionMap {
    let mut map = ActionMap::default();
    map.register(ActionDefinition::new(
        "run",
        "RUN",
        vec![Binding::key(KeyCode::R)],
    ))
    .unwrap();
    map.register(ActionDefinition::new(
        "jump",
        "JUMP",
        vec![Binding::key(KeyCode::Space)],
    ))
    .unwrap();
    map
}

#[test]
fn touch_keyboard_mouse_and_pad_resolve_to_the_same_action() {
    for binding in [
        Binding::key(KeyCode::R),
        Binding::mouse(macroquad::prelude::MouseButton::Right),
        Binding::gamepad(gamepads::Button::ActionDown),
    ] {
        let map = map();
        let mut settings = ControlSettings::default();
        map.rebind(&mut settings, "run", vec![binding.clone()])
            .unwrap();
        let mut runtime = ActionRuntime::default();
        let mut frame = ActionSnapshot::default();
        frame.down.insert(binding);
        runtime.update(&map, &settings, &frame);
        assert!(runtime.state("run").pressed);
        runtime.update(&map, &settings, &frame);
        assert!(runtime.state("run").down);
        assert!(!runtime.state("run").pressed);
        runtime.update(&map, &settings, &ActionSnapshot::default());
        assert!(runtime.state("run").released);
        let mut touch = ActionSnapshot::default();
        touch.touch_pressed.insert("run".into());
        runtime.update(&map, &settings, &touch);
        assert!(runtime.state("run").pressed);
    }
}

#[test]
fn toggle_holds_until_next_press_and_clear_recovers() {
    let map = map();
    let mut settings = ControlSettings::default();
    settings.modes.insert("run".into(), ActionMode::Toggle);
    let mut runtime = ActionRuntime::default();
    let mut frame = ActionSnapshot::default();
    frame.down.insert(Binding::key(KeyCode::R));
    runtime.update(&map, &settings, &frame);
    runtime.update(&map, &settings, &frame);
    assert!(runtime.state("run").down);
    runtime.update(&map, &settings, &ActionSnapshot::default());
    assert!(runtime.state("run").down);
    runtime.update(&map, &settings, &frame);
    assert!(runtime.state("run").released);
    runtime.clear();
    assert!(!runtime.state("run").down);
}

#[test]
fn conflict_rejection_is_atomic_and_empty_binding_stays_unbound() {
    let map = map();
    let mut settings = ControlSettings::default();
    assert!(map
        .rebind(&mut settings, "run", vec![Binding::key(KeyCode::Space)])
        .is_err());
    assert!(settings.bindings.is_empty());
    map.rebind(&mut settings, "run", vec![]).unwrap();
    assert!(map.bindings("run", &settings).is_empty());
    assert!(map.rebind(&mut settings, "absent", vec![]).is_err());
    assert!(map
        .rebind(&mut settings, "run", vec![Binding::Key("bogus".into())])
        .is_err());
}

#[test]
fn prompts_keep_touch_target_and_follow_actual_activity() {
    let map = map();
    let settings = ControlSettings::default();
    assert_eq!(
        map.prompt("run", &settings, InputDevice::KeyboardMouse),
        "Tap RUN or use R"
    );
    let mut runtime = ActionRuntime::default();
    let frame = ActionSnapshot {
        device_activity: Some(InputDevice::Controller),
        ..Default::default()
    };
    runtime.update(&map, &settings, &frame);
    runtime.update(&map, &settings, &ActionSnapshot::default());
    assert_eq!(runtime.active_device, InputDevice::Controller);
}

#[test]
fn gesture_prompts_name_the_direct_touch_gesture() {
    let mut map = ActionMap::default();
    map.register(
        ActionDefinition::new("drag", "DRAG MAP", vec![]).with_touch_instruction("Drag the map"),
    )
    .unwrap();
    assert_eq!(
        map.prompt("drag", &ControlSettings::default(), InputDevice::Touch),
        "Drag the map"
    );
}

#[test]
fn legacy_labels_preserve_controller_defaults_and_existing_overrides() {
    let mut map = map();
    let pad = Binding::gamepad(gamepads::Button::ActionDown);
    map.register(ActionDefinition::new(
        "confirm",
        "CONFIRM",
        vec![pad.clone()],
    ))
    .unwrap();
    let mut settings = ControlSettings::default();
    settings.bindings.insert("run".into(), vec![]);
    let labels = BTreeMap::from([
        ("confirm".into(), "Enter".into()),
        ("run".into(), "T".into()),
    ]);
    map.migrate_legacy(&mut settings, &labels);
    assert!(map.bindings("run", &settings).is_empty());
    assert!(map.bindings("confirm", &settings).contains(&pad));
    assert!(map
        .bindings("confirm", &settings)
        .contains(&Binding::key(KeyCode::Enter)));
    assert!(map.conflicts(&settings).is_empty());
}

#[test]
fn analog_dead_zone_sensitivity_and_inversion_work_together() {
    let controls = ControlSettings {
        dead_zone: 0.2,
        stick_sensitivity: [2.0, 1.0],
        invert_y: true,
        ..Default::default()
    };
    assert_eq!(controls.stick(Vec2::new(0.1, 0.1)), Vec2::ZERO);
    let value = controls.stick(Vec2::new(0.6, 0.0));
    assert!((value.x - 1.0).abs() < 1e-6);
    assert_eq!(controls.stick(Vec2::new(0.0, 1.0)).y, -1.0);
    assert!(controls
        .stick(Vec2::new(f32::NAN, f32::INFINITY))
        .is_finite());
}
