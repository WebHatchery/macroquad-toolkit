# Macroquad Toolkit

A collection of common utilities for Macroquad game development, extracted from multiple games to reduce duplication and provide consistent patterns.

> **`MACROQUAD_TOOLKIT.md` is the fuller module reference** and is kept in sync
> across every game in the catalog from `rust_management/docs/`. It covers the
> modules this file does not — math, timing, fx, form widgets, scroll and tabs,
> settings, achievements, the debug overlay, raster, and the capture harness.
> This README keeps the sections that live only here: the text layout rule, and
> persistence, audio, states, the optional `db` feature, and the optional `net`
> feature for authoritative client/server games.

## Features

- **Input utilities**: Mouse hovering, clicking, rectangle collision detection
- **UI rendering**: Buttons, colored buttons, windows/modals, panels, progress bars
- **Asset management**: Texture loading and caching
- **Audio management**: Sound effect and music handling with volume control
- **Persistence**: Easy JSON save/load system
- **State management**: Game state trait and transition system
- **Camera2D**: Pan and zoom for 2D games
- **Event bus**: Generic event system for decoupled game logic
- **Color palettes**: Consistent dark theme colors
- **Sprite system**: Builder pattern for texture rendering with transformations
- **Bounded text layout**: Helpers for measuring, wrapping, fitting, truncating, and drawing text inside UI boxes
- **Optional networking**: Frame-polled JSON HTTP for native and WASM clients
- **Optional game analytics**: Anonymous IDs, active-time heartbeats, milestones,
  batching, and retry delivery to Hatchery Signals (enables `net`)

## Usage

Runtime JSON overrides with historical fallback behavior can use
`data_loader::load_json_file_with_fallback` (async native/WASM) or
`load_json_file_with_fallback_sync` (native runtime, WASM embedded-only). Select `JsonFallbackPolicy::ReadError`
to fall back only when reading fails, or `ReadOrParseError` to also tolerate
malformed runtime content. Both retain source-labeled errors and reject an
invalid fallback. Required files use `load_json_file[_sync]`; the existing
candidate-path fallback loader still rejects unreadable existing files.

Add to your `Cargo.toml`:

```toml
[dependencies]
macroquad-toolkit = { path = "../macroquad-toolkit" }
```

Client/server games should opt into the transport feature:

```toml
macroquad-toolkit = { path = "../macroquad-toolkit", features = ["net"] }
```

For shared WebHatchery gameplay analytics:

```toml
macroquad-toolkit = { path = "../macroquad-toolkit", features = ["analytics"] }
```

### Quick Start

```rust
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;

#[macroquad::main("My Game")]
async fn main() {
    let mut assets = AssetManager::new();
    assets.load_texture("player", "assets/player.png").await.ok();

    loop {
        clear_background(dark::BACKGROUND);

        // Draw a button
        if button(10.0, 10.0, 100.0, 40.0, "Click Me") {
            println!("Button clicked!");
        }

        next_frame().await;
    }
}
```

## Modules

### Input (`input` module)

```rust
use macroquad_toolkit::input::*;

// Check if mouse is over a rectangle
if is_hovered(x, y, w, h) {
    // ...
}

// Check if rectangle was clicked (released)
if was_clicked(x, y, w, h) {
    // ...
}

// Check if rectangle was pressed (down)
if was_pressed(x, y, w, h) {
    // ...
}

// Capture input state
let input = InputState::capture();
if input.left_click {
    // ...
}

// Map rectangles to semantic actions
let action = hit_test(
    [
        HitTarget::new(Rect::new(10.0, 10.0, 80.0, 24.0), "inventory"),
        HitTarget::new(Rect::new(96.0, 10.0, 80.0, 24.0), "map"),
    ],
    input.mouse_pos,
);
```

### UI (`ui` module)

```rust
use macroquad_toolkit::ui::*;

// Simple button (triggers on release)
if button(x, y, w, h, "Click") {
    // Button was clicked
}

// Button with custom style
let style = ButtonStyle::default_dark();
if button_styled(x, y, w, h, "Custom", &style) {
    // ...
}

// Button that triggers on press (instead of release)
if button_on_press(x, y, w, h, "Press", &style) {
    // Triggers immediately when mouse down
}

// Panel with title
panel(x, y, w, h, Some("Title"));

// Window (Modal-like) with close button
if window(x, y, w, h, Some("Title"), true) {
    // Window close button clicked
}

// Progress bar
progress_bar(x, y, w, h, current, max, dark::POSITIVE);

// Colored Button
if colored_button(x, y, w, h, "Action", RED) {
   // ...
}

// Wrap and fit text inside a box
let lines = wrap_text("Long paragraph text", 280.0, 20.0);
let layout = fit_text_to_box("Long paragraph text", 280.0, 120.0, 20.0, 6.0, 12.0);
draw_text_block("Long paragraph text", x, y, 280.0, 120.0, 20.0, 6.0, dark::TEXT);
draw_text_centered_in_box("Title", x, y, 280.0, 32.0, 20.0, dark::TEXT);
```

### Text Layout Rule

All UI text should be treated as **box-bounded**.

Rule:
- every text draw in a panel, button, modal, card, header, label, tooltip, or status area must have a width and height budget
- never assume a string will fit because it looked short in one build
- if text is too large for the box, the UI must wrap, shrink, or truncate before it can overlap another element

Preferred toolkit usage:
- `draw_text_centered_in_box()` for button labels, titles, and centered captions
- `draw_text_block()` for paragraph or panel body text
- `wrap_text()` and `fit_text_to_box()` when layout must be calculated before drawing
- `truncate_text_to_width()` for single-line labels that must stay on one line

This rule exists to keep UIs visually stable across:
- longer content
- localization
- dynamic values
- different resolutions
- future content additions

### Assets (`assets` module)

The repository also includes a small reference art pack for examples and
smoke-test scenes: an archivist portrait, a 4x2 drone sprite atlas, a
standalone billboard frame, a semantic icon sheet, and a missing-texture
fallback. Load it through the normal texture-manifest path:

```rust,no_run
use macroquad_toolkit::assets::AssetManager;
let mut assets = AssetManager::new();
let loaded = macroquad_toolkit::artwork::load_toolkit_artwork(&mut assets).await?;
assert_eq!(loaded, 5);
```

The manifest is `assets/artwork_manifest.json`; keys and filtering stay in one
place so native and WASM builds resolve the same files.

The same pack also ships 50 individual 48px semantic glyphs. Load those when a
screen needs named icons rather than atlas coordinates:

```rust,no_run
let loaded = macroquad_toolkit::artwork::load_toolkit_icons(&mut assets).await;
assert_eq!(loaded, 50);
```

```rust
use macroquad_toolkit::assets::AssetManager;

let mut assets = AssetManager::new();

// Load single texture
assets.load_texture("player", "assets/player.png").await.ok();

// Load a ZIP asset pack, then keep using normal asset paths.
// The ZIP entries should be named like assets/tiles/tile_01.png.
assets.load_asset_pack("assets/tiles.zip").await.ok();
assets.load_texture("tile_01", "assets/tiles/tile_01.png").await.ok();

// Get texture
if let Some(tex) = assets.get_texture("player") {
    draw_texture(tex, x, y, WHITE);
}

// Data-driven loading (from JSON config)
// JSON: [{"key": "hero", "path": "assets/hero.png"}]
use macroquad_toolkit::assets::TextureConfig;
if let Ok(configs) = TextureConfig::load_from_file("assets/textures.json").await {
    for config in configs {
        assets.load_texture(&config.key, &config.path).await.ok();
    }
}
```

### Camera (`camera` module)

```rust
use macroquad_toolkit::camera::Camera2D;

let mut camera = Camera2D::new(vec2(0.0, 0.0), 1.0);

// In game loop
camera.update(get_frame_time(), false);

// Convert coordinates
let world_pos = camera.screen_to_world(mouse_position().into());
let screen_pos = camera.world_to_screen(world_pos);
```

### Viewport camera transforms (`camera` module)

`CameraTransform::new(target, zoom)` creates a pure logical-pixel transform.
Pass an explicit `Rect` viewport to `screen_to_world`, `world_to_screen`, and
`zoom_at`; no Macroquad window or input polling is needed. Coordinates include
the viewport's panel offset. `pan_screen` follows a drag, while `zoom_at` keeps
the anchor's world point fixed even at zoom limits. Invalid inputs are rejected.

`constrain` accepts `CameraBoundsPolicy::TargetInside` or `KeepVisible { pixels }`.
Apply constraints after movement; bounds may intentionally move the zoom anchor.
`apply_gesture` consumes claimed frames from the existing `TouchGesture`
recognizer, leaving taps to game controls. Games retain routing, projection,
selection and saved schemas. `Camera2D::transform` and `set_transform` bridge
the pure transform to the existing interactive camera; set explicit policies on
the transform before applying it.

### Events (`events` module)

```rust
use macroquad_toolkit::events::EventBus;

enum GameEvent {
    PlayerDied,
    EnemySpawned,
}

let mut events = EventBus::new();
events.push(GameEvent::PlayerDied);

// Process events
for event in events.drain() {
    match event {
        GameEvent::PlayerDied => { /* ... */ }
        GameEvent::EnemySpawned => { /* ... */ }
    }
}
```

### Colors (`colors` module)

```rust
use macroquad_toolkit::colors::dark;

clear_background(dark::BACKGROUND);
draw_rectangle(x, y, w, h, dark::PANEL);
draw_text("Hello", x, y, 20.0, dark::TEXT);
```

Available colors:
- `BACKGROUND`, `PANEL`, `PANEL_HEADER`
- `TEXT`, `TEXT_BRIGHT`, `TEXT_DIM`
- `ACCENT`, `POSITIVE`, `WARNING`, `NEGATIVE`
- `HOVERED`

Rarity colors (for RPG items):
```rust
use macroquad_toolkit::colors::rarity;

draw_rectangle(x, y, w, h, rarity::COMMON);     // Gray
draw_rectangle(x, y, w, h, rarity::UNCOMMON);   // Green
draw_rectangle(x, y, w, h, rarity::RARE);       // Blue
draw_rectangle(x, y, w, h, rarity::EPIC);       // Purple
draw_rectangle(x, y, w, h, rarity::LEGENDARY);  // Orange
```

### Additional UI Components

```rust
use macroquad_toolkit::ui::*;

// Section panel with title header
section_panel(x, y, w, h, "Section Title");

// Clickable card (returns true if clicked)
if card(x, y, w, h, is_selected) {
    // Card was clicked
}

// Full-screen overlay for modals
full_screen_overlay(0.7); // 70% opacity

// String helpers
let title = capitalize("warrior");           // "Warrior"
let name = display_name("health_potion");    // "Health Potion"

// Grid Layout Helper
let grid = GridLayout::new(x, y, width, padding, cols, card_height);
let (bx, by, bw, bh) = grid.get_item_rect(index, scroll_y);

// Scroll Helper
// Returns new scroll value clamped to content bounds
let new_scroll = handle_scroll(current_scroll, content_height, view_height, 30.0);

```

### RNG (`rng` module)

```rust
use macroquad_toolkit::rng::*;

// Seeded random number generator
let mut rng = GameRng::new(12345);
let value = rng.gen_range(1..100);
let choice = rng.choose(&["a", "b", "c"]);
```

### Legacy seeded streams (`rng` module)

`LegacyLcg64<INCREMENT>` shares the historical multiply/add/upper-32-bit
generator used by Final Landing (`INCREMENT = 1`) and Hatchspire
(`INCREMENT = 1_442_695_040_888_963_407`). Both game adapters now use it while
retaining their original range and chance behavior. Fixed pre-migration vectors
cover zero, ordinary and maximum seeds. `new(0)` retains the old seed-to-one
rule; `from_state(0)` restores zero exactly. The increment is part of the saved
algorithm contract and must not change between save and restore. Use
`SeededRng` for new streams; this API exists for compatibility.

### Sprite (`sprite` module)

```rust
use macroquad_toolkit::sprite::Sprite;

let sprite = Sprite::new()
    .with_texture(texture)
    .at(100.0, 100.0)
    .scaled(2.0, 2.0)
    .rotated(0.5)
    .colored(RED);

sprite.draw();
```

### Persistence (`persistence` module)

```rust
use macroquad_toolkit::persistence::*; // save_json, load_json, get_app_data_path

#[derive(Serialize, Deserialize)]
struct SaveData {
    score: u32,
    level: u32,
}

// Save to standard app data location
if let Some(path) = get_app_data_path("my_game", "save.json") {
    let data = SaveData { score: 100, level: 1 };
    save_json(&path, &data).ok();
}
```

For a production key path that needs isolated native restart tests, use
`save_json_key_configured`, `load_json_key_configured`, and `json_key_exists_configured`. Their
optional environment-variable argument supplies the complete native test path; browser builds keep
using the normal qualified storage key.

### Backup generations (`persistence` module)

`BackupChain` rotates raw saves across a caller-selected number of generations.
Use `with_generations("campaign", 3)` for `campaign_backup`, `campaign_backup_2`,
and `campaign_backup_3`, or `new` with explicit historical names. `KeySaveStore`
adapts the existing native/browser qualified key API; implement `RawSaveStore`
for other stores or in-memory tests.

Call `save` with a validator that rejects unsupported versions as well as corrupt
data. Existing primary validation must succeed before replacement. Rotation
preserves the original bytes and version envelope, skips invalid backup copies,
and writes oldest-first and primary-last. A failed write can leave partially
rotated backups, but cannot destroy the previous primary if the backend honors
the single-key atomic-write contract. Serialize concurrent writers externally.

`recover` accepts a game-owned decoder/migration callback and returns the value,
`SaveSource`, and rejected candidates. Recovery never writes automatically.
Quarantine or otherwise explicitly resolve an invalid primary before saving over
it. Native read failures remain errors, distinct from absent files; the browser
adapter inherits the existing bridge's missing/read-unavailable behavior.

### Legacy browser keys (`persistence` module)

`load_json_key_with_legacy(game, key, &["old_raw_key"], validate)` reads the
normal qualified key first. Only when it is absent on WASM does it try the
explicit raw-key allowlist. Both primary and historical JSON must pass the
game's schema/version validator. A present corrupt/future primary blocks import;
it is never silently replaced by old progress. Imported bytes and historical
keys are preserved, and failed writes return an error. Native builds use the
ordinary key file and ignore browser aliases. `LegacySource` identifies imports.

`load_with_legacy_keys` provides the same policy over `RawSaveStore` for tests
or custom backends, with exact backend keys. No key discovery or deletion occurs.

### Audio (`audio` module)

```rust
use macroquad_toolkit::audio::{SoundManager, SoundId};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Sfx {
    Jump,
    Shoot,
}

let mut audio = SoundManager::new();
audio.load_sound(Sfx::Jump, "assets/jump.wav").await.ok();

audio.play_sfx(Sfx::Jump, 1.0); // Plays with 1.0 * global_sfx_volume
```

### States (`states` module)

```rust
use macroquad_toolkit::states::{GameState, Transition};
use std::any::Any;

struct MyGame;

struct MenuState;
impl GameState<MyGame> for MenuState {
    fn update(&mut self, _ctx: &mut MyGame) -> Option<Box<dyn Any>> {
        None
    }
    fn draw(&self, _ctx: &MyGame) {
        // Draw menu
    }
}
```

### Capture harness

`scripts/capture_ui.ps1` can run a game's complete deterministic scene manifest in one process.
Pass `-ProcessReportPath <path>` to write a compact JSON record after the process exits. The report
includes the executable path, scene/frame counts, requested surface, fullscreen mode, the largest
working set sampled every 25 ms, and Windows' lifetime peak working set. It also records the sample
count plus first, median, p95, and final sampled working sets so callers can distinguish a brief
peak from a sustained footprint. Relative report paths resolve from `-GameDir`; ordinary captures
write no process report unless this option is supplied.

Pass `-SampleWindowsGpuCounters` to additionally sample process-scoped dedicated/shared GPU memory
and aggregate 3D-engine utilization. The report records `sampled`, `unavailable`, or `not_requested`
explicitly and imposes no default GPU threshold; performance-counter availability depends on
Windows, the display driver, and whether the process creates a matching GPU instance.

For a sustained render/device probe, `-MinFrameMilliseconds` enforces a minimum wall-clock frame
duration without changing the game's deterministic capture timestep. The report records that value
and total elapsed wall time. `-ExecutablePath <path> -SkipBuild` runs a specific previously built or
packaged executable while retaining the same scene, image, timing, and memory checks.

## Button Click Semantics

The toolkit provides two button variants to handle different click behaviors:

- **`button()` and `button_on_release()`**: Fire when mouse button is **released** over the button. This is the safer default as it prevents accidental double-clicks and allows users to move the mouse away to cancel.

- **`button_on_press()`**: Fires when mouse button is **pressed down** over the button. Use this for instant feedback scenarios.

## Database (`db` module)

Utilities for SQLite using `sqlx`. Enabled via the `db` feature.

**Cargo.toml**:
```toml
macroquad-toolkit = { version = "...", features = ["db"] }
```

**Usage**:
```rust
use macroquad_toolkit::db::Database;

// Initialize (auto-creates DB file if missing)
let db = Database::new("sqlite://game.db").await?;

// Run raw migrations/queries
db.run_raw_migrations(&[
    "CREATE TABLE IF NOT EXISTS players (id TEXT PRIMARY KEY)",
    "INSERT INTO players (id) VALUES ('p1')",
]).await?;

// Access the underlying sqlx::SqlitePool
let row: (i64,) = sqlx::query_as("SELECT count(*) FROM players")
    .fetch_one(&db.pool)
    .await?;
```

## License


This toolkit is extracted from game projects and shared for reuse across multiple games.
# Practical Future Improvements

- Add public API docs and compatibility tests for UI primitives, layout helpers, font loading, surface drawing, and widgets.
- Provide shared save/load migration helpers and asset-loading diagnostics that games can opt into instead of duplicating recovery code.
- Add micro-benchmarks for text layout, panel drawing, cached fonts, and widget hit-testing under common WebGL screen sizes.
- Create example screens that demonstrate toolkit patterns for menus, overlays, panels, notifications, and responsive scaling.

