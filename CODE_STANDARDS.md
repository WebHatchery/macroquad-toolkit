# Coding standards for Macroquad Toolkit

These standards apply to this library and its examples. Consuming games also
follow the workspace instructions in [AGENTS.md](AGENTS.md). This document is
the repository's contribution reference; the actual module layout is indexed in
[MACROQUAD_TOOLKIT.md](MACROQUAD_TOOLKIT.md).

## 1. Core philosophy

Prefer readable code, explicit ownership and small modules. Follow established
naming and structure, and avoid broad refactors during focused changes. Extract
reusable runtime/platform behavior into the toolkit; keep game rules and authored
content in consuming games. Remove unused private code and fields. Public APIs
can be used externally even when this crate has no internal caller; inspect
consumers before removing or changing a public contract.

## 2. Project structure

### 2.1 Responsibilities

`src/lib.rs` declares public modules and preludes. Each module owns one reusable
responsibility, with child modules for implementation details and tests.
`examples/shared_settings.rs` owns its example's main loop. The library does not
have a `main.rs` or central game state. Stateful types own their mutations; keep
pure calculations and rendering separate where practical.

### 2.2 File size

Target 200–400 lines and reconsider the structure as a file approaches 600.
Every `.rs` file must stay at or below **800 total physical lines**, including
comments, whitespace, attributes, tests, examples, generated source, build
scripts and benches. There are no exceptions. Extract a cohesive responsibility;
never compress formatting or strip useful spacing to meet the limit.

### 2.3 Module source filenames

Use `foo.rs` and child sources such as `foo/bar.rs`. Do not create `mod.rs` files.
Migrate an existing `foo/mod.rs` when restructuring that module; never keep both
module-root forms. Existing module filenames are not a template for new work.

## 3. Naming

Use PascalCase for types, snake_case for functions/variables/modules, and
SCREAMING_SNAKE_CASE for constants. Prefer names that state purpose and booleans
that read as facts (`is_active`, `can_interact`). Match the surrounding public API
rather than introducing a mandatory service/engine suffix scheme.

## 4. Functions and methods

Keep functions focused and easy to scan, normally 20–50 lines. Split long
functions by responsibility. Prefer a configuration/context struct when many
related parameters would obscure intent; preserve established drawing signatures
when consistency matters. Use `Option` for absence, `Result` for failure, and
named result types when a tuple would hide meaning.

## 5. Data and state

Keep deterministic simulation state and randomness explicit. Prefer state-owned
RNG for replayable work; isolate global/random runtime helpers from pure logic.
Games own typed schemas and semantic validation of IDs, references and rules.
Route game-data parsing and embedded/runtime loading through `data_loader` and
its exported include macros. Do not add generic loader wrappers in games.

Game balance and authored content belong in JSON under the game's `assets/`.
Toolkit algorithm constants, layout defaults and type invariants can live with
the code they define. Choose embedded or runtime data deliberately; both are
supported. Keep save schema migration and server authority with the consumer.

## 6. Errors

Use clear source-labeled errors for asset loading, saves and platform services.
Propagate fallible I/O and let the game show recovery controls. Logging alone is
insufficient when a player needs to know an operation failed. Reserve panics for
unrecoverable invariants. Do not silently discard a failed save or required asset.

## 7. UI and input

UI reads game state and returns intents; the game applies gameplay changes.
Widgets can own interaction state without taking ownership of the simulation.
Use shared UI, pointer and action helpers. Keep every UI text draw within a width
and height budget, using bounded text helpers from the module guide.

Ship touch-first: every required start, tutorial, core and recovery action must
have a visible tap/click target or an explicit direct-touch gesture. Use at least
44x44 logical pixels for required touch controls. Keyboard shortcuts supplement
these controls; player-facing shortcut text must also name the visible control.
Tutorials state the exact next target or gesture. Keep input coordinates aligned
with the drawing camera, reflow/scroll scaled layouts, and prevent menu events
from reaching gameplay. Do not rely on color alone to convey state.

## 8. Platforms and validation

This library targets native and WASM; optional `db` is for native/server code.
Avoid new dependencies unless they remove real complexity or follow an existing
pattern. Keep platform differences behind toolkit APIs. Browser consumers supply
the host and required JavaScript bridges, runtime assets and game publisher.
Published catalog games also own their root `catalog_thumbnail.png`; this
library does not need one.

After meaningful changes, the prescribed validation is the affected project's
`.\publish.ps1` with no parameters. It is absent in this repository: report that
fact. Additional package-scoped checks are in [README.md](README.md#validation).
Do not substitute a local game/server run for publishing validation. Validate
adopting games with their own publisher when changing their integration.

## 9. Documentation

Explain intent, invariants, platform limits and failure behavior. Public modules
need a short purpose statement and accurate API examples. Keep longer workflow
guidance in the linked documents rather than duplicating it across README and
source. Document currently supported behavior; do not mix speculative roadmaps
or completed implementation diaries into usage instructions.

## 10. Formatting and tooling

Use `cargo fmt -p macroquad-toolkit` for Rust edits and keep Clippy diagnostics
visible. Explain intentional lint allowances. Avoid confusing variable shadowing.
Remove unused private fields rather than hiding them with underscore prefixes;
unused parameters required by a trait/API can use an underscore prefix.

## 11. Tests

### 11.1 Coverage

Test calculations, state transitions, serialization, fallback/recovery and
platform-independent input/layout behavior. Rendering helpers with pure or
CPU-side paths can have meaningful tests. GPU, browser and hardware behavior
needs corresponding integration/visual verification; a compile check is not
proof of interaction behavior.

### 11.2 Style

Tests should express observable rules and failure cases, with small deterministic
fixtures. Avoid tests that simply mirror implementation details.

### 11.3 Test placement

Unit tests always live in separate child files, never inline module bodies:

```rust
// In src/foo.rs; implementation goes in src/foo/tests.rs.
#[cfg(test)]
mod tests;
```

This preserves `use super::*` and private-item access. Split larger suites into
focused child files; every test source also has the 800-total-line limit.
Crate-root `tests/` is for actual integration tests of the public API, not a
place to move private unit tests. The toolkit's source gate runs in its own test
suite with an empty exception list.

## 12. Verification artifacts

Store screenshots directly in `docs/verification/`, with no subfolders. Replace
an earlier image of the same screen/state rather than keeping duplicate captures.
Concept art is separately documented under `concept_art/`; it is not a screenshot
or evidence of runtime behavior.

## 13. Completing changes

Review the diff and working tree, perform required validation, and follow the
commit/staging rules in [AGENTS.md](AGENTS.md). Report validation failures or
missing infrastructure honestly, alongside the final commit hash. Keep this
library's documentation and source contracts consistent in the same change.
