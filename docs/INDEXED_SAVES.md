# Indexed save catalogues

`persistence::IndexedCatalogue<M>` discovers any number of named saves on native
and browser builds. Display names belong to game metadata; storage keys use
monotonic internal identities. The catalogue never rotates or deletes saves
implicitly. Actual filesystem/localStorage capacity still applies.

## Ownership and startup

The game owns one `IndexedKeyStore` and one `IndexedCatalogue<Metadata>` using the
same lowercase ASCII namespace. Names may contain digits, underscores and hyphens.
Constructors do no I/O. Call `store.poll_writer()` during startup frames:

- `Ready`: the store holds an exclusive writer lease for its lifetime.
- `Pending`: browser Web Locks acquisition has not completed; poll next frame.
- `Busy`: another window owns this namespace; offer a visible Retry control that
  calls `retry_writer()` after the other window closes.
- `Err`: explain the storage/locking problem and preserve the live session.

Native locking uses `std::fs::File::try_lock`, available since Rust 1.89. Browser
locking uses an exclusive Web Lock, so HTTPS or trusted localhost and Web Locks
support are required. The browser callback retains an unresolved promise until
the store is dropped. A closed/crashed document releases its lease. The toolkit
does not substitute a racy localStorage lock when Web Locks are unavailable.

`refresh(&mut store, validate)` reads the current catalogue and reconciles pending
operations when the writer is ready. Without a lease, it still exposes committed
entries and reports pending recovery as a warning. A future/corrupt control
record or an unreadable store is an error, never an empty new catalogue.

The backend frames game and namespace separately, preventing underscored names
from aliasing. Native files are `indexed.<namespace>.<logical-key>.json` under the
game's normal app-data directory. Browser keys are
`mq-indexed:<game>:<namespace>:<logical-key>`. These are independent of legacy slot
keys; a game must explicitly inspect/import its own legacy schema.

## Public operations

Metadata requires `Clone + Serialize + DeserializeOwned`. The game supplies
`validate: Fn(&Metadata, &str) -> Result<(), String>`; validate full payload
semantics and metadata/schema consistency, not just JSON syntax.

| Operation | Behavior |
| --- | --- |
| `entries()` | Stable IDs and game metadata for committed saves |
| `reserve_identity(&mut store)` | Durable monotonic external ID, independent of simulation RNG |
| `load(&store, id)` | Exact raw payload; game validates a candidate before replacing play |
| `write(&mut store, token, None, metadata, raw, validate)` | New distinct entry |
| `write(&mut store, token, Some(id), metadata, raw, validate)` | Explicit overwrite using a fresh payload key |
| `delete(&mut store, token, id)` | Explicit catalogue removal, then payload cleanup |
| `refresh(&mut store, validate)` | Discovery, interrupted-operation recovery and warnings |

A `SaveCommit` returns `entry_id`, `replayed`, and cleanup `warnings`. Cleanup
warnings mean publication succeeded; they must not cause the game to repeat a
round or award an effect again. `RecoveryReport` contains recovered/abandoned
operation tokens, `committed_writes` entry IDs, an optional pending token, and
warnings. `committed_writes` includes previously published writes whose journal
cleanup is being retried, allowing a game to reconcile its Continue selection.

Reserve an external identity for each new campaign or save request. Persist or
retain a prepared save's unique operation token and immutable raw payload for
retry. Do not use only campaign+round for new checkpoints: returning to an older
save can reach that round again with different orders. A lost reservation response
can leave a harmless identity gap. IDs are never recycled.

Reusing a token with another operation/payload is rejected. A committed write
retry keeps its original metadata; it fails if that entry was later overwritten
or deleted. A failed, abandoned attempt can retry its original payload with new
metadata and the originally reserved entry ID. Deletion retry remains idempotent.

Use `RawSaveStore::read/write` on additional namespace-prefixed logical keys for
game-owned preferences such as Continue selection. These writes require the same
lease but are separate from catalogue publication; handle their failures honestly.
Keep preferences and campaign state logically separate.

## Publication and recovery

One atomic control record owns a format version, revision, monotonic counter,
committed references, compact operation receipts, and at most one pending record.
Payload bytes never enter receipts or the journal.

1. Validate the new payload. Atomically reserve its fresh identity and journal.
2. Write exact bytes to the fresh payload key.
3. Read and validate the payload; atomically publish its catalogue reference.
4. Remove obsolete payloads after they are no longer listed.
5. Clear the journal.

On restart, a valid pending payload is published once. Missing, corrupt, or
semantically rejected payloads are abandoned with a visible warning, leaving
previous entries unchanged. An unreadable pending payload remains pending for
retry. Cleanup failure can leave an internal orphan; it cannot delete a listed
good save. Malformed references, aliased payloads, mismatched deletion targets,
and unsupported control formats are rejected before mutation.

Every mutation checks the currently persisted control bytes against the caller's
snapshot while holding the writer lease. A stale snapshot must be refreshed.
`IndexedSaveStore` is the injectable seam; implement checked raw reads/writes,
idempotent checked removal, and exclusive writer ownership. Tests can use memory
instead of the player's real store.

Native writes flush staging bytes and replace with `std::fs::rename`, including
Windows; the destination is never deleted first. The protocol is tested at every
process-interruption boundary. This is not hardware power-loss certification or
a backup/export facility. LocalStorage removal by the browser/user remains
outside the catalogue's retention guarantee.

The list and compact operation receipt map have no artificial count cap. Each
receipt retains identifiers, request fingerprint and status, not campaign bytes.
Games should measure catalogue/save sizes and communicate real storage limits.

## Verification

`tests/indexed_persistence.rs` contains five behavioral cases, with in-memory
failure injection and a native backend reopen/lease check. The native fixture uses
the separate `macroquad_toolkit_indexed_verification` app-data identity and
`test_<process-id>` namespaces. Tests remove only their exact generated payloads,
control, probe, and lock files; no real game namespace is accessed.

The shared bridge has durable protocol tests in `rust_management/scripts/`:

```powershell
node --test .\scripts\test-storage-bridge.cjs
node .\scripts\test-storage-bridge-browser.cjs
```

The browser test uses isolated Chromium storage and real Web Locks across blank
documents. It does not operate game UI or user browser profiles. Consuming games
must still publish and exercise their own visible save/retry/load flows.
