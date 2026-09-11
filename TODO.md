# Macroquad Toolkit TODO

This backlog records follow-up work identified by reviewing the repository
against [AGENTS.md](AGENTS.md) and [CODE_STANDARDS.md](CODE_STANDARDS.md).
Keep entries concrete, link new behavior to the affected API, and remove an
entry when its verification is complete.

## Compliance and structure

- [ ] Move the inline test modules in `src/crash.rs`, `src/noise.rs`, and
  `src/persistence/slots.rs` into `tests.rs` child files. Leave only the
  `#[cfg(test)] mod tests;` declarations in implementation files, as required
  by `CODE_STANDARDS.md` §11.3.
- [ ] Migrate `src/render3d/mod.rs` to the named module layout
  (`src/render3d.rs` with `src/render3d/{billboard,camera,picking}.rs`), then
  update the module-guide link and any consumer imports. Do this when the
  module is next restructured, per `CODE_STANDARDS.md` §2.3.
- [ ] Split the source-size hotspots by responsibility before adding more
  behavior. The current review found `src/ui/widgets.rs` at 621 lines,
  `src/ui/scroll_tabs.rs` at 575, and `src/ui/font/text.rs` at 556; the next
  group is `src/assets.rs` (495), `src/ui/surfaces.rs` (483),
  `src/ui/pointer.rs` (480), and `src/persistence/slots.rs` (473). Keep every
  resulting `.rs` file below 800 lines and move toward the 200–400 line target.
- [ ] Add a structural source check for inline `#[cfg(test)] mod ... {}` bodies,
  newly added `mod.rs` roots, and files approaching the 600-line review
  threshold so these rules do not depend on a manual audit.
- [ ] Explain every intentional `#[allow(...)]` at
  `src/pathfinding/search.rs`, `src/ui/font/text.rs`, `src/ui/widgets.rs`,
  `src/persistence/slots.rs`, and the test-only allowance in
  `src/ui/bounds/tests.rs`, or remove the allowance by changing the design.

## UI, input, and accessibility

- [ ] Make the generic controls safe inside scaled and letterboxed UI frames.
  `src/ui/forms.rs` and the legacy button/tab wrappers still read the raw
  window mouse, while only some APIs accept an explicit logical pointer.
  Provide consistent `Pointer`/input-aware variants or pure `*_with` paths,
  document the coordinate contract, and cover device-pixel-ratio, touch,
  drag, and virtual-UI cases.
- [ ] Connect the touch-target and text-bound audits to production widgets.
  `note_target`, `note_neighbour`, and `note_control` are currently exercised
  by tests and callers but are not registered by the shipped widgets. Make
  required controls report their drawn and hit areas, and make the audit
  usable on real integration screens without making it part of the normal
  frame cost.
- [ ] Finish the bounded-text path for UI status surfaces. Tooltip wrapping
  currently has no viewport-height budget, and direct draws remain in
  `src/ui/widgets.rs`, `src/notifications.rs`, and `src/fx/floating_text.rs`.
  Route UI text through fit/trim helpers with collision and contrast reporting;
  document an explicit exemption for world-anchored floating text if it is
  intentionally outside the UI contract.
- [ ] Add non-color state affordances and meaningful accessibility checks to
  shared controls, including selected, disabled, hover, and pressed states.
  Preserve the visible labels and the 44x44 logical-pixel minimum while making
  the result understandable without color or a mouse hover.
- [ ] Separate rendering from mutation for the form widgets and settings
  panel. Add intent-returning paths that let an owning game apply changes to
  its draft or simulation state, while keeping compatibility wrappers for
  existing callers.
- [ ] Add controller focus navigation to the settings and rebinding panels, or
  document a stable game-owned adapter for controller-only operation. The
  current settings guide explicitly leaves this behavior unimplemented.

## Errors, persistence, and platform behavior

- [ ] Give persistence failures source context. Errors such as `File not found`,
  `Read error`, and `Deserialization error` in `src/persistence/files.rs` and
  `src/persistence/keys.rs` should identify the path or storage key and retain
  the underlying cause. Add failure-case tests for missing, unreadable,
  malformed, and failed-replacement saves.
- [ ] Harden `db::Database::new`: propagate directory/file-creation failures,
  avoid library-level `println!` diagnostics, and handle SQLite file URLs,
  `:memory:` databases, and query parameters without treating the URL as a
  raw filesystem path. Add native `db` feature tests for setup and migration
  failures.
- [ ] Make asset-manifest loading return source-labeled parse and per-entry
  errors while preserving the current partial-load counts. Required assets
  should remain distinguishable from optional entries, with tests for missing,
  malformed, and packed assets.
- [ ] Add a native/WASM integration matrix for storage, gamepads, networking,
  audio focus, DPI scaling, and touch gestures. The current package checks
  compile these targets but cannot prove browser, graphics-driver,
  controller, or audio-device behavior.

## API documentation and validation

- [ ] Add accurate, target-aware, compile-checked Rustdoc examples for the
  public modules that currently have only a purpose line: achievements,
  analytics, artwork, audio, colors, `db`, debug, `fx`, noise, pathfinding,
  raster, reveal, RNG, score, series, states, UI, and WASM storage. Keep
  platform- or feature-dependent examples explicitly annotated.
- [ ] Add `cargo fmt --check` and `cargo clippy --all-targets --all-features`
  to the documented package validation and to a repeatable local/CI runner.
  Keep intentional lint allowances visible and keep the WASM feature set
  separate from native-only `db`.
- [ ] Expand the shared-settings example or add a small integration fixture
  covering a menu, overlay, notification, responsive layout, scroll region,
  and recovery action. Capture representative states through the existing
  harness and keep verification images directly in `docs/verification/`.
- [ ] Add focused performance measurements for text layout, panel drawing,
  cached fonts, and widget hit-testing at common WebGL sizes once the layout
  and input contracts are stable.
- [ ] Choose and add a root project license, or document the distribution terms
  for the toolkit and artwork. The current OFL file covers only the bundled
  Rajdhani font.
