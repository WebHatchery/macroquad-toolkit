use super::*;

/// The bug, stated: two games with the same slot name must not collide.
///
/// `localStorage` is per origin, and every game in this workspace is
/// published to the same host. Three of them shipped with `save_slot:
/// "autosave"` and two more with `"campaign"`, so playing one overwrote
/// another's save (§5.56). The native path never had the problem because a
/// directory per game did the qualifying for free.
#[test]
fn two_games_with_the_same_slot_get_different_keys() {
    let one = storage_key("dragons_den", "autosave");
    let other = storage_key("biofoundry", "autosave");
    assert_ne!(one, other, "two games would share a browser save");
    assert!(one.contains("dragons_den"));
    assert!(other.contains("biofoundry"));
}

/// And one game's two slots still differ, which is the thing that was
/// already working and must not be broken by fixing the other.
#[test]
fn one_game_keeps_its_slots_apart() {
    assert_ne!(
        storage_key("dragons_hoard", "dragon_autosave"),
        storage_key("dragons_hoard", "wallet")
    );
}

/// The migration path: the old key is still recognised, and is not the same
/// as the new one, or the fallback would be reading itself.
#[test]
fn the_legacy_key_is_what_the_old_build_wrote() {
    assert_eq!(legacy_storage_key("autosave"), "save_autosave");
    assert_ne!(
        legacy_storage_key("autosave"),
        storage_key("dragons_den", "autosave")
    );
}

/// A game name with a path separator in it must not produce a key that
/// looks like two keys — the same sanitising the sibling module has always
/// done, now shared rather than duplicated.
#[test]
fn a_hostile_game_name_is_sanitised() {
    let key = storage_key("../other", "autosave");
    assert!(!key.contains('/'), "{}", key);
    assert!(!key.contains('\\'), "{}", key);
}
