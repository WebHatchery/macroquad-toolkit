//! Recovery promotes only validated fresh payloads; old listed saves stay intact.

use super::*;

impl<M: Clone + Serialize + DeserializeOwned> IndexedCatalogue<M> {
    /// Refreshes discovery even without a writer lease. Recovery waits for Ready.
    /// Unsupported/corrupt control records are errors and are never replaced.
    pub fn refresh(
        &mut self,
        store: &mut impl IndexedSaveStore,
        validate: impl Fn(&M, &str) -> Result<(), String>,
    ) -> Result<RecoveryReport, String> {
        self.check_namespace(store)?;
        let raw = store.read(&self.control_key())?;
        let control = match &raw {
            Some(raw) => serde_json::from_str::<Control<M>>(raw)
                .map_err(|error| format!("Save catalogue decoding: {error}"))?,
            None => Control::default(),
        };
        control.validate(&self.namespace)?;
        self.control = control;
        self.observed = raw;
        if let Some(pending) = &self.control.pending {
            if let Err(error) = require_writer(store) {
                return Ok(RecoveryReport {
                    pending: Some(pending.operation.clone()),
                    warnings: vec![format!("Pending save recovery: {error}")],
                    ..RecoveryReport::default()
                });
            }
        }
        self.reconcile(store, &validate)
    }

    pub(super) fn reconcile(
        &mut self,
        store: &mut impl IndexedSaveStore,
        validate: &impl Fn(&M, &str) -> Result<(), String>,
    ) -> Result<RecoveryReport, String> {
        let Some(pending) = self.control.pending.clone() else {
            return Ok(RecoveryReport::default());
        };
        self.ensure_current(store)?;
        let outcome = self.control.operations[&pending.operation].clone();
        let mut report = RecoveryReport::default();
        if outcome.status != OperationStatus::Committed {
            if let Some(entry) = &pending.entry {
                // A read failure preserves the pending operation for a later retry.
                let raw = store.read(&entry.payload)?;
                let valid = raw
                    .as_deref()
                    .ok_or_else(|| "Payload write did not complete".into())
                    .and_then(|raw| {
                        if fingerprint(raw) != outcome.fingerprint {
                            return Err("Pending save payload checksum does not match".into());
                        }
                        validate(&entry.metadata, raw)
                    });
                if let Err(error) = valid {
                    return self.abandon(store, pending, error);
                }
            }
            let mut next = self.control.clone();
            next.entries.retain(|entry| entry.id != outcome.entry_id);
            if let Some(entry) = &pending.entry {
                next.entries.push(entry.clone());
                next.entries.sort_by_key(|entry| entry.id);
            }
            next.operations
                .get_mut(&pending.operation)
                .expect("validated operation")
                .status = OperationStatus::Committed;
            self.commit_control(store, next)?;
            report.recovered.push(pending.operation.clone());
        }
        if pending.entry.is_some() {
            report.committed_writes.push(outcome.entry_id);
        }
        // Never delete a payload that a committed catalogue still references.
        if let Some(cleanup) = &pending.cleanup {
            if self.entries().iter().any(|entry| &entry.payload == cleanup) {
                return Err("Save cleanup references a listed payload; recovery stopped".into());
            }
            if let Err(error) = store.remove(cleanup) {
                report.pending = Some(pending.operation);
                report.warnings.push(format!(
                    "Save committed; old payload cleanup failed: {error}"
                ));
                return Ok(report);
            }
        }
        let mut next = self.control.clone();
        next.pending = None;
        if let Err(error) = self.commit_control(store, next) {
            report.pending = Some(pending.operation);
            report
                .warnings
                .push(format!("Save committed; journal cleanup failed: {error}"));
        }
        Ok(report)
    }

    fn abandon(
        &mut self,
        store: &mut impl IndexedSaveStore,
        pending: Pending<M>,
        error: String,
    ) -> Result<RecoveryReport, String> {
        if pending.entry.as_ref().is_some_and(|pending_entry| {
            self.entries()
                .iter()
                .any(|entry| entry.payload == pending_entry.payload)
        }) {
            return Err(
                "Rejected pending save references a listed payload; recovery stopped".into(),
            );
        }
        let mut next = self.control.clone();
        next.pending = None;
        next.operations
            .get_mut(&pending.operation)
            .expect("validated operation")
            .status = OperationStatus::Abandoned;
        self.commit_control(store, next)?;
        let mut report = RecoveryReport {
            abandoned: vec![pending.operation],
            warnings: vec![format!("Incomplete save was not published: {error}")],
            ..RecoveryReport::default()
        };
        if let Some(entry) = pending.entry {
            if let Err(error) = store.remove(&entry.payload) {
                report
                    .warnings
                    .push(format!("Unlisted payload cleanup failed: {error}"));
            }
        }
        Ok(report)
    }
}
