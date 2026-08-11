use super::*;

#[test]
fn effective_volumes_multiply_groups() {
    let settings = GameSettings {
        master_volume: 0.5,
        sfx_volume: 0.5,
        music_volume: 1.0,
        ..Default::default()
    };
    assert!((settings.effective_sfx_volume() - 0.25).abs() < 1e-6);
    assert!((settings.effective_music_volume() - 0.5).abs() < 1e-6);
}

#[test]
fn partial_json_loads_with_defaults() {
    let settings: GameSettings = serde_json::from_str(r#"{"fullscreen": true}"#).unwrap();
    assert!(settings.fullscreen);
    assert!((settings.master_volume - 1.0).abs() < 1e-6);
    assert!(settings.screen_shake);
}

#[test]
fn autosave_interval_defaults_and_clamps() {
    assert!((GameSettings::default().autosave_interval - 30.0).abs() < 1e-6);

    let mut too_fast = GameSettings {
        autosave_interval: 1.0,
        ..Default::default()
    };
    too_fast.sanitize();
    assert!((too_fast.autosave_interval - 5.0).abs() < 1e-6);

    let mut too_slow = GameSettings {
        autosave_interval: 9_999.0,
        ..Default::default()
    };
    too_slow.sanitize();
    assert!((too_slow.autosave_interval - 600.0).abs() < 1e-6);
    let mut bad_speed = GameSettings {
        default_speed: 9,
        ..Default::default()
    };
    bad_speed.sanitize();
    assert_eq!(bad_speed.default_speed, 4);
}

#[test]
fn sanitize_clamps_out_of_range_values() {
    let mut settings = GameSettings {
        master_volume: 5.0,
        sfx_volume: -1.0,
        ui_text_scale: 100.0,
        ..Default::default()
    };
    settings.sanitize();
    assert!((settings.master_volume - 1.0).abs() < 1e-6);
    assert!(settings.sfx_volume.abs() < 1e-6);
    assert!((settings.ui_text_scale - 4.0).abs() < 1e-6);
}

#[test]
fn round_trips_through_json() {
    let settings = GameSettings {
        music_volume: 0.3,
        show_fps: true,
        ..Default::default()
    };
    let json = serde_json::to_string(&settings).unwrap();
    let back: GameSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(settings, back);
}
