use super::*;

#[test]
fn older_settings_preserve_game_defaults_for_missing_fields() {
    let defaults = GameSettings {
        autosave_enabled: false,
        voice_volume: 0.7,
        ..Default::default()
    };
    let saved = serde_json::json!({"music_volume": 0.25});
    let loaded = GameSettings::overlay_defaults(saved, &defaults).unwrap();
    assert!(!loaded.autosave_enabled);
    assert_eq!(loaded.voice_volume, 0.7);
    assert_eq!(loaded.music_volume, 0.25);
}

#[test]
fn failed_save_preserves_committed_snapshot_and_cancel_restores_it() {
    let mut editor = SettingsSession::new(GameSettings::default(), GameSettings::default());
    editor.draft.music_volume = 0.1;
    assert!(editor.commit_with(|_| Err("disk full".into())).is_err());
    assert!(editor.is_dirty());
    editor.cancel();
    assert_eq!(editor.draft.music_volume, 0.8);
    editor.draft.music_volume = 0.2;
    editor.commit_with(|_| Ok(())).unwrap();
    editor.reset_defaults();
    editor.cancel();
    assert_eq!(editor.draft.music_volume, 0.2);
}

#[test]
fn nonfinite_values_are_repaired_and_autosave_is_wired() {
    let mut settings = GameSettings {
        master_volume: f32::NAN,
        ui_text_scale: f32::INFINITY,
        autosave_enabled: false,
        autosave_interval: 90.0,
        ..Default::default()
    };
    settings.sanitize();
    assert_eq!(settings.master_volume, 1.0);
    assert_eq!(settings.ui_text_scale, 1.0);
    let mut scheduler = crate::persistence::AutoSaveManager::default();
    settings.apply_autosave(&mut scheduler);
    assert!(!scheduler.is_enabled());
    assert_eq!(scheduler.interval_seconds(), 90.0);
}

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
    assert_eq!(settings.ui_scale, 1.0);
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
        ui_scale: 1.25,
        ..Default::default()
    };
    let json = serde_json::to_string(&settings).unwrap();
    let back: GameSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(settings, back);
}
