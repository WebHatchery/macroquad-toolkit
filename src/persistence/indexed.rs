//! Unlimited indexed save discovery with fresh payloads and recoverable writes.
//!
//! Games own metadata, payload schemas, migration, and semantic validation. The
//! toolkit owns allocation, writer exclusion, journal recovery, and publication.
//! Call `refresh` before working with an existing catalogue and after a failure.

mod record;
mod recovery;

use super::indexed_store::{require_writer, valid_name};
use super::{IndexedSaveStore, RawSaveStore};
use record::{fingerprint, Control, OperationKind, OperationStatus, Outcome, Pending};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// One stable catalogue identity. Overwriting changes its payload, never its ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedEntry<M> {
    pub id: u64,
    pub metadata: M,
    payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCommit {
    pub entry_id: u64,
    /// This operation token had already committed; its original result survives.
    pub replayed: bool,
    /// Publication succeeded, but a cleanup stage may still require retry.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    pub recovered: Vec<String>,
    /// Published write IDs, including a prior commit whose cleanup is resumed.
    pub committed_writes: Vec<u64>,
    pub abandoned: Vec<String>,
    pub pending: Option<String>,
    pub warnings: Vec<String>,
}

/// One caller's snapshot. A stale writer is rejected before any storage mutation.
pub struct IndexedCatalogue<M> {
    namespace: String,
    control: Control<M>,
    observed: Option<String>,
}

impl<M: Clone + Serialize + DeserializeOwned> IndexedCatalogue<M> {
    pub fn new(namespace: &str) -> Result<Self, String> {
        if !valid_name(namespace) {
            return Err("Save namespace must use lowercase ASCII names, digits, _ or -".into());
        }
        Ok(Self {
            namespace: namespace.into(),
            control: Control::default(),
            observed: None,
        })
    }

    pub fn entries(&self) -> &[IndexedEntry<M>] {
        &self.control.entries
    }

    fn control_key(&self) -> String {
        format!("{}_index", self.namespace)
    }

    fn payload_key(&self, identity: u64) -> String {
        format!("{}_payload_{identity}", self.namespace)
    }

    fn ensure_current(&self, store: &mut impl IndexedSaveStore) -> Result<(), String> {
        self.check_namespace(store)?;
        require_writer(store)?;
        if store.read(&self.control_key())? != self.observed {
            return Err("The save catalogue changed; refresh it before retrying".into());
        }
        Ok(())
    }

    fn check_namespace(&self, store: &impl IndexedSaveStore) -> Result<(), String> {
        if store
            .namespace()
            .is_some_and(|namespace| namespace != self.namespace)
        {
            return Err("Save catalogue does not belong to the leased store namespace".into());
        }
        Ok(())
    }

    fn commit_control(
        &mut self,
        store: &mut impl IndexedSaveStore,
        mut control: Control<M>,
    ) -> Result<(), String> {
        control.revision = self
            .control
            .revision
            .checked_add(1)
            .ok_or_else(|| "Save catalogue revision capacity exhausted".to_owned())?;
        control.validate(&self.namespace)?;
        let raw = serde_json::to_string(&control)
            .map_err(|error| format!("Save catalogue encoding: {error}"))?;
        store.write(&self.control_key(), &raw)?;
        self.observed = Some(raw);
        self.control = control;
        Ok(())
    }

    /// Durably reserves a unique external identity without consuming game RNG.
    /// A crash after reservation can leave a gap; an identity is never reused.
    pub fn reserve_identity(&mut self, store: &mut impl IndexedSaveStore) -> Result<u64, String> {
        self.ensure_current(store)?;
        self.require_no_pending()?;
        let mut next = self.control.clone();
        let identity = next.allocate()?;
        self.commit_control(store, next)?;
        Ok(identity)
    }

    /// Returns exact payload bytes. Games validate the candidate before loading it.
    pub fn load(&self, store: &impl RawSaveStore, id: u64) -> Result<String, String> {
        let entry = self
            .entries()
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| format!("Save {id} is not in this catalogue"))?;
        store.read(&entry.payload)?.ok_or_else(|| {
            format!("Save {id} payload is missing; its catalogue entry was preserved")
        })
    }

    fn require_no_pending(&self) -> Result<(), String> {
        if self.control.pending.is_some() {
            return Err("A save operation needs recovery; refresh the catalogue first".into());
        }
        Ok(())
    }

    fn check_operation(
        &self,
        operation: &str,
        kind: &OperationKind,
        fingerprint: &str,
    ) -> Result<Option<&Outcome>, String> {
        if operation.trim().is_empty() {
            return Err("A save operation needs a nonempty retry token".into());
        }
        let outcome = self.control.operations.get(operation);
        if outcome
            .is_some_and(|outcome| &outcome.kind != kind || outcome.fingerprint != fingerprint)
        {
            return Err("This save retry token already belongs to a different operation".into());
        }
        Ok(outcome)
    }

    /// New entries use `None`; overwriting requires an explicit existing ID.
    /// Retry the same immutable payload with the same token. Committed retries
    /// retain the original metadata. Abandoned attempts retain the reserved ID.
    #[allow(clippy::too_many_arguments)]
    pub fn write(
        &mut self,
        store: &mut impl IndexedSaveStore,
        operation: &str,
        target: Option<u64>,
        metadata: M,
        raw: &str,
        validate: impl Fn(&M, &str) -> Result<(), String>,
    ) -> Result<SaveCommit, String> {
        self.ensure_current(store)?;
        validate(&metadata, raw).map_err(|error| format!("New save rejected: {error}"))?;
        let kind = OperationKind::Write { target };
        let signature = fingerprint(raw);
        if let Some(outcome) = self.check_operation(operation, &kind, &signature)? {
            if outcome.status == OperationStatus::Committed {
                if !self.entries().iter().any(|entry| {
                    entry.id == outcome.entry_id && Some(&entry.payload) == outcome.payload.as_ref()
                }) {
                    return Err("This save completed earlier, but its entry was subsequently replaced or deleted".into());
                }
                return Ok(outcome.receipt(true, self.pending_warning()));
            }
        }
        if self
            .control
            .pending
            .as_ref()
            .is_some_and(|pending| pending.operation == operation)
        {
            let payload = self
                .control
                .pending
                .as_ref()
                .and_then(|pending| pending.entry.as_ref())
                .ok_or_else(|| "Pending write has no payload reference".to_owned())?
                .payload
                .clone();
            store.write(&payload, raw)?;
            return self.finish_write(store, operation, &validate);
        }
        self.require_no_pending()?;
        let previous = target
            .map(|id| {
                self.entries()
                    .iter()
                    .find(|entry| entry.id == id)
                    .map(|entry| entry.payload.clone())
                    .ok_or_else(|| format!("Cannot overwrite missing save {id}"))
            })
            .transpose()?;
        let mut next = self.control.clone();
        let id = if let Some(outcome) = next.operations.get(operation) {
            if outcome.previous_payload != previous {
                return Err("The overwrite target changed since this failed attempt; create a new operation".into());
            }
            outcome.entry_id
        } else if let Some(id) = target {
            id
        } else {
            next.allocate()?
        };
        let payload = self.payload_key(next.allocate()?);
        // Even malformed external data must never make an allocation overwrite bytes.
        if store.read(&payload)?.is_some() {
            return Err("Reserved save payload key already exists; catalogue preserved".into());
        }
        next.operations.insert(
            operation.into(),
            Outcome {
                kind,
                entry_id: id,
                fingerprint: signature,
                payload: Some(payload.clone()),
                previous_payload: previous.clone(),
                status: OperationStatus::Reserved,
            },
        );
        next.pending = Some(Pending {
            operation: operation.into(),
            entry: Some(IndexedEntry {
                id,
                metadata,
                payload: payload.clone(),
            }),
            cleanup: previous,
        });
        self.commit_control(store, next)?;
        store.write(&payload, raw)?;
        self.finish_write(store, operation, &validate)
    }

    fn finish_write(
        &mut self,
        store: &mut impl IndexedSaveStore,
        operation: &str,
        validate: &impl Fn(&M, &str) -> Result<(), String>,
    ) -> Result<SaveCommit, String> {
        let report = self.reconcile(store, validate)?;
        let outcome = self
            .control
            .operations
            .get(operation)
            .ok_or_else(|| "Save operation receipt disappeared".to_owned())?;
        if outcome.status != OperationStatus::Committed {
            return Err(report.warnings.join("; "));
        }
        Ok(outcome.receipt(false, report.warnings))
    }

    fn pending_warning(&self) -> Vec<String> {
        self.control
            .pending
            .as_ref()
            .map(|_| vec!["Save committed; catalogue cleanup still needs recovery".into()])
            .unwrap_or_default()
    }

    /// Explicit deletion commits catalogue removal before removing payload bytes.
    pub fn delete(
        &mut self,
        store: &mut impl IndexedSaveStore,
        operation: &str,
        id: u64,
    ) -> Result<SaveCommit, String> {
        self.ensure_current(store)?;
        let signature = id.to_string();
        if let Some(outcome) =
            self.check_operation(operation, &OperationKind::Delete, &signature)?
        {
            if outcome.status == OperationStatus::Committed {
                return Ok(outcome.receipt(true, self.pending_warning()));
            }
        }
        if self
            .control
            .pending
            .as_ref()
            .is_some_and(|pending| pending.operation == operation)
        {
            return self.finish_write(store, operation, &|_, _| Ok(()));
        }
        self.require_no_pending()?;
        let payload = self
            .entries()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.payload.clone())
            .ok_or_else(|| format!("Cannot delete missing save {id}"))?;
        let mut next = self.control.clone();
        next.operations.insert(
            operation.into(),
            Outcome {
                kind: OperationKind::Delete,
                entry_id: id,
                fingerprint: signature,
                payload: None,
                previous_payload: Some(payload.clone()),
                status: OperationStatus::Reserved,
            },
        );
        next.pending = Some(Pending {
            operation: operation.into(),
            entry: None,
            cleanup: Some(payload),
        });
        self.commit_control(store, next)?;
        self.finish_write(store, operation, &|_, _| Ok(()))
    }
}
