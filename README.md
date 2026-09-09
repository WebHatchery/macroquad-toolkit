# Macroquad Toolkit

Shared Rust utilities for Macroquad games targeting native Windows and browser
WASM. This repository is a library, with one integration example; it has no game
executable, web host, publisher, or catalog thumbnail.

## Use in a game

In a sibling game's `Cargo.toml`:

```toml
[dependencies]
macroquad = "0.4"
macroquad-toolkit = { path = "../macroquad-toolkit" }
```

The crate uses Rust edition 2021. The RustGames parent workspace supplies build
profiles and the shared target directory. Enable optional features on the toolkit
dependency only when needed:

| Feature | Purpose | Target |
| --- | --- | --- |
| Default (empty) | UI, input, rendering helpers, data, saves, audio, settings | Native and WASM |
| `net` | Frame-polled JSON HTTP | Native and WASM |
| `analytics` | Batched Hatchery Signals telemetry; enables `net` | Native and WASM |
| `db` | SQLite helpers using SQLx and async-std | Native/server only |

For example, add `features = ["net"]` to the path dependency for a network client.
Browser hosts must supply the JavaScript plugins required by storage, gamepads
and optional networking; compiling the library does not install a web host.

## Documentation

- [Module and integration guide](MACROQUAD_TOOLKIT.md): ownership, module index,
  data loading, rendering, saves, networking and the capture harness.
- [Shared settings](docs/SETTINGS.md): settings editing, display previews, audio,
  accessible controls and camera integration.
- [Artwork](docs/ARTWORK.md): bundled assets, loading paths and delivery requirements.
- [Concept art](concept_art/README.md): non-runtime visual references.
- [Coding standards](CODE_STANDARDS.md) and [agent instructions](AGENTS.md):
  contribution, validation and commit rules.

[Public source](src/lib.rs) and its Rustdoc contain exact signatures and API
examples. [shared_settings.rs](examples/shared_settings.rs) is a complete
integration example with touch controls, rebinding and display confirmation.

## Validation

The required project publishing command is `.\publish.ps1`, run with no parameters.
**This library has no `publish.ps1`**, so that
validation path is unavailable here. Validate consuming games with their own
publishers after adopting toolkit changes; library checks do not establish
browser, graphics-driver, controller or audio-device behavior.

From this directory, additional checks scoped to this package are:

```powershell
cargo test -p macroquad-toolkit --all-features
cargo check -p macroquad-toolkit --examples --all-features
cargo check -p macroquad-toolkit --target wasm32-unknown-unknown --lib --examples --features analytics
cargo doc -p macroquad-toolkit --all-features --no-deps
```

The WASM check needs the `wasm32-unknown-unknown` Rust target installed. Do not use
`--all-features` for WASM: it enables the native database dependencies. Tests
include the toolkit's own 800-total-line source gate. Keep verification images
directly in `docs/verification/`, replacing earlier captures of the same state.

## Licensing

No project-wide license is declared in `Cargo.toml` or a root license file. The
bundled Rajdhani font has its own [OFL license](assets/fonts/OFL-Rajdhani.txt);
that license does not license the toolkit code or artwork.
