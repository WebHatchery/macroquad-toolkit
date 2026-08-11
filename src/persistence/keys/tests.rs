use super::*;

#[test]
fn test_key_file_name_sanitizes_paths() {
    assert_eq!(key_file_name("profile/settings"), "profile_settings.json");
    assert_eq!(key_file_name("save.json"), "save.json");
}
