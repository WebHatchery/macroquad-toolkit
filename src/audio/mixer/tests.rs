use super::*;

#[test]
fn groups_obey_master_focus_and_finite_bounds() {
    let settings = GameSettings {
        master_volume: 0.5,
        voice_volume: 0.4,
        mute_when_unfocused: true,
        ..Default::default()
    };
    assert!((group_volume(&settings, AudioGroup::Voice, true) - 0.2).abs() < 1e-6);
    for group in [
        AudioGroup::Sfx,
        AudioGroup::Music,
        AudioGroup::Voice,
        AudioGroup::Ui,
        AudioGroup::Ambience,
    ] {
        assert_eq!(group_volume(&settings, group, false), 0.0);
    }
    let invalid = GameSettings {
        master_volume: f32::NAN,
        ui_volume: f32::INFINITY,
        ..Default::default()
    };
    assert_eq!(group_volume(&invalid, AudioGroup::Ui, true), 1.0);
}
