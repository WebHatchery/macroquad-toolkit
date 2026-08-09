//! Background screenshot capture harness.
//!
//! Lets a game screenshot *itself*: when a `PREFIX_CAPTURE_MANIFEST` env var is
//! set, the game boots into the requested scenes, steps each simulation a
//! fixed number of frames at a fixed timestep, and writes PNGs. This makes
//! UI/rendering changes visually verifiable from a script (or by an AI agent
//! that reads the PNG back) with no interactive input.
//!
//! Env vars (replace `PREFIX` with your game's prefix, e.g. `CARRIAGE`):
//! - `PREFIX_CAPTURE_MANIFEST` — tab-separated scene/path rows for a batch
//! - `PREFIX_CAPTURE_FRAMES` — frames to simulate before capturing (default 150)
//! - `PREFIX_WINDOW_WIDTH` / `PREFIX_WINDOW_HEIGHT` — window size override
//! - `PREFIX_HEADLESS` — hide the game window; on by default while capturing,
//!   set to `0` to watch the run (see [`headless`])
//!
//! Integration (see `docs/screenshot_capture_harness_guide.md` for the full
//! walkthrough and gotchas):
//!
//! ```ignore
//! fn window_conf() -> Conf {
//!     capture::capture_window_conf("MYGAME", "My Game", 1280, 720)
//! }
//!
//! #[macroquad::main(window_conf)]
//! async fn main() {
//!     let mut game = Game::new().await;
//!
//!     if let Some(configs) = capture::CaptureConfig::all_from_env("MYGAME") {
//!         for config in configs {
//!             game.begin_capture_scene(&config.scene);
//!             capture::run_capture_once(&config, |dt| {
//!                 game.update(dt);
//!                 game.draw();
//!             })
//!             .await;
//!         }
//!         return;
//!     }
//!
//!     loop { /* normal interactive loop */ }
//! }
//! ```
//!
//! All env access is stubbed out on `wasm32`, so web builds are unaffected.

pub mod filmstrip;
pub mod headless;

use macroquad::prelude::*;

/// Capture parameters read from `PREFIX_CAPTURE_*` env vars.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// The game's env-var prefix, kept so the harness can read the rest of the
    /// `PREFIX_*` family (e.g. `PREFIX_HEADLESS`) without being told twice.
    pub prefix: String,
    /// Output PNG path from the manifest row.
    pub path: String,
    /// Scene name to seed before capturing, from the manifest row.
    pub scene: String,
    /// Number of frames to simulate before writing the PNG (`PREFIX_CAPTURE_FRAMES`, default 150).
    pub frames: u32,
    /// Fixed timestep per simulated frame. Fixed (not `get_frame_time()`) so
    /// repeated runs are deterministic. Default 1/60.
    pub timestep: f32,
}

impl CaptureConfig {
    /// Returns every requested capture from the batch manifest. One manifest is
    /// consumed by one process/window. Always `None` on wasm32.
    pub fn all_from_env(prefix: &str) -> Option<Vec<Self>> {
        if let Some(manifest_path) = env_string(&format!("{prefix}_CAPTURE_MANIFEST")) {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let contents = std::fs::read_to_string(&manifest_path).unwrap_or_else(|error| {
                    panic!("could not read capture manifest {manifest_path}: {error}")
                });
                let frames = env_u32(&format!("{prefix}_CAPTURE_FRAMES"), 150).max(1);
                let configs = contents
                    .trim_start_matches('\u{feff}')
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| {
                        let (scene, path) = line.split_once('\t').unwrap_or_else(|| {
                            panic!("invalid capture manifest row (expected scene<TAB>path): {line}")
                        });
                        Self {
                            prefix: prefix.to_owned(),
                            path: path.to_owned(),
                            scene: scene.to_owned(),
                            frames,
                            timestep: 1.0 / 60.0,
                        }
                    })
                    .collect::<Vec<_>>();
                if configs.is_empty() {
                    panic!("capture manifest {manifest_path} contains no scenes");
                }
                return Some(configs);
            }

            #[cfg(target_arch = "wasm32")]
            {
                let _ = manifest_path;
                return None;
            }
        }

        None
    }
}

/// True when the process was launched with a batch capture manifest.
pub fn capture_requested(prefix: &str) -> bool {
    env_string(&format!("{prefix}_CAPTURE_MANIFEST")).is_some()
}

/// Capture-aware `Conf` for `#[macroquad::main(window_conf)]`.
///
/// Reads `PREFIX_WINDOW_WIDTH/HEIGHT` overrides and disables `high_dpi` while
/// capturing so the screenshot framebuffer is pixel-aligned with the logical
/// UI layout (on scaled displays `high_dpi: true` captures at 2x size).
///
/// Also arms [`headless`] window hiding. `window_conf()` is the earliest hook a
/// game has — arming here means the window is hidden as it appears rather than
/// after the game has finished loading.
pub fn capture_window_conf(
    prefix: &str,
    title: &str,
    default_width: i32,
    default_height: i32,
) -> Conf {
    headless::arm(prefix);
    Conf {
        window_title: title.to_owned(),
        window_width: env_i32(&format!("{prefix}_WINDOW_WIDTH"), default_width),
        window_height: env_i32(&format!("{prefix}_WINDOW_HEIGHT"), default_height),
        window_resizable: true,
        high_dpi: !capture_requested(prefix),
        ..Default::default()
    }
}

/// Screenshot harness loop for one scene: call `frame(timestep)` (your update +
/// draw) a fixed number of times and write its PNG without exiting the process.
///
/// Seed your scene (e.g. `game.begin_capture_scene(&config.scene)`) before
/// calling this.
pub async fn run_capture_once<F: FnMut(f32)>(config: &CaptureConfig, mut frame: F) {
    // No-op when `window_conf` already armed it; the safety net for a game that
    // builds its `Conf` by hand and never called `capture_window_conf`.
    headless::arm(&config.prefix);

    let mut rendered = 0;
    loop {
        frame(config.timestep);
        rendered += 1;
        // Read the framebuffer after drawing this frame but before presenting
        // it; reading after `next_frame` would return the swapped/cleared
        // buffer (a solid-black PNG).
        if rendered >= config.frames {
            get_screen_data().export_png(&config.path);
            break;
        }
        next_frame().await;
    }

    println!(
        "captured {} (scene: {}, {} frames)",
        config.path, config.scene, config.frames
    );
}

/// Read an env var as an `i32`, falling back on missing/unparsable values.
pub fn env_i32(name: &str, fallback: i32) -> i32 {
    env_string(name)
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(fallback)
}

/// Read an env var as a `u32`, falling back on missing/unparsable values.
pub fn env_u32(name: &str, fallback: u32) -> u32 {
    env_string(name)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(fallback)
}

/// Read an env var as a bool: unset uses the fallback; `0`/`false` are false,
/// anything else is true.
pub fn env_bool(name: &str, fallback: bool) -> bool {
    env_string(name)
        .map(|value| value != "0" && !value.eq_ignore_ascii_case("false"))
        .unwrap_or(fallback)
}

/// Read an env var as a `f32`, falling back on missing/unparsable values.
pub fn env_f32(name: &str, fallback: f32) -> f32 {
    env_string(name)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(fallback)
}

/// Read an env var. Always `None` on wasm32 (no env access in the browser).
pub fn env_string(name: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(name).ok()
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        None
    }
}
