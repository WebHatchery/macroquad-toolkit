//! Bundled reference artwork and its texture-manifest entry point.

use crate::assets::AssetManager;

/// Manifest path for the toolkit-owned reference art pack.
pub const ARTWORK_MANIFEST_PATH: &str = "assets/artwork_manifest.json";

/// Keys present in [`ARTWORK_MANIFEST_PATH`], in manifest order.
pub const ARTWORK_KEYS: &[&str] = &[
    "portrait_archivist_neutral",
    "drone_atlas",
    "drone_billboard",
    "semantic_icons",
    "missing_texture",
];

/// Individual 48px semantic glyphs shipped beside the 4x4 reference sheet.
pub const ICON_KEYS: &[&str] = &[
    "close",
    "back",
    "forward",
    "home",
    "menu",
    "expand",
    "collapse",
    "play",
    "pause",
    "resume",
    "stop",
    "restart",
    "save",
    "load",
    "autosave",
    "sync",
    "delete",
    "settings",
    "music",
    "sound",
    "mute",
    "fullscreen",
    "help",
    "info",
    "success",
    "warning",
    "danger",
    "search",
    "filter",
    "sort_ascending",
    "sort_descending",
    "add",
    "remove",
    "edit",
    "inventory",
    "equipment",
    "map",
    "objectives",
    "journal",
    "achievements",
    "lock",
    "unlock",
    "visible",
    "hidden",
    "favorite",
    "notification",
    "tap",
    "drag",
    "swipe",
    "pinch",
];

/// Load all bundled reference artwork into an [`AssetManager`].
pub async fn load_toolkit_artwork(manager: &mut AssetManager) -> Result<usize, String> {
    manager
        .load_texture_manifest_file(ARTWORK_MANIFEST_PATH)
        .await
}

/// Load each individual semantic icon as a named texture.
pub async fn load_toolkit_icons(manager: &mut AssetManager) -> usize {
    let mut loaded = 0;
    for key in ICON_KEYS {
        let path = format!("assets/images/ui/icons/icon_{key}.png");
        if manager.load_texture(key, &path).await.is_ok() {
            loaded += 1;
        }
    }
    loaded
}

#[cfg(test)]
mod tests;
