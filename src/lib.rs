//! Macroquad Toolkit
//!
//! A comprehensive collection of utilities for Macroquad game development.
//! Extracted from multiple games to reduce duplication and provide consistent patterns.
//!
//! # Module Organization
//!
//! ## 2D Game Development
//! - [`camera`] - 2D camera with pan and zoom
//! - [`sprite`] - Sprite rendering utilities
//! - [`ui`] - UI components (buttons, panels, progress bars)
//! - [`input`] - Input handling helpers
//! - [`colors`] - Color palettes (dark theme, rarity colors)
//!
//! ## 3D Game Development
//! - [`render3d`] - 3D rendering utilities
//!   - [`render3d::billboard`] - Billboard sprites for 3D
//!   - [`render3d::camera`] - Isometric and orbit cameras
//!
//! ## Grid & Pathfinding
//! - [`grid`] - Grid data structures, coordinate conversion, fog of war
//! - [`pathfinding`] - A* pathfinding with caching
//!
//! ## Game Systems
//! - [`entities`] - Generic entity management system
//! - [`notifications`] - Toast notification system
//! - [`states`] - Game state machine helpers
//! - [`rng`] - Random number utilities
//! - [`math`] - Interpolation, easing, pulse, and tween helpers
//! - [`timing`] - Cooldowns, timers, interval tickers, phase timelines
//! - [`fx`] - Screen shake, screen fades, particles, floating text
//! - [`settings`] - Shared user-settings model with persistence
//! - [`achievements`] - Achievement registry with unlock tracking
//! - [`debug`] - FPS / frame-time debug overlay
//!
//! ## Data & Persistence
//! - [`persistence`] - Save/load system (native + WASM)
//! - [`data_loader`] - JSON data loading patterns
//! - [`assets`] - Asset management and texture loading
//! - [`raster`] - CPU-side pixel drawing onto images for procedural art
//!
//! ## Other
//! - [`audio`] - Audio playback utilities
//! - [`events`] - Event handling utilities
//! - [`capture`] - Headless screenshot capture harness (env-var driven)
//! - [`db`] - Database support (optional, requires `db` feature)

// Core 2D modules (existing)
pub mod achievements;
pub mod assets;
pub mod audio;
pub mod camera;
pub mod capture;
pub mod colors;
pub mod debug;
pub mod events;
pub mod fx;
pub mod input;
pub mod math;
pub mod paint;
pub mod raster;
pub mod rng;
pub mod settings;
pub mod sprite;
pub mod states;
pub mod strip;
pub mod synth;
pub mod timing;
pub mod ui;

// New modules extracted from dungeon_manager
pub mod data_loader;
pub mod entities;
pub mod grid;
pub mod notifications;
pub mod pathfinding;
pub mod persistence;
pub mod render3d;

// Optional database support
#[cfg(feature = "db")]
pub mod db;

// WASM storage support
#[cfg(target_arch = "wasm32")]
pub mod wasm_storage;

/// Convenient re-exports for common 2D game development usage
pub mod prelude {
    // Input and UI
    pub use crate::colors::{
        dark, darken, lerp_color, lighten, mix, multiply_alpha, scale_rgb, shade, tint, with_alpha,
    };
    pub use crate::input::*;
    pub use crate::ui::*;

    // Effects and timing
    pub use crate::fx::{
        is_fully_typed, prefix_chars, reveal_block, typed_char_count, typed_prefix, BlockReveal,
        CrtOverlay, CrtStyle, FloatingTextLayer, ParticleSystem, ProjectileLayer, ScreenFade,
        ScreenShake,
    };
    pub use crate::math::{
        approach, blink, clamp01, inv_lerp, lerp, pulse01, pulse_range, smoothstep,
    };
    pub use crate::timing::{Cooldown, IntervalTimer, Timeline, Timer};

    // Assets and rendering
    pub use crate::assets::{AssetManager, AssetPack};
    pub use crate::camera::{Camera2D, Camera2DConfig, CameraBounds};
    pub use crate::sprite::Sprite;

    // Persistence
    pub use crate::persistence::*;

    // Game systems
    pub use crate::notifications::{NotificationManager, NotificationType};
    pub use crate::rng::*;
    pub use crate::states::*;

    // Screenshot capture harness
    pub use crate::capture::{capture_requested, capture_window_conf, run_capture, CaptureConfig};
}

/// Re-exports for 3D game development
pub mod prelude_3d {
    pub use crate::entities::*;
    pub use crate::grid::*;
    pub use crate::pathfinding::*;
    pub use crate::render3d::*;
}

/// Re-exports for data-driven game development
pub mod prelude_data {
    pub use crate::data_loader::*;
    pub use crate::persistence::*;
}
