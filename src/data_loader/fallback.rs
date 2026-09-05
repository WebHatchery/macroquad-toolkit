//! Explicit fallback policies for games with runtime content overrides.

use super::parse_json_labeled;
use serde::de::DeserializeOwned;

/// Which runtime failures permit using the caller's fallback JSON.
/// Required files should use `load_json_file` or `load_json_file_sync` instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonFallbackPolicy {
    /// Fall back on missing or unreadable files, but reject malformed runtime JSON.
    ReadError,
    /// Fall back on missing, unreadable, or malformed runtime JSON.
    ReadOrParseError,
}

/// Load a native runtime override, using fallback JSON according to `policy`.
/// On WASM this synchronous API uses only the embedded/default copy; use the
/// async counterpart when browser runtime overrides should be fetched.
/// Unlike the existing candidate-path loader, this can fall back on read errors
/// for an existing file. An invalid fallback always returns a labeled error.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_json_file_with_fallback_sync<T: DeserializeOwned>(
    path: impl AsRef<std::path::Path>,
    fallback_json: &str,
    policy: JsonFallbackPolicy,
) -> Result<T, String> {
    let path = path.as_ref();
    resolve(
        &path.display().to_string(),
        std::fs::read_to_string(path).map_err(|error| error.to_string()),
        fallback_json,
        policy,
    )
}

/// WASM synchronous loading uses the embedded/default copy without network I/O.
#[cfg(target_arch = "wasm32")]
pub fn load_json_file_with_fallback_sync<T: DeserializeOwned>(
    path: impl AsRef<std::path::Path>,
    fallback_json: &str,
    _policy: JsonFallbackPolicy,
) -> Result<T, String> {
    parse_json_labeled(
        &format!("embedded {}", path.as_ref().display()),
        fallback_json,
    )
}

/// Load a runtime override on native or WASM with an explicit fallback policy.
/// WASM attempts the browser asset path before using the embedded/default copy.
pub async fn load_json_file_with_fallback<T: DeserializeOwned>(
    path: &str,
    fallback_json: &str,
    policy: JsonFallbackPolicy,
) -> Result<T, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let source = std::fs::read_to_string(path).map_err(|error| error.to_string());
    #[cfg(target_arch = "wasm32")]
    let source = macroquad::file::load_string(path)
        .await
        .map_err(|error| format!("{error:?}"));
    resolve(path, source, fallback_json, policy)
}

fn resolve<T: DeserializeOwned>(
    path: &str,
    source: Result<String, String>,
    fallback_json: &str,
    policy: JsonFallbackPolicy,
) -> Result<T, String> {
    let failure = match source {
        Ok(json) => match parse_json_labeled(path, &json) {
            Ok(value) => return Ok(value),
            Err(error) if policy == JsonFallbackPolicy::ReadError => return Err(error),
            Err(error) => error,
        },
        Err(error) => format!("JSON data file read error in '{path}': {error}"),
    };
    parse_json_labeled(&format!("fallback for {path}"), fallback_json)
        .map_err(|error| format!("{failure}; {error}"))
}

#[cfg(test)]
mod tests;
