# Indexed storage verification — 2026-09-27

This is the shared persistence prerequisite for Kestrum K03. Tests ran in the
actual `D:\WebHatchery\RustGames\macroquad-toolkit` checkout against its registered
workspace and real dependency configuration. No alternative manifest or copied
project was used.

## Checks

| Check | Result |
| --- | --- |
| `cargo fmt -p macroquad-toolkit -- --check` | Pass |
| `..\rust_management\cargo.ps1 clippy -p macroquad-toolkit --locked --all-targets --all-features '--' -D warnings` | Pass |
| `..\rust_management\cargo.ps1 test -p macroquad-toolkit --locked --all-features` | 428 existing unit tests + 5 indexed persistence tests + 29 doc tests passed; 9 existing doc examples ignored |
| `..\rust_management\cargo.ps1 check -p macroquad-toolkit --locked --examples --all-features` | Pass |
| `..\rust_management\cargo.ps1 check -p macroquad-toolkit --locked --target wasm32-unknown-unknown --lib --examples --features analytics` | Pass |
| `..\rust_management\cargo.ps1 doc -p macroquad-toolkit --locked --all-features --no-deps` | Pass |
| Source-size gate | Pass in toolkit suite; largest Rust file 621 lines, new integration suite 531 lines |
| `git diff --check` | Pass |
| Toolkit `publish.ps1` | Absent, as documented for this library. Kestrum publishing validates adoption separately. |

## Behavioral evidence

The five integration tests cover 32-entry discovery after rebuilding the catalogue
from stored bytes; unique external IDs; same-name independent entries; explicit
overwrite/delete; retry identity; stale/Busy/Pending writer refusal; exhausted
counters; invalid schema and corrupt/future control records; and unreadable
storage. They exercise all 13 injected failing mutation positions and all 16
before/after-mutation restart snapshots across create, overwrite and delete.

Malformed-record cases attempt a pending payload alias of a listed save, unrelated
cleanup reference, mismatched overwrite target and zero-padded payload identity.
Every case fails without changing any bytes. Pending recovery remains discoverable
without a writer lease and never removes the previous listed save.

Native backend verification uses the separate app-data identity
`macroquad_toolkit_indexed_verification`, with `test_<process-id>` and its child
namespace. It checks OS lease contention, failed replacement preserving the old
file, namespace-prefix separation, and **eight actual catalogue entries after
dropping and reopening the native stores**. This is automated backend reopening,
not a claim of manual game UI restart testing. The harness removes only its exact
owned payload/control/probe/lock files. After the run, the isolated app-data
directory was absent.

## Browser bridge evidence

From the actual `rust_management` checkout:

- `node --test .\scripts\test-storage-bridge.cjs`: all five tests passed.
- `node .\scripts\test-storage-bridge-browser.cjs`: isolated Chromium protocol
  harness passed cross-tab Web Lock exclusion, lease release after reload/close,
  twelve payload reads after reload, quota rejection and checked read/remove
  failures. No game UI, player profile, fixture file or screenshot is involved.

The existing browser storage imports remain available. New indexed keys and locks
frame game/namespace separately to avoid legacy underscore concatenation aliases.
The consuming game must be published to deliver the updated shared bridge and
must separately verify its visible naming, retry, deletion and load flows.

## Limits

Web Locks require a supported secure browser context. Unsupported locking refuses
writes explicitly while committed catalogue discovery remains available. These
tests do not certify sudden hardware power-loss behavior or physical mobile
devices. Native/backend checks, browser protocol checks, and later game UI review
are distinct evidence.
