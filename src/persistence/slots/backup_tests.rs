use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Fixture {
    value: u32,
}

#[test]
fn versioned_backup_save_and_restore_preserve_both_sides_of_the_swap() {
    const GAME: &str = "toolkit_slot_backup_restore_test";
    const SLOT: &str = "campaign";

    save_to_slot_with_version(GAME, SLOT, &Fixture { value: 1 }, "1").unwrap();
    save_to_slot_with_version_and_backup(GAME, SLOT, &Fixture { value: 2 }, "2").unwrap();
    assert!(slot_backup_exists(GAME, SLOT));
    assert_eq!(load_from_slot::<Fixture>(GAME, SLOT).unwrap().value, 2);
    assert_eq!(
        load_from_slot::<Fixture>(GAME, "campaign_backup")
            .unwrap()
            .value,
        1
    );

    let displaced = restore_slot_backup(GAME, SLOT).unwrap();
    assert_eq!(displaced.as_deref(), Some("campaign_before_restore"));
    assert_eq!(load_from_slot::<Fixture>(GAME, SLOT).unwrap().value, 1);
    assert_eq!(
        load_from_slot::<Fixture>(GAME, "campaign_before_restore")
            .unwrap()
            .value,
        2
    );

    for name in [SLOT, "campaign_backup", "campaign_before_restore"] {
        delete_slot(GAME, name).unwrap();
    }
}

#[test]
fn restoring_without_a_backup_fails_without_touching_the_primary() {
    const GAME: &str = "toolkit_slot_missing_backup_test";
    const SLOT: &str = "campaign";

    save_to_slot_with_version(GAME, SLOT, &Fixture { value: 7 }, "1").unwrap();
    assert!(restore_slot_backup(GAME, SLOT).is_err());
    assert_eq!(load_from_slot::<Fixture>(GAME, SLOT).unwrap().value, 7);
    delete_slot(GAME, SLOT).unwrap();
}
