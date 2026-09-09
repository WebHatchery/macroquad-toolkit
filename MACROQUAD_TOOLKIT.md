# Macroquad Toolkit: modules and integration

This guide describes the API exported by [src/lib.rs](src/lib.rs). Use
[README.md](README.md) for dependencies and validation, and the linked source
modules/Rustdoc for signatures rather than copying partial game skeletons.

## Ownership and architecture

A consuming game owns its main loop, typed content schemas, simulation, state
transitions, asset paths and save migrations. Keep rendering separate from state
updates where practical: UI returns an intent, and the owning state applies it.
Use the toolkit for reusable runtime, input, rendering and platform behavior.
Game balance and authored content belong in game data; generic JSON loading
belongs in `data_loader`.

For authoritative multiplayer, the server owns validation, simulation time,
shared state and durable persistence. The client displays projections and sends
commands. Protocol types can live in a shared protocol crate. The transport here
does not provide a server, authentication policy or database schema.

## Module index

All modules below are available without optional features unless marked.

| Area | Modules and responsibilities |
| --- | --- |
| UI | [ui](src/ui.rs): bounded text, font, formatting, layout, pointer widgets, surfaces, forms, scrolling, tabs, tooltips, plaques, contrast and scaling |
| Input | [input](src/input.rs): hit targets, device capture, gestures, action bindings and remapping |
| Cameras | [camera](src/camera.rs): interactive 2D camera, pure viewport transforms and settings-aware control |
| Rendering | [sprite](src/sprite.rs): sprites, uniform atlases and variations; [render3d](src/render3d/mod.rs): billboards, cameras and picking |
| Assets | [assets](src/assets.rs): textures, fonts, ZIP packs, manifests and chroma helpers; [artwork](src/artwork.rs): optional reference pack |
| Color and procedural images | [colors](src/colors.rs), [raster](src/raster.rs), [noise](src/noise.rs), [paint](src/paint.rs) |
| Outcome presentation | [reveal](src/reveal.rs): count-ups and staged reveals; [strip](src/strip.rs): deterministic decided-stop reel animation |
| Motion and time | [math](src/math.rs): interpolation/easing; [timing](src/timing.rs): timers/timelines; [fx](src/fx.rs): shake, fades, particles, floating text, travel, typewriter and CRT effects |
| World data | [grid](src/grid.rs): grids, coordinates, fog and vision; [pathfinding](src/pathfinding.rs): A* and caching; [entities](src/entities.rs): entity storage |
| Game services | [states](src/states.rs), [events](src/events.rs), [rng](src/rng.rs), [notifications](src/notifications.rs), [achievements](src/achievements.rs) |
| Audio | [audio](src/audio.rs): playback and settings routing; [synth](src/synth.rs): sound synthesis; [score](src/score.rs): musical composition for synthesis |
| Data and saves | [data_loader](src/data_loader.rs), [persistence](src/persistence.rs); [wasm_storage](src/wasm_storage.rs) is WASM-only |
| Preferences and diagnostics | [settings](src/settings.rs), [debug](src/debug.rs), [crash](src/crash.rs), [series](src/series.rs) |
| Verification | [capture](src/capture.rs): deterministic screenshots; [source_gate](src/source_gate.rs): Rust source line-count checks |
| Optional services | [net](src/net.rs) (`net`), [analytics](src/analytics.rs) (`analytics`), [db](src/db.rs) (`db`, native only) |

`prelude` re-exports common 2D helpers, `prelude_3d` exports entities/grid/3D
helpers, and `prelude_data` exports data loading and persistence. Feature-gated
`prelude_net` and `prelude_analytics` expose the corresponding clients. Explicit
imports avoid ambiguity with Macroquad names such as `Camera2D`.

## Data loading

Use `parse_json_labeled` for JSON text with a meaningful source name, or
`load_embedded_json_labeled` for embedded data. `include_json_str!` embeds text;
`include_json!` embeds and parses it with a path label. Macro paths resolve
relative to the invoking Rust source. Games own types and content validation.

`load_json_file` asynchronously loads required runtime data on native/WASM.
The synchronous runtime loader is native-only in behavior: on WASM it cannot
read runtime files. For explicit embedded fallbacks use
`load_json_file_with_fallback` or `load_json_file_with_fallback_sync` and choose
`JsonFallbackPolicy::ReadError` (read failure only) or `ReadOrParseError` (also
malformed runtime content). The synchronous fallback path uses embedded data on
WASM. Invalid fallback JSON remains an error. Candidate-path loading via
`load_json_with_fallback_sync` rejects an existing unreadable or malformed candidate.

`DataRegistry::from_embedded_arrays` merges arrays in authored order with
caller-selected IDs. `overlay_json_directories` reads sorted JSON files from the
first readable native directory and reports invalid files; it is a no-op in
browsers. Use `into_map` for game-owned validation and simulation.

## UI, text and input

Every label, button, panel, card, tooltip and status area needs a width and height
budget. Wrap, shrink or truncate before text can overlap neighbors:

- `draw_text_centered_in_box` for button labels, titles and centered captions.
- `draw_text_block` for paragraphs; `wrap_text` and `fit_text_to_box` for layout.
- `truncate_text_to_width` for single-line labels.

`button`, `button_styled` and `button_on_release` test release over the target;
`button_on_press` tests press over the target. Release-over behavior is not a
promise that a press originated inside the target. `is_hovered`, `was_clicked`
and `was_pressed` provide low-level rectangle checks. `Pointer` and explicit
coordinate mapping support scaled UI; action maps connect visible controls,
keyboard and gamepads. See [settings integration](docs/SETTINGS.md) for routing,
rebinding and gestures. Every required game action needs a visible tap target or
an explicit direct-touch gesture; shortcuts are supplemental.

`GameSettings::ui_scale` is 75–200%, default 100%, independently of text scaling.
Apply it with `apply_display` or `ui::set_ui_scale`. Create
`VirtualUi::responsive()` each frame and reflow against its logical dimensions.
It fills the window: smaller scales expose more layout space, larger scales
require reflow or scrolling. `VirtualUi::scaled(min_width, min_height)` can reduce
the requested scale to fit minimum logical dimensions. Pair `begin()` with
`screen_to_ui()` and restore the default camera with `end_virtual_ui_frame()`.
Use a separate world camera. `draw_notifications_in_viewport` accepts the same UI
bounds. See [shared settings](docs/SETTINGS.md) for safe display previews.

## Cameras

`CameraTransform` is a pure logical-pixel transform. Supply an explicit `Rect`
viewport to `screen_to_world`, `world_to_screen` and `zoom_at`; offsets are part
of the transform. `pan_screen` follows a drag and `zoom_at` preserves the anchor
world point even at zoom limits. Invalid inputs are rejected.

`constrain` supports `CameraBoundsPolicy::TargetInside` and `KeepVisible { pixels }`.
Apply bounds after movement; constraints may move the zoom anchor. `apply_gesture`
consumes claimed `TouchGesture` frames, leaving taps for controls. `Camera2D`'s
`transform` and `set_transform` bridge to the interactive camera. Games retain
input routing, projection, selection and saved schemas.

Use `CameraController::update_2d` for shared settings/action integration; do not
also call `Camera2D::update` in that frame. The latter remains available for
callers managing their own controls.

## Assets, sprites and audio

`AssetManager` caches named textures and fonts. Texture manifests use a
`textures` array of `key`, `path` and optional `filter` entries. Loading returns
counts; handle errors and check required keys instead of assuming every texture
loaded. ZIP packs preserve normal asset paths, for example
`assets/tiles/tile_01.png`. Register a placeholder explicitly with
`set_placeholder_texture` or `set_placeholder_texture_direct`; the reference
missing-texture image is not installed automatically.

PNG/TGA decoding comes through Macroquad; this crate additionally enables JPEG.
Use PNG for alpha and pixel art. Set nearest/linear filtering deliberately.
`SpriteAtlas` uses uniform cells rather than packed-atlas metadata. See
[artwork requirements](docs/ARTWORK.md) for the reference pack and art delivery.

`SoundManager` loads sounds by caller-defined IDs, plays effects with `play_sfx`
and routes music/voice/UI/ambience with `play_group`. Use `apply_settings` with
real host focus state for master/group volume and focus muting. Audio routing,
reduced motion and autosave integration are covered in [SETTINGS.md](docs/SETTINGS.md).

## Saves and compatibility

Use `save_to_slot`/`load_from_slot` for slots and `save_json_key`/`load_json_key`
for named JSON values. Native backends use files; WASM uses localStorage through
the `sapp-jsutils` storage plugin. Native-only `save_json`/`load_json` accept file
paths. Propagate errors and keep recovery visible. `AutoSaveManager` schedules
attempts; games still own serialization and safe save points.

Versioned slot APIs and `load_json_key_with_migration` let games supply migration
logic. Configured key APIs (`save_json_key_configured`, `load_json_key_configured`,
`json_key_exists_configured`) accept an optional environment-variable name for a
complete native save path, useful for isolated restart checks. Browser storage
continues using the qualified key.

`BackupChain::with_generations("campaign", 3)` uses `campaign_backup`,
`campaign_backup_2` and `campaign_backup_3`; `new` accepts explicit names.
`KeySaveStore` adapts qualified keys, `FileSaveStore` native paths, and
`RawSaveStore` custom/in-memory backends. Validate schema/version before saving:
an invalid primary must be explicitly resolved first. Rotation preserves raw
bytes, skips invalid backups, and writes oldest-first and primary-last. Failed
writes may leave partial backup rotation; primary preservation depends on the
backend's single-key atomic-write contract. Serialize concurrent writers.
`recover` returns the decoded value, `SaveSource` and rejected candidates without
writing automatically.

The currently supported `load_json_key_with_legacy` API imports explicitly
allowlisted raw browser keys only when the qualified key is absent. Validate
both current and imported data; a corrupt/future primary blocks import. It
preserves old keys, reports failed writes and exposes `LegacySource`. Native
builds ignore browser aliases. `load_with_legacy_keys` offers the same policy for
`RawSaveStore`. These are compatibility tools for existing saves, not automatic
key discovery or deletion.

For new deterministic simulations use state-owned `SeededRng`, persisting and
restoring its state. `LegacyLcg64<INCREMENT>` is a supported compatibility stream
for games whose saves depend on that sequence. Keep game-specific seed mixing,
range mapping and random draw order in the game; switching generators changes
replays. Global RNG helpers are available for callers that do not need isolated
streams.

## Optional services

With `net`, retain the `Pending<T>` returned by `HttpClient` and call
`poll_timed(dt, timeout)` once per frame. Native requests use background threads;
WASM uses `quad-net.js`. The toolkit handles headers, JSON and transport timeouts.
Games own retry cooldowns, sessions, authentication, endpoint types and CORS.
Retain the last safe projection and expose connection failures in the game UI.

With `analytics`, create one `AnalyticsClient`, call `update(dt, is_active)` each
frame and report sparse milestones or the dedicated completion/store-click
events. It owns anonymous IDs, launch sessions, active-time heartbeats, batching
and retries. Games own collection/consent policy and when the client is created;
telemetry failures must not interrupt gameplay.

With native-only `db`, `Database::new` opens SQLite, `run_raw_migrations` executes
supplied statements sequentially, and `pool` exposes SQLx. The helper is not a
versioned migration framework or an automatic transaction around the batch.

## Screenshot capture

The [Windows wrapper](scripts/capture_ui.ps1) builds a consuming game's executable
and runs all requested scenes in one process/window. The game must read
`CaptureConfig::all_from_env(prefix)`, seed each scene, await `run_capture_once`
for it, then return after the batch. Use `capture_window_conf` at startup to apply
capture dimensions and arm Windows window hiding. No capture mode is enabled on
WASM, and hiding still requires a native graphics context.

The manifest named by `PREFIX_CAPTURE_MANIFEST` contains one `scene<TAB>path` row
per capture. Frames default to 150 at a deterministic 1/60-second timestep.
`PREFIX_CAPTURE_FRAMES`, `PREFIX_CAPTURE_MIN_FRAME_MS`, `PREFIX_WINDOW_WIDTH`,
`PREFIX_WINDOW_HEIGHT`, `PREFIX_CAPTURE_FULLSCREEN` and `PREFIX_HEADLESS` control
the run. There is no single-path capture environment API.

From a consuming game's directory, when a capture is requested:

```powershell
& ..\macroquad-toolkit\scripts\capture_ui.ps1 -Scenes gameplay,map
& ..\macroquad-toolkit\scripts\capture_ui.ps1 -Scenes gameplay -SkipBuild
```

Use the game's actual scene names. The script derives the package, target path
and uppercase underscore-separated prefix from Cargo metadata. Override `-Prefix`
or `-ExeName` when necessary. `-GameDir` selects a game; this library itself has
no executable for the wrapper. Images replace `docs/verification/ui_<scene>.png`.
Use `-Release` for optimized builds; `-Visible` explicitly shows the game window.

`-ExecutablePath <path> -SkipBuild` checks an existing packaged binary.
`-Fullscreen` requests the monitor-sized framebuffer. `-MinFrameMilliseconds`
sets a minimum wall-clock frame duration without changing simulation time.
`-ProcessReportPath <path>` writes scene/surface/timing metadata, sampled working
set distribution and OS peak memory. `-SampleWindowsGpuCounters` adds optional
GPU memory and 3D utilization samples; the report distinguishes sampled,
unavailable and not-requested counters and imposes no GPU threshold. These
captures supplement the consuming game's required publisher validation.

## Source gate

`source_gate::assert_source_files_within_limit(env!("CARGO_MANIFEST_DIR"), &[])`
checks all `.rs` files below the project directory, excluding `.git` and build
`target` directories. It counts every physical line, including test sources,
examples, build scripts, comments and whitespace. The limit is 800. Missing
project directories fail. Add the call to a test; the toolkit tests itself.

The API retains an exception-list parameter for compatibility, but repository
policy requires an empty list and splitting oversized files by responsibility.
Unit tests belong in separate child files as specified in
[CODE_STANDARDS.md](CODE_STANDARDS.md#113-test-placement).
