//! The catalogue protocol uses an in-memory failure store; no player saves.
#[path = "support/indexed.rs"]
mod support;

use macroquad_toolkit::persistence::{IndexedCatalogue, WriterStatus};
use serde::{Deserialize, Serialize};
use serde_json::json;
use support::MemoryStore;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Metadata {
    name: String,
    schema: u32,
}

fn metadata(name: &str) -> Metadata {
    Metadata {
        name: name.into(),
        schema: 2,
    }
}

fn validate(metadata: &Metadata, raw: &str) -> Result<(), String> {
    if metadata.schema != 2 || !raw.starts_with("v2:") {
        return Err("unsupported or invalid campaign".into());
    }
    Ok(())
}

fn catalogue(store: &mut MemoryStore) -> IndexedCatalogue<Metadata> {
    let mut catalogue = IndexedCatalogue::new("campaigns").unwrap();
    catalogue.refresh(store, validate).unwrap();
    catalogue
}

#[test]
fn unlimited_discovery_and_external_identities_survive_reopening() {
    let mut store = MemoryStore::default();
    let mut library = catalogue(&mut store);
    let campaign = library.reserve_identity(&mut store).unwrap();
    let mut ids = Vec::new();
    for index in 0..32 {
        let commit = library
            .write(
                &mut store,
                &format!("manual_{index}"),
                None,
                metadata("A name / with paths: only a label"),
                &format!("v2:{index}"),
                validate,
            )
            .unwrap();
        assert!(commit.entry_id > campaign);
        ids.push(commit.entry_id);
    }
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    let mut restarted = store.restart();
    let mut library = catalogue(&mut restarted);
    assert_eq!(library.entries().len(), 32);
    for (index, id) in ids.iter().enumerate() {
        assert_eq!(
            library.load(&restarted, *id).unwrap(),
            format!("v2:{index}")
        );
    }
    assert!(library.reserve_identity(&mut restarted).unwrap() > *ids.last().unwrap());
    assert!(restarted
        .values
        .keys()
        .all(|key| key.starts_with("campaigns_")));
}

#[test]
fn every_interrupted_and_failed_stage_preserves_previous_saves_and_recovers() {
    let mut seed = MemoryStore::default();
    let mut library = catalogue(&mut seed);
    let keep = library
        .write(
            &mut seed,
            "keep",
            None,
            metadata("Keep"),
            "v2:keep",
            validate,
        )
        .unwrap()
        .entry_id;
    let target = library
        .write(
            &mut seed,
            "target",
            None,
            metadata("Target"),
            "v2:old",
            validate,
        )
        .unwrap()
        .entry_id;
    for mode in ["create", "overwrite", "delete"] {
        let apply = |library: &mut IndexedCatalogue<Metadata>, store: &mut MemoryStore| {
            if mode == "delete" {
                library.delete(store, "operation", target)
            } else {
                library.write(
                    store,
                    "operation",
                    (mode == "overwrite").then_some(target),
                    metadata("New"),
                    "v2:new",
                    validate,
                )
            }
        };
        let mut complete = seed.restart();
        let mut library = catalogue(&mut complete);
        let expected = apply(&mut library, &mut complete).unwrap().entry_id;
        let mut snapshots = vec![seed.values.clone()];
        snapshots.extend(complete.snapshots.clone());
        // Each snapshot is a process exit immediately after a durable mutation.
        for values in snapshots {
            let mut restarted = MemoryStore {
                values,
                ..MemoryStore::default()
            };
            let mut library = catalogue(&mut restarted);
            assert_eq!(library.load(&restarted, keep).unwrap(), "v2:keep");
            let result = apply(&mut library, &mut restarted).unwrap();
            assert_eq!(result.entry_id, expected, "{mode} retry identity");
            if mode == "delete" {
                assert!(library.load(&restarted, target).is_err());
            } else {
                assert_eq!(library.load(&restarted, result.entry_id).unwrap(), "v2:new");
            }
        }
        // Also fail immediately before every write or removal.
        for stage in 1..=complete.mutations {
            let mut failing = seed.restart();
            failing.fail_mutation = Some(stage);
            let mut library = catalogue(&mut failing);
            let _ = apply(&mut library, &mut failing);
            let mut restarted = failing.restart();
            let mut library = catalogue(&mut restarted);
            assert_eq!(library.load(&restarted, keep).unwrap(), "v2:keep");
            let result = apply(&mut library, &mut restarted).unwrap();
            assert_eq!(result.entry_id, expected, "{mode} failed stage {stage}");
            assert!(apply(&mut library, &mut restarted).unwrap().replayed);
        }
    }
}

#[test]
fn overwrite_delete_and_retry_are_explicit_and_idempotent() {
    let mut store = MemoryStore::default();
    let mut library = catalogue(&mut store);
    let first = library
        .write(
            &mut store,
            "one",
            None,
            metadata("Same name"),
            "v2:one",
            validate,
        )
        .unwrap()
        .entry_id;
    let second = library
        .write(
            &mut store,
            "two",
            None,
            metadata("Same name"),
            "v2:two",
            validate,
        )
        .unwrap()
        .entry_id;
    assert_ne!(first, second);
    let replaced = library
        .write(
            &mut store,
            "replace",
            Some(first),
            metadata("Renamed"),
            "v2:three",
            validate,
        )
        .unwrap();
    assert_eq!(replaced.entry_id, first);
    assert!(library
        .write(
            &mut store,
            "one",
            None,
            metadata("Old creation"),
            "v2:one",
            validate
        )
        .is_err());
    let before = store.values.clone();
    assert!(
        library
            .write(
                &mut store,
                "replace",
                Some(first),
                metadata("Ignored on retry"),
                "v2:three",
                validate
            )
            .unwrap()
            .replayed
    );
    assert_eq!(store.values, before);
    assert_eq!(library.entries()[0].metadata.name, "Renamed");
    assert!(library
        .write(
            &mut store,
            "replace",
            Some(first),
            metadata("Conflict"),
            "v2:four",
            validate
        )
        .is_err());
    assert!(library
        .write(
            &mut store,
            "missing",
            Some(u64::MAX),
            metadata("Missing"),
            "v2:new",
            validate
        )
        .is_err());
    assert_eq!(store.values, before);
    library.delete(&mut store, "delete", first).unwrap();
    assert!(library
        .write(
            &mut store,
            "replace",
            Some(first),
            metadata("Deleted save"),
            "v2:three",
            validate
        )
        .is_err());
    assert!(
        library
            .delete(&mut store, "delete", first)
            .unwrap()
            .replayed
    );
    assert_eq!(library.entries().len(), 1);
    assert_eq!(library.load(&store, second).unwrap(), "v2:two");
    assert!(library.delete(&mut store, "new_delete", first).is_err());
}

#[test]
fn rejected_payloads_corrupt_indexes_and_read_failures_preserve_bytes() {
    let mut store = MemoryStore::default();
    let mut library = catalogue(&mut store);
    let good = library
        .write(
            &mut store,
            "good",
            None,
            metadata("Good"),
            "v2:good",
            validate,
        )
        .unwrap()
        .entry_id;
    let pristine = store.values.clone();
    assert!(library
        .write(
            &mut store,
            "bad",
            None,
            metadata("Bad"),
            "v9:future",
            validate
        )
        .is_err());
    assert_eq!(store.values, pristine);
    // Fail index publication, leaving a payload to validate on restart.
    store.fail_mutation = Some(store.mutations + 3);
    assert!(library
        .write(
            &mut store,
            "pending",
            None,
            metadata("Pending"),
            "v2:pending",
            validate
        )
        .is_err());
    let pending_key = store
        .values
        .iter()
        .find(|(_, raw)| raw.as_str() == "v2:pending")
        .unwrap()
        .0
        .clone();
    store.values.insert(pending_key, "corrupt".into());
    store.fail_mutation = None;
    let report = library.refresh(&mut store, validate).unwrap();
    assert_eq!(report.abandoned, ["pending"]);
    assert_eq!(library.load(&store, good).unwrap(), "v2:good");
    library
        .write(
            &mut store,
            "pending",
            None,
            metadata("Pending"),
            "v2:pending",
            validate,
        )
        .unwrap();
    let valid_values = store.values.clone();
    for raw in ["not json".to_owned(), {
        let mut value: serde_json::Value =
            serde_json::from_str(&valid_values["campaigns_index"]).unwrap();
        value["format"] = json!(99);
        value.to_string()
    }] {
        store.values.insert("campaigns_index".into(), raw);
        let before = store.values.clone();
        assert!(library.refresh(&mut store, validate).is_err());
        assert!(library.reserve_identity(&mut store).is_err());
        assert_eq!(store.values, before);
    }
    store.values = valid_values;
    store.fail_read = Some("campaigns_index".into());
    let before = store.values.clone();
    assert!(library.refresh(&mut store, validate).is_err());
    assert_eq!(store.values, before);
    malformed_journals_cannot_delete_listed_payloads();
}

fn malformed_journals_cannot_delete_listed_payloads() {
    let mut store = MemoryStore::default();
    let mut library = catalogue(&mut store);
    library
        .write(
            &mut store,
            "good",
            None,
            metadata("Good"),
            "v2:good",
            validate,
        )
        .unwrap();
    store.fail_mutation = Some(store.mutations + 3);
    assert!(library
        .write(
            &mut store,
            "pending",
            None,
            metadata("Pending"),
            "v2:pending",
            validate
        )
        .is_err());
    let pending = store.restart();
    let original: serde_json::Value =
        serde_json::from_str(&pending.values["campaigns_index"]).unwrap();
    let listed_payload = original["entries"][0]["payload"].clone();
    let mut aliased = original.clone();
    aliased["pending"]["entry"]["payload"] = listed_payload.clone();
    aliased["operations"]["pending"]["payload"] = listed_payload.clone();
    let mut wrong_cleanup = original.clone();
    wrong_cleanup["pending"]["cleanup"] = listed_payload.clone();
    let mut wrong_target = original.clone();
    wrong_target["operations"]["pending"]["kind"]["Write"]["target"] =
        original["entries"][0]["id"].clone();
    let mut aliased_suffix = original.clone();
    let noncanonical = json!(format!(
        "campaigns_payload_0{}",
        original["next_identity"].as_u64().unwrap() - 1
    ));
    aliased_suffix["pending"]["entry"]["payload"] = noncanonical.clone();
    aliased_suffix["operations"]["pending"]["payload"] = noncanonical;
    for corrupt in [aliased, wrong_cleanup, wrong_target, aliased_suffix] {
        let mut tested = pending.restart();
        tested
            .values
            .insert("campaigns_index".into(), corrupt.to_string());
        let before = tested.values.clone();
        let mut library = IndexedCatalogue::<Metadata>::new("campaigns").unwrap();
        assert!(library.refresh(&mut tested, validate).is_err());
        assert_eq!(
            tested.values, before,
            "malformed recovery must be read-only"
        );
    }
    let mut busy = pending.restart();
    busy.writer = WriterStatus::Busy;
    let mut readonly = IndexedCatalogue::<Metadata>::new("campaigns").unwrap();
    let report = readonly.refresh(&mut busy, validate).unwrap();
    assert_eq!(report.pending.as_deref(), Some("pending"));
    assert_eq!(readonly.entries().len(), 1);
    assert_eq!(busy.values, pending.values);
}

#[test]
fn stale_and_busy_writers_cannot_mutate_and_native_replacement_holds_its_lease() {
    let mut store = MemoryStore::default();
    let mut first = catalogue(&mut store);
    let mut stale = catalogue(&mut store);
    first.reserve_identity(&mut store).unwrap();
    let before = store.values.clone();
    assert!(stale.reserve_identity(&mut store).is_err());
    assert_eq!(store.values, before);
    for status in [WriterStatus::Pending, WriterStatus::Busy] {
        store.writer = status;
        stale.refresh(&mut store, validate).unwrap();
        assert!(stale.reserve_identity(&mut store).is_err());
        assert_eq!(store.values, before);
    }
    store.writer = WriterStatus::Ready;
    let mut value: serde_json::Value = serde_json::from_str(&before["campaigns_index"]).unwrap();
    value["next_identity"] = json!(u64::MAX);
    store
        .values
        .insert("campaigns_index".into(), value.to_string());
    stale.refresh(&mut store, validate).unwrap();
    assert!(stale.reserve_identity(&mut store).is_err());
    assert!(IndexedCatalogue::<Metadata>::new("../escape").is_err());
    #[cfg(not(target_arch = "wasm32"))]
    native_lease_and_atomic_replacement();
}

#[cfg(not(target_arch = "wasm32"))]
fn native_lease_and_atomic_replacement() {
    use macroquad_toolkit::persistence::{
        get_app_data_path, IndexedKeyStore, IndexedSaveStore, RawSaveStore,
    };
    // A distinct toolkit verification identity uses the real native backend.
    // The test owns and cleans only its generated keys and the backend lock file.
    let game = "macroquad_toolkit_indexed_verification";
    let namespace = format!("test_{}", std::process::id());
    let key = format!("{namespace}_probe");
    let path = get_app_data_path(game, &format!("indexed.{namespace}.{key}.json")).unwrap();
    let mut first = IndexedKeyStore::new(game, &namespace).unwrap();
    let mut second = IndexedKeyStore::new(game, &namespace).unwrap();
    assert_eq!(first.poll_writer().unwrap(), WriterStatus::Ready);
    assert_eq!(second.poll_writer().unwrap(), WriterStatus::Busy);
    first.write(&key, "original").unwrap();
    assert!(second.write(&key, "blocked").is_err());
    first.write(&key, "replacement").unwrap();
    assert_eq!(second.read(&key).unwrap().as_deref(), Some("replacement"));
    let mut native_catalogue = IndexedCatalogue::new(&namespace).unwrap();
    native_catalogue.refresh(&mut first, validate).unwrap();
    let mut saved = Vec::new();
    for index in 0..8 {
        let raw = format!("v2:native_{index}");
        let id = native_catalogue
            .write(
                &mut first,
                &format!("native_{index}"),
                None,
                metadata("Native restart"),
                &raw,
                validate,
            )
            .unwrap()
            .entry_id;
        saved.push((id, raw));
    }
    let child_namespace = format!("{namespace}_child");
    let child_key = format!("{child_namespace}_probe");
    let mut child = IndexedKeyStore::new(game, &child_namespace).unwrap();
    assert_eq!(child.poll_writer().unwrap(), WriterStatus::Ready);
    first.write(&child_key, "parent namespace").unwrap();
    child.write(&child_key, "child namespace").unwrap();
    assert_eq!(
        first.read(&child_key).unwrap().as_deref(),
        Some("parent namespace")
    );
    assert_eq!(
        child.read(&child_key).unwrap().as_deref(),
        Some("child namespace")
    );
    let mut wrong_catalogue = IndexedCatalogue::<Metadata>::new(&child_namespace).unwrap();
    assert!(wrong_catalogue.refresh(&mut first, validate).is_err());
    drop(child);
    for name in [
        format!("indexed.{namespace}.{child_key}.json"),
        format!("indexed.{child_namespace}.{child_key}.json"),
        format!(".{child_namespace}_writer.lock"),
    ] {
        std::fs::remove_file(get_app_data_path(game, &name).unwrap()).unwrap();
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        assert!(first.write(&key, "denied replacement").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "replacement");
        drop(held);
    }
    drop(first);
    assert_eq!(second.retry_writer().unwrap(), WriterStatus::Ready);
    second.write(&key, "after release").unwrap();
    drop(second);
    let mut reopened = IndexedKeyStore::new(game, &namespace).unwrap();
    assert_eq!(reopened.poll_writer().unwrap(), WriterStatus::Ready);
    let mut discovered = IndexedCatalogue::new(&namespace).unwrap();
    discovered.refresh(&mut reopened, validate).unwrap();
    assert_eq!(discovered.entries().len(), 8);
    for (id, raw) in saved {
        assert_eq!(discovered.load(&reopened, id).unwrap(), raw);
    }
    let index_key = format!("{namespace}_index");
    let control: serde_json::Value =
        serde_json::from_str(&reopened.read(&index_key).unwrap().unwrap()).unwrap();
    for entry in control["entries"].as_array().unwrap() {
        reopened.remove(entry["payload"].as_str().unwrap()).unwrap();
    }
    reopened.remove(&index_key).unwrap();
    reopened.remove(&key).unwrap();
    drop(reopened);
    std::fs::remove_file(get_app_data_path(game, &format!(".{namespace}_writer.lock")).unwrap())
        .unwrap();
    // Empty directory removal never removes another concurrently running test's files.
    let _ = std::fs::remove_dir(path.parent().unwrap());
}
