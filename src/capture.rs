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
//! - `PREFIX_CAPTURE_MIN_FRAME_MS` — optional minimum wall-clock duration per
//!   rendered frame; useful for sustained device soaks, zero/unset by default
//! - `PREFIX_WINDOW_WIDTH` / `PREFIX_WINDOW_HEIGHT` — window size override
//! - `PREFIX_CAPTURE_FULLSCREEN` — request a borderless fullscreen framebuffer
//!   (useful when store media needs the monitor's exact pixel dimensions)
//! - `PREFIX_HEADLESS` — hide the game window; on by default while capturing,
//!   set to `0` to watch the run (see [`headless`])
//!
//! Integration (see the repository's `MACROQUAD_TOOLKIT.md`, Screenshot capture
//! section, for the wrapper commands and supported options):
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
//!             capture::prepare_capture_surface(&config.prefix)
//!                 .await
//!                 .expect("capture surface must match the requested size");
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

#[cfg(target_os = "windows")]
static CAPTURE_SURFACE_REPORTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

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
    /// Optional minimum wall-clock duration per rendered frame. This does not
    /// change the deterministic simulation timestep.
    pub minimum_frame_millis: f32,
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
                let minimum_frame_millis =
                    env_f32(&format!("{prefix}_CAPTURE_MIN_FRAME_MS"), 0.0).max(0.0);
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
                            minimum_frame_millis,
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
/// Reads `PREFIX_WINDOW_WIDTH/HEIGHT` and `PREFIX_CAPTURE_FULLSCREEN`
/// overrides and disables `high_dpi` while capturing so the screenshot
/// framebuffer is pixel-aligned with the logical UI layout (on scaled displays
/// `high_dpi: true` captures at 2x size).
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
        fullscreen: capture_requested(prefix)
            && env_bool(&format!("{prefix}_CAPTURE_FULLSCREEN"), false),
        ..Default::default()
    }
}

/// Resolve requested capture client dimensions, using the live framebuffer size
/// when either window override is absent or invalid. Non-positive results fail.
pub fn capture_surface_size(prefix: &str, fallback: (i32, i32)) -> Result<(i32, i32), String> {
    let width = env_i32(&format!("{prefix}_WINDOW_WIDTH"), fallback.0);
    let height = env_i32(&format!("{prefix}_WINDOW_HEIGHT"), fallback.1);
    if width <= 0 || height <= 0 {
        return Err(format!(
            "invalid capture client size {width}x{height}; width and height must be positive"
        ));
    }
    Ok((width, height))
}

/// Prepare the native capture client at the requested window dimensions.
///
/// Headless mode hides the process-owned window; resizing uses non-activating Win32
/// positioning and waits at most three rendered frames for Macroquad to report
/// the exact client size. Browser builds do not have a native window and return
/// successfully without doing anything.
pub async fn prepare_capture_surface(prefix: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = prefix;
        return Ok(());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Fullscreen capture uses the display framebuffer, not windowed overrides.
        if env_bool(&format!("{prefix}_CAPTURE_FULLSCREEN"), false) {
            return Ok(());
        }
        let fallback = (
            screen_width().round() as i32,
            screen_height().round() as i32,
        );
        let (width, height) = capture_surface_size(prefix, fallback)?;

        #[cfg(target_os = "windows")]
        let headless_mode = headless::headless_requested(prefix);
        #[cfg(target_os = "windows")]
        let resize_diagnostic = {
            if headless_mode {
                headless::hide_window();
            }
            headless::resize_window_client(width, height, headless_mode)?
        };

        #[cfg(not(target_os = "windows"))]
        if (
            screen_width().round() as i32,
            screen_height().round() as i32,
        ) != (width, height)
        {
            return Err(format!(
                "native client resize to {width}x{height} is unsupported on this platform"
            ));
        }

        for _ in 0..3 {
            if (
                screen_width().round() as i32,
                screen_height().round() as i32,
            ) == (width, height)
            {
                #[cfg(target_os = "windows")]
                if headless_mode
                    && !CAPTURE_SURFACE_REPORTED.swap(true, std::sync::atomic::Ordering::Relaxed)
                {
                    let native = headless::window_size_report().unwrap_or_else(|error| error);
                    println!(
                        "capture client verified: Macroquad {}x{}; Win32 {native}; resize {resize_diagnostic}",
                        width, height
                    );
                }
                return Ok(());
            }
            next_frame().await;
        }

        let actual = (
            screen_width().round() as i32,
            screen_height().round() as i32,
        );
        if actual == (width, height) {
            Ok(())
        } else {
            #[cfg(target_os = "windows")]
            let diagnostic = format!(
                "; Win32 after 3 frames [{}]; resize [{resize_diagnostic}]",
                headless::window_size_report().unwrap_or_else(|error| error)
            );
            #[cfg(not(target_os = "windows"))]
            let diagnostic = String::new();
            Err(format!(
                "capture client resize did not settle: requested {width}x{height}, got {}x{} after 3 frames{diagnostic}",
                actual.0, actual.1
            ))
        }
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
    prepare_capture_surface(&config.prefix)
        .await
        .unwrap_or_else(|error| panic!("cannot prepare capture surface: {error}"));

    // Games commonly load a persisted windowed preference after `Conf` has
    // created the window. Reassert capture fullscreen after game startup and
    // give the platform one event-loop turn to resize the framebuffer.
    if env_bool(&format!("{}_CAPTURE_FULLSCREEN", config.prefix), false) {
        set_fullscreen(true);
        next_frame().await;
    }

    let mut rendered = 0;
    loop {
        #[cfg(not(target_arch = "wasm32"))]
        let frame_started = std::time::Instant::now();
        frame(config.timestep);
        rendered += 1;
        #[cfg(not(target_arch = "wasm32"))]
        if config.minimum_frame_millis > 0.0 {
            let minimum = std::time::Duration::from_secs_f32(config.minimum_frame_millis / 1_000.0);
            if let Some(remaining) = minimum.checked_sub(frame_started.elapsed()) {
                std::thread::sleep(remaining);
            }
        }
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
    // Finish the photographed frame before the next scene starts drawing.
    // Without this boundary, Macroquad's text batch can retain the previous
    // scene's texture state and later glyphs render as black silhouettes.
    next_frame().await;
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
