use super::*;

#[test]
fn envelope_decode_preserves_migration_version_and_payload() {
    let raw = encode_slot("campaign_backup_1", &42u32, "1.0").unwrap();
    let value: u32 =
        decode_slot_with_migration(&raw, "1.0", |_, _| panic!("current save")).unwrap();
    assert_eq!(value, 42);
    let migrated: u32 = decode_slot_with_migration(&raw, "2.0", |version, value| {
        assert_eq!(version.as_deref(), Some("1.0"));
        assert_eq!(value["slot"]["name"], "campaign_backup_1");
        Ok(value["data"].as_u64().unwrap() as u32 + 1)
    })
    .unwrap();
    assert_eq!(migrated, 43);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn raw_store_and_existing_slots_share_paths_and_exact_backup_bytes() {
    use crate::persistence::{BackupChain, RawSaveStore};
    let game = format!("toolkit_raw_slot_test_{}", std::process::id());
    let mut store = SlotSaveStore { game_name: &game };
    let primary = "campaign";
    let backup = "campaign_backup_1";
    super::super::save_to_slot_with_version(&game, primary, &42u32, "old").unwrap();
    let original = store.read(primary).unwrap().unwrap();
    let chain = BackupChain::new(primary, vec![backup.to_owned()]);
    let next = encode_slot(primary, &99u32, "new").unwrap();
    chain.save(&mut store, &next, |_| Ok(())).unwrap();
    assert_eq!(store.read(backup).unwrap().unwrap(), original);
    assert_eq!(
        super::super::load_from_slot::<u32>(&game, backup).unwrap(),
        42
    );
    assert_eq!(
        super::super::peek_slot_version(&game, backup)
            .unwrap()
            .as_deref(),
        Some("old")
    );
    assert_eq!(
        super::super::load_from_slot::<u32>(&game, primary).unwrap(),
        99
    );
    super::super::delete_slot(&game, primary).unwrap();
    super::super::delete_slot(&game, backup).unwrap();
}
