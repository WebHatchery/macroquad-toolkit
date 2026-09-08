use super::*;

#[test]
fn display_timeout_only_fires_once_and_confirmation_disarms_it() {
    let mut preview = DisplayPreview::new(GameSettings::default());
    assert!(!preview.elapse(14.0));
    assert!(preview.elapse(1.0));
    assert!(!preview.elapse(20.0));
    let mut confirmed = DisplayPreview::new(GameSettings::default());
    confirmed.confirm();
    assert!(!confirmed.elapse(20.0));
}
