//! On-disk control record. Payload bytes never enter the journal or receipts.

use super::{IndexedEntry, SaveCommit};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Control<M> {
    pub format: u32,
    pub revision: u64,
    pub next_identity: u64,
    pub entries: Vec<IndexedEntry<M>>,
    pub operations: BTreeMap<String, Outcome>,
    pub pending: Option<Pending<M>>,
}

impl<M> Default for Control<M> {
    fn default() -> Self {
        Self {
            format: 1,
            revision: 0,
            next_identity: 1,
            entries: Vec::new(),
            operations: BTreeMap::new(),
            pending: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum OperationKind {
    Write { target: Option<u64> },
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum OperationStatus {
    Reserved,
    Committed,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Outcome {
    pub kind: OperationKind,
    pub entry_id: u64,
    pub fingerprint: String,
    pub payload: Option<String>,
    pub previous_payload: Option<String>,
    pub status: OperationStatus,
}

impl Outcome {
    pub fn receipt(&self, replayed: bool, warnings: Vec<String>) -> SaveCommit {
        SaveCommit {
            entry_id: self.entry_id,
            replayed,
            warnings,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pending<M> {
    pub operation: String,
    pub entry: Option<IndexedEntry<M>>,
    pub cleanup: Option<String>,
}

/// A deterministic corruption/request-conflict check, not a security signature.
pub(super) fn fingerprint(raw: &str) -> String {
    let mut hash: u128 = 0x6c62272e07bb014262b821756295c58d;
    for byte in raw.bytes() {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(0x0000000001000000000000000000013b);
    }
    format!("{}:{hash:032x}", raw.len())
}

impl<M> Control<M> {
    pub fn allocate(&mut self) -> Result<u64, String> {
        let identity = self.next_identity;
        self.next_identity = identity
            .checked_add(1)
            .ok_or_else(|| "Save identity capacity exhausted".to_owned())?;
        Ok(identity)
    }

    pub fn validate(&self, namespace: &str) -> Result<(), String> {
        if self.format != 1 || self.next_identity == 0 {
            return Err("Unsupported or corrupt indexed save control record".into());
        }
        let prefix = format!("{namespace}_payload_");
        let valid_payload = |key: &str| {
            key.strip_prefix(&prefix)
                .and_then(|value| {
                    value
                        .parse::<u64>()
                        .ok()
                        .filter(|id| id.to_string() == value)
                })
                .is_some_and(|id| id > 0 && id < self.next_identity)
        };
        let valid_id = |id| id > 0 && id < self.next_identity;
        let mut ids = BTreeSet::new();
        let mut payloads = BTreeSet::new();
        for entry in &self.entries {
            if !valid_id(entry.id)
                || !ids.insert(entry.id)
                || !valid_payload(&entry.payload)
                || !payloads.insert(&entry.payload)
            {
                return Err("Corrupt save catalogue entry identity or payload reference".into());
            }
        }
        for (token, outcome) in &self.operations {
            if token.is_empty()
                || !valid_id(outcome.entry_id)
                || outcome
                    .previous_payload
                    .as_deref()
                    .is_some_and(|key| !valid_payload(key))
                || outcome
                    .payload
                    .as_deref()
                    .is_some_and(|key| !valid_payload(key))
            {
                return Err("Corrupt save operation receipt".into());
            }
            match outcome.kind {
                OperationKind::Write { target } => {
                    if outcome.payload.is_none()
                        || target.is_some_and(|id| id != outcome.entry_id)
                        || target.is_some() != outcome.previous_payload.is_some()
                    {
                        return Err("Invalid save operation target or payload".into());
                    }
                }
                OperationKind::Delete => {
                    if outcome.payload.is_some() || outcome.previous_payload.is_none() {
                        return Err("Invalid deletion operation payload".into());
                    }
                }
            }
            if outcome.status == OperationStatus::Reserved
                && self
                    .pending
                    .as_ref()
                    .is_none_or(|pending| &pending.operation != token)
            {
                return Err("Reserved save operation has no matching journal".into());
            }
        }
        if let Some(pending) = &self.pending {
            let outcome = self
                .operations
                .get(&pending.operation)
                .ok_or_else(|| "Pending save has no operation receipt".to_owned())?;
            if outcome.status == OperationStatus::Abandoned {
                return Err("Abandoned save still has an active journal".into());
            }
            match (&outcome.kind, &pending.entry) {
                (OperationKind::Write { .. }, Some(entry))
                    if entry.id == outcome.entry_id
                        && Some(&entry.payload) == outcome.payload.as_ref()
                        && valid_payload(&entry.payload) => {}
                (OperationKind::Delete, None) => {}
                _ => return Err("Pending save operation does not match its payload".into()),
            }
            if pending.cleanup != outcome.previous_payload {
                return Err("Pending save has an invalid cleanup reference".into());
            }
            let listed = self
                .entries
                .iter()
                .find(|entry| entry.id == outcome.entry_id);
            if outcome.status == OperationStatus::Reserved {
                if listed.map(|entry| &entry.payload) != outcome.previous_payload.as_ref()
                    || outcome
                        .payload
                        .as_ref()
                        .is_some_and(|key| payloads.contains(key))
                {
                    return Err("Pending save aliases a listed payload or changed target".into());
                }
            } else if listed.map(|entry| &entry.payload) != outcome.payload.as_ref()
                || pending
                    .cleanup
                    .as_ref()
                    .is_some_and(|key| payloads.contains(key))
            {
                return Err("Committed save journal does not match the catalogue".into());
            }
        }
        Ok(())
    }
}
